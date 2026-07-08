use fluidic::api::state::{ApiState, RecentShift};
use fluidic::api::start_api_server;
use fluidic::consensus::Oscillator;
use fluidic::crypto::keys::KeyPair;
use fluidic::crypto::{CommutativeShift, Signal, StakeShift, DEFAULT_DEX_DOMAIN};
use fluidic::field::coordinates::Coordinate;
use fluidic::light_client::LightClient;
use fluidic::network::{
    fetch_bootstrap_url, mdns_announce, mdns_browse, resolve_txt_seeds, DhtDiscovery,
    EndpointScheme, PeerAnnouncement, TcpGossipNode, WsGossipNode, DEFAULT_NETWORK_ID,
};
use fluidic::persistence;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{info, trace, warn};

fn peer_cache_path() -> std::path::PathBuf {
    let dir = std::env::var("FLUIDIC_DATA_DIR").unwrap_or_else(|_| "./data".to_string());
    std::path::PathBuf::from(dir).join("peers.json")
}

fn load_peer_cache() -> Vec<String> {
    let path = peer_cache_path();
    if !path.exists() {
        return Vec::new();
    }
    match std::fs::read_to_string(&path) {
        Ok(json) => {
            // Support both old flat endpoint list and new directory JSON.
            if json.trim().starts_with('[') {
                serde_json::from_str::<Vec<String>>(&json).unwrap_or_default()
            } else {
                Vec::new()
            }
        }
        Err(e) => {
            warn!("failed to read peer cache {}: {}", path.display(), e);
            Vec::new()
        }
    }
}

fn save_peer_cache(peers: &[String]) {
    let path = peer_cache_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(peers) {
        let _ = std::fs::write(&path, json);
    }
}

/// Resolve a legacy DNS seed (host or host:port) into socket addresses with a
/// bounded timeout so a missing/stale seed does not block node startup.
pub async fn resolve_dns_seed(seed: &str) -> Vec<std::net::SocketAddr> {
    let host = if seed.contains(':') {
        seed.to_string()
    } else {
        format!("{}:7000", seed)
    };
    match tokio::time::timeout(std::time::Duration::from_secs(5), tokio::net::lookup_host(&host))
        .await
    {
        Ok(Ok(addrs)) => addrs.collect(),
        Ok(Err(e)) => {
            warn!("failed to resolve DNS seed {}: {}", seed, e);
            Vec::new()
        }
        Err(_) => {
            warn!("DNS seed resolution for {} timed out", seed);
            Vec::new()
        }
    }
}

/// Normalize a signed bootstrap endpoint into a string the connection loop can
/// dial.  TCP endpoints are reduced to `host:port`; WebSocket endpoints are
/// kept as a full URL.
fn add_bootstrap_peer(peers: &mut Vec<String>, endpoint: &str) {
    match EndpointScheme::parse(endpoint) {
        Some((EndpointScheme::Tcp, rest)) => peers.push(rest.to_string()),
        Some((EndpointScheme::Ws | EndpointScheme::Wss, _)) => peers.push(endpoint.to_string()),
        None if endpoint.contains(':') => peers.push(endpoint.to_string()),
        None => warn!("ignoring malformed bootstrap endpoint: {}", endpoint),
    }
}

/// Return the local IP to advertise over mDNS.  Unspecified and loopback
/// addresses are skipped because they don't help LAN peers reach us.
fn local_announce_ip(bind_addr: SocketAddr) -> Option<std::net::IpAddr> {
    let ip = bind_addr.ip();
    if ip.is_unspecified() || ip.is_loopback() {
        None
    } else {
        Some(ip)
    }
}

#[tokio::main]
async fn main() {
    eprintln!("mesh_node: starting up");
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("PANIC: {:?}\n", info);
        eprintln!("{}", msg);
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/data/panic.log")
            .and_then(|mut f| std::io::Write::write_all(&mut f, msg.as_bytes()));
    }));
    tracing_subscriber::fmt::init();

    let data_dir = std::env::var("FLUIDIC_DATA_DIR").unwrap_or_else(|_| "./data".to_string());
    let id_str = std::env::var("OSCILLATOR_ID").unwrap_or_else(|_| "0".to_string());
    let id = {
        let mut arr = [0u8; 32];
        // Support both plain numbers ("0") and StatefulSet pod names ("mesh-node-0").
        // If the variable is not a valid number, fall back to a persisted random id
        // so multiple users running the default instructions do not collide.
        let n: u64 = id_str
            .rsplit_once('-')
            .and_then(|(_, suffix)| suffix.parse().ok())
            .or_else(|| id_str.parse().ok())
            .unwrap_or_else(|| {
                let id_file = std::path::Path::new(&data_dir).join("operator_id");
                if let Ok(s) = std::fs::read_to_string(&id_file) {
                    if let Ok(n) = s.trim().parse::<u64>() {
                        return n;
                    }
                }
                let n: u64 = rand::random();
                let _ = std::fs::create_dir_all(&data_dir)
                    .and_then(|_| std::fs::write(&id_file, n.to_string()));
                info!("generated random OSCILLATOR_ID {}; persisted to {:?}", n, id_file);
                n
            });
        arr[0..8].copy_from_slice(&n.to_le_bytes());
        arr
    };

    let bind_addr: SocketAddr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:7000".to_string())
        .parse()
        .expect("BIND_ADDR must be a valid SocketAddr");

    // Build initial peer list from explicit PEERS and cached peers immediately.
    // DNS seed, signed DNS TXT, HTTPS bootstrap, DHT, and mDNS lookups all run
    // concurrently with API startup so the health endpoint is available before
    // any potentially slow network resolution finishes.
    let mut peers: Vec<String> = std::env::var("PEERS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let cached_peers = load_peer_cache();
    peers.extend(cached_peers);

    let seed_dns_task = if let Ok(seed) = std::env::var("SEED_DNS") {
        let seed = seed.trim().to_string();
        if !seed.is_empty() {
            Some(tokio::spawn(async move {
                info!("resolving legacy DNS seed {}", seed);
                resolve_dns_seed(&seed)
                    .await
                    .into_iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<_>>()
            }))
        } else {
            None
        }
    } else {
        None
    };

    let bootstrap_dns_task = if let Ok(dns_seed) = std::env::var("BOOTSTRAP_DNS") {
        let dns_seed = dns_seed.trim().to_string();
        if !dns_seed.is_empty() {
            Some(tokio::spawn(async move {
                info!("resolving signed bootstrap DNS TXT records for {}", dns_seed);
                resolve_txt_seeds(&dns_seed).await
            }))
        } else {
            None
        }
    } else {
        None
    };

    let bootstrap_url_task = if let Ok(url) = std::env::var("BOOTSTRAP_URL") {
        let url = url.trim().to_string();
        if !url.is_empty() {
            Some(tokio::spawn(async move {
                info!("fetching signed bootstrap records from {}", url);
                fetch_bootstrap_url(&url).await
            }))
        } else {
            None
        }
    } else {
        None
    };

    // Query public Mainline DHT and local mDNS for additional peers.  These are
    // completely independent of DNS/HTTPS bootstraps, so the mesh can survive
    // the loss of all Fluidic-operated infrastructure.  Run them concurrently
    // with API startup so the health endpoint is available as soon as possible.
    let network_id = std::env::var("FLUIDIC_NETWORK_ID")
        .unwrap_or_else(|_| DEFAULT_NETWORK_ID.to_string());
    let enable_dht = std::env::var("DHT_BOOTSTRAP")
        .unwrap_or_else(|_| "true".to_string())
        .eq_ignore_ascii_case("true");
    let enable_mdns = std::env::var("MDNS_BOOTSTRAP")
        .unwrap_or_else(|_| "true".to_string())
        .eq_ignore_ascii_case("true");

    let public_endpoint = std::env::var("PUBLIC_ENDPOINT").unwrap_or_else(|_| {
        // Default to the bind address if it looks like a public IP; otherwise
        // run as a leaf node.  Leaf nodes still bind a local TCP gossip socket
        // so they can dial TCP peers discovered via DHT; inbound connections
        // simply won't reach them behind NAT, which is fine for normal users.
        if bind_addr.ip().is_unspecified() || bind_addr.ip().is_loopback() {
            String::new()
        } else {
            format!("tcp://{}", bind_addr)
        }
    });

    // Leaf nodes (no public inbound address) default to client/light mode.  This
    // matches whitepaper section 8.2: clients submit Signals and can run light
    // nodes, while operators explicitly advertise PUBLIC_ENDPOINT.  Set
    // FLUIDIC_CLIENT_MODE=false to force full-node behavior behind NAT.
    let client_mode = match std::env::var("FLUIDIC_CLIENT_MODE").as_deref() {
        Ok("true") => true,
        Ok("false") => false,
        _ => public_endpoint.is_empty(),
    };
    if client_mode {
        info!("running in client mode: will verify operator certificates instead of synthesizing");
    }

    let dht_discovery = if enable_dht {
        match DhtDiscovery::new(&network_id) {
            Ok(d) => Some(d),
            Err(e) => {
                warn!("failed to start DHT discovery: {}", e);
                None
            }
        }
    } else {
        None
    };

    let dht_lookup_task = {
        let dht_discovery = dht_discovery.clone();
        tokio::spawn(async move {
            let mut out = Vec::new();
            if let Some(ref d) = dht_discovery {
                let addrs = d.lookup_peers().await;
                info!("dht lookup returned {} peer(s)", addrs.len());
                out = addrs.into_iter().map(|a| a.to_string()).collect();
            }
            out
        })
    };

    let mdns_lookup_task = if enable_mdns {
        Some(tokio::task::spawn_blocking(move || {
            match mdns_browse(std::time::Duration::from_secs(3)) {
                Ok(endpoints) => {
                    info!("mdns lookup returned {} peer(s)", endpoints.len());
                    endpoints
                }
                Err(e) => {
                    warn!("mdns browse failed: {}", e);
                    Vec::new()
                }
            }
        }))
    } else {
        None
    };

    let synth_interval_ms: u64 = std::env::var("SYNTHESIS_INTERVAL_MS")
        .unwrap_or_else(|_| "1000".to_string())
        .parse()
        .expect("SYNTHESIS_INTERVAL_MS must be a number");

    let snapshot_interval_ms: u64 = std::env::var("SNAPSHOT_INTERVAL_MS")
        .unwrap_or_else(|_| "5000".to_string())
        .parse()
        .expect("SNAPSHOT_INTERVAL_MS must be a number");

    let generator_interval_ms: u64 = std::env::var("GENERATOR_INTERVAL_MS")
        .unwrap_or_else(|_| "1000".to_string())
        .parse()
        .expect("GENERATOR_INTERVAL_MS must be a number");


    let discovery_mode = match EndpointScheme::parse(&public_endpoint) {
        Some((EndpointScheme::Tcp, _)) => DiscoveryMode::Tcp,
        Some((EndpointScheme::Ws | EndpointScheme::Wss, _)) => DiscoveryMode::WebSocket,
        // Default leaf nodes to TCP so DHT-discovered TCP seeds are dialed
        // immediately without requiring a hardcoded WebSocket bootstrap URL.
        None => DiscoveryMode::Tcp,
    };

    // Derive a deterministic local keypair from the oscillator id so the node
    // keeps the same identity across restarts.
    let local_keypair = KeyPair::from_seed(&id);

    let mut oscillator = Oscillator::new(id, 2048);
    oscillator.set_operator_keypair(local_keypair.clone());

    // Load persisted state if available, then seed only fresh accounts.
    let snapshot_path = persistence::snapshot_path();
    if let Err(e) = persistence::load(&mut oscillator, &snapshot_path) {
        warn!("failed to load snapshot: {}", e);
    } else {
        info!("loaded snapshot from {:?}", snapshot_path);
    }

    let oscillator = Arc::new(oscillator);

    let local_account = local_keypair.account_id();
    let genesis_balance = 1_000_000_000_000_000_000u128;
    let mut api_state = Arc::new(ApiState::new(oscillator.clone()));
    Arc::get_mut(&mut api_state)
        .unwrap()
        .load_peer_directory(&peer_cache_path());

    if client_mode {
        // Client nodes follow the operator mesh; they do not stake or synthesize.
        api_state.set_operator_keypair(local_keypair.clone());
        api_state.register_key(local_keypair.account_id(), local_keypair.public_key());
    } else {
        // Seed genesis balance for the local operator on first boot and lock it as
        // stake so a fresh node is immediately eligible to synthesize certificates.
        if oscillator
            .wave_field
            .lock()
            .unwrap()
            .account_balance(local_account)
            .units
            == 0
        {
            oscillator.seed_account(local_account, genesis_balance);
        }
        let genesis_stake = StakeShift::sign(
            &local_keypair,
            genesis_balance,
            0,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0),
        );
        if !oscillator.apply_stake(&genesis_stake) {
            warn!("failed to lock genesis stake for {}", local_account);
        }
        api_state.set_operator_keypair(local_keypair.clone());
        api_state.register_key(local_keypair.account_id(), local_keypair.public_key());
    }

    let light_client = if client_mode {
        Some(LightClient::new(oscillator.stake_table.clone()))
    } else {
        None
    };

    // Railway (and some other hosts) provide the public HTTP port via the
    // standard PORT variable. Use API_PORT if set explicitly, otherwise fall
    // back to PORT, then 8080 for local runs.
    let api_port: u16 = std::env::var("API_PORT")
        .or_else(|_| std::env::var("PORT"))
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("API_PORT/PORT must be a number");
    info!(
        "starting API server on port {} (API_PORT={:?}, PORT={:?})",
        api_port,
        std::env::var("API_PORT").ok(),
        std::env::var("PORT").ok()
    );
    // Run the API server on a dedicated OS thread with its own current-thread
    // Tokio runtime so HTTP handling is isolated from consensus work.
    let api_state_for_server = api_state.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build API runtime");
        rt.block_on(async move {
            if let Err(e) = start_api_server(api_state_for_server, api_port).await {
                tracing::error!("API server failed: {}", e);
            }
        });
    });

    info!(
        "starting mesh node id={} bind={} public_endpoint={} discovery={:?} peers={:?}",
        id_str, bind_addr, public_endpoint, discovery_mode, peers
    );

    let psk: Option<[u8; 32]> = std::env::var("FLUIDIC_PSK")
        .ok()
        .and_then(|s| {
            let bytes = hex::decode(s.trim()).ok()?;
            if bytes.len() == 32 {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                Some(arr)
            } else {
                None
            }
        });
    if psk.is_some() {
        info!("gossip authentication enabled via FLUIDIC_PSK");
    }

    // Build the local peer announcement signed by the operator keypair.
    let local_announcement = if public_endpoint.is_empty() {
        None
    } else {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        Some(PeerAnnouncement::sign(
            &local_keypair,
            &public_endpoint,
            ts,
            3,
        ))
    };
    if let Some(ref ann) = local_announcement {
        api_state.peer_directory.insert_announcements(&[ann.clone()],
            None,
        );
    }

    // Bind gossip transport: TCP for public nodes, WebSocket for leaf nodes.
    let (mut gossip, mut ws_gossip) = match discovery_mode {
        DiscoveryMode::Tcp => {
            let gossip = TcpGossipNode::bind_with_discovery(
                bind_addr,
                psk,
                local_announcement.clone(),
                Some(api_state.peer_directory.clone()),
            )
            .await
            .expect("failed to bind gossip socket");
            info!("gossip bound to {}", gossip.local_addr);
            (Some(gossip), None)
        }
        DiscoveryMode::WebSocket => {
            let ws = WsGossipNode::new_with_discovery(
                psk,
                local_announcement.clone(),
                Some(api_state.peer_directory.clone()),
            )
            .await;
            info!("websocket gossip transport ready");
            (None, Some(ws))
        }
    };

    let outbound: mpsc::Sender<Signal> = match (&gossip, &ws_gossip) {
        (Some(g), _) => g.outbound.clone(),
        (_, Some(w)) => w.outbound.clone(),
        _ => unreachable!("at least one gossip transport must be active"),
    };
    api_state.set_gossip(outbound.clone());

    // Announce this public node to the Mainline DHT and via mDNS so LAN peers
    // can find it even without a DNS seed or bootstrap URL.  When the public
    // endpoint carries a different port than the local bind address (e.g. a
    // Railway TCP proxy), announce the public port to the DHT so leaf nodes can
    // dial the correct address.
    if discovery_mode == DiscoveryMode::Tcp && !public_endpoint.is_empty() {
        let dht_port = EndpointScheme::parse(&public_endpoint)
            .and_then(|(_, rest)| rest.rsplit_once(':').and_then(|(_, p)| p.parse().ok()))
            .unwrap_or_else(|| bind_addr.port());
        if let Some(dht) = dht_discovery.clone() {
            tokio::spawn(async move {
                dht.announce_peer(dht_port).await;
            });
        }
        if let Some(ip) = local_announce_ip(bind_addr) {
            let instance_name = format!("fluidic-{}", hex::encode(&id[..8]));
            match mdns_announce(&instance_name, &public_endpoint, ip, bind_addr.port()) {
                Ok(daemon) => {
                    tokio::task::spawn_blocking(move || {
                        // Hold the daemon alive for the process lifetime.
                        let _ = daemon;
                        loop {
                            std::thread::park();
                        }
                    });
                }
                Err(e) => warn!("mdns announce failed: {}", e),
            }
        }
    }

    // Build the local operator's stake announcement.  We re-announce it
    // reliably on an interval so peers that connect after we start still learn
    // our operator public key and can verify our synthesis certificates.
    // Client nodes do not operate, so they have no stake to announce.
    if !client_mode {
        let timestamp_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let stake_signal = Signal::Stake(StakeShift::sign(
            &local_keypair,
            genesis_balance,
            0,
            timestamp_ns,
        ));

        let announce_outbound = outbound.clone();
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(3));
            loop {
                ticker.tick().await;
                match announce_outbound.send(stake_signal.clone()).await {
                    Ok(_) => trace!("re-announced operator stake"),
                    Err(e) => warn!("stake re-announce send error: {}", e),
                }
            }
        });
    }

    // Wait for concurrent peer discovery to finish before connecting.
    if let Some(task) = seed_dns_task {
        match task.await {
            Ok(addrs) => peers.extend(addrs),
            Err(e) => warn!("legacy DNS seed lookup task failed: {}", e),
        }
    }
    if let Some(task) = bootstrap_dns_task {
        match task.await {
            Ok(endpoints) => {
                for ep in endpoints {
                    add_bootstrap_peer(&mut peers, &ep);
                }
            }
            Err(e) => warn!("bootstrap DNS TXT lookup task failed: {}", e),
        }
    }
    if let Some(task) = bootstrap_url_task {
        match task.await {
            Ok(endpoints) => {
                for ep in endpoints {
                    add_bootstrap_peer(&mut peers, &ep);
                }
            }
            Err(e) => warn!("bootstrap URL lookup task failed: {}", e),
        }
    }
    let (dht_peers, mdns_peers) = tokio::join!(
        dht_lookup_task,
        async {
            match mdns_lookup_task {
                Some(handle) => handle.await.unwrap_or_default(),
                None => Vec::new(),
            }
        }
    );
    for p in dht_peers.unwrap_or_default() {
        peers.push(p);
    }
    for p in mdns_peers {
        add_bootstrap_peer(&mut peers, &p);
    }
    // Deduplicate while preserving order.
    let mut seen = std::collections::HashSet::new();
    peers.retain(|p| seen.insert(p.clone()));

    // Connect to peers.  Endpoints may be raw TCP host:port, tcp://host:port, or
    // a full ws/wss URL.  Unknown strings are treated as TCP host:port for
    // backwards compatibility.
    let mut peer_addrs: Vec<SocketAddr> = Vec::new();
    for peer in &peers {
        match EndpointScheme::parse(peer) {
            Some((EndpointScheme::Tcp, rest)) => {
                match tokio::net::lookup_host(rest).await {
                    Ok(mut addrs) => {
                        if let Some(addr) = addrs.next() {
                            peer_addrs.push(addr);
                            if let Some(ref g) = gossip {
                                if let Err(e) = g.add_peer(addr).await {
                                    warn!("failed to queue peer {}: {}", addr, e);
                                }
                            }
                        } else {
                            warn!("peer {} resolved to no addresses", peer);
                        }
                    }
                    Err(e) => warn!("failed to resolve peer {}: {}", peer, e),
                }
            }
            Some((EndpointScheme::Ws | EndpointScheme::Wss, _)) => {
                if let Some(ref w) = ws_gossip {
                    if let Err(e) = w.add_peer(peer.clone()).await {
                        warn!("failed to queue websocket peer {}: {}", peer, e);
                    }
                } else {
                    warn!("ignoring websocket peer {} on a TCP-only node", peer);
                }
            }
            None => {
                match tokio::net::lookup_host(peer).await {
                    Ok(mut addrs) => {
                        if let Some(addr) = addrs.next() {
                            peer_addrs.push(addr);
                            if let Some(ref g) = gossip {
                                if let Err(e) = g.add_peer(addr).await {
                                    warn!("failed to queue peer {}: {}", addr, e);
                                }
                            }
                        } else {
                            warn!("peer {} resolved to no addresses", peer);
                        }
                    }
                    Err(e) => warn!("failed to resolve peer {}: {}", peer, e),
                }
            }
        }
    }
    info!("queued {} bootstrap peer(s)", peer_addrs.len());

    // Periodically save peer directory cache.
    let cache_dir = std::path::PathBuf::from(&data_dir);
    let cache_peer_dir = api_state.peer_directory.clone();
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(30));
        loop {
            ticker.tick().await;
            cache_peer_dir.save(&cache_dir.join("peers.json"));
        }
    });


    // Ingest loop: apply incoming phase-shifts to the oscillator.
    let osc_ingest = oscillator.clone();
    let api_state_ingest = api_state.clone();
    let light_client_ingest = light_client.clone();
    let ping_outbound = outbound.clone();
    let mut inbound = match (gossip.take(), ws_gossip.take()) {
        (Some(g), _) => g.inbound,
        (_, Some(w)) => w.inbound,
        _ => unreachable!(),
    };
    tokio::spawn(async move {
        while let Some(shift) = inbound.recv().await {
            match shift {
                Signal::Registration(reg) => {
                    let vk = match ed25519_dalek::VerifyingKey::from_bytes(&reg.public_key) {
                        Ok(vk) => vk,
                        Err(_) => {
                            warn!("invalid public key in registration gossip");
                            continue;
                        }
                    };
                    api_state_ingest.register_key(reg.account, vk);
                    api_state_ingest.register_key(reg.wave_account, vk);
                    api_state_ingest.register_key(reg.usdc_account, vk);
                    api_state_ingest.register_derived(reg.wave_account, reg.account);
                    api_state_ingest.register_derived(reg.usdc_account, reg.account);
                    if !client_mode {
                        osc_ingest.apply_registration(&reg);
                    }
                }
                Signal::Stake(stake) => {
                    // Learn the operator's public key from the stake announcement
                    // so we can verify synthesis certificates from peers that join
                    // before we do.
                    if let Ok(vk) = ed25519_dalek::VerifyingKey::from_bytes(&stake.public_key) {
                        api_state_ingest.register_key(stake.operator, vk);
                        if let Some(ref lc) = light_client_ingest {
                            lc.register_key(stake.operator, vk);
                        }
                    }
                    if client_mode {
                        // Clients trust observed operator stakes without requiring
                        // local liquid balance; they are not synthesizing themselves.
                        if stake.verify() {
                            osc_ingest.stake_table.stake(stake.operator, stake.amount);
                        } else {
                            warn!("rejected invalid stake gossip from {}", stake.operator);
                        }
                    } else if !osc_ingest.apply_stake(&stake) {
                        warn!("rejected invalid stake gossip from {}", stake.operator);
                    }
                }
                Signal::Ping { timestamp_ns, nonce } => {
                    let _ = ping_outbound.try_send(Signal::Pong { timestamp_ns, nonce });
                }
                Signal::Pong { timestamp_ns, .. } => {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos() as u64)
                        .unwrap_or(0);
                    let rtt_ms = (now.saturating_sub(timestamp_ns)) as f64 / 1_000_000.0;
                    api_state_ingest.record_network_latency_ms(rtt_ms);
                }
                Signal::Certificate(cert) => {
                    if let Some(ref lc) = light_client_ingest {
                        match lc.ingest_certificate(cert) {
                            Ok(Some(view)) => {
                                info!(
                                    "light client finalized tick {} roots comm={} state={} evm={}",
                                    lc.latest_finalized_tick(),
                                    hex::encode(view.commutative_root),
                                    hex::encode(view.stateful_root),
                                    hex::encode(view.evm_root)
                                );
                            }
                            Ok(None) => {}
                            Err(e) => warn!("light client rejected certificate: {}", e),
                        }
                    } else {
                        let registry = api_state_ingest.key_registry();
                        if let Err(e) = osc_ingest.ingest_certificate(cert.clone(), &registry) {
                            warn!("rejected peer certificate: {:?}", e);
                        } else {
                            trace!("accepted certificate for tick {} from {}", cert.tick, cert.operator);
                        }
                    }
                }
                Signal::PeerAnnounce(anns) => {
                    api_state_ingest.peer_directory.insert_announcements(&anns, None);
                }
                Signal::Auth { .. } => {
                    // Authentication is handled at the gossip layer; once a
                    // Signal reaches the ingest loop the peer is trusted.
                }
                other => {
                    if !client_mode {
                        let registry = api_state_ingest.key_registry();
                        if let Err(e) = osc_ingest.ingest(other, &registry) {
                            warn!("ingest error: {}", e);
                        }
                    }
                }
            }
        }
    });

    // Gossip RTT probe loop.
    let ping_sender = outbound.clone();
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(1));
        let mut nonce = 0u64;
        loop {
            ticker.tick().await;
            let timestamp_ns = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0);
            if let Err(e) = ping_sender.try_send(Signal::Ping { timestamp_ns, nonce }) {
                warn!("ping send error: {}", e);
            }
            nonce += 1;
        }
    });

    // Generator loop: emit periodic commutative phase-shifts.
    // Apply them locally so the node synthesizes real activity, and broadcast
    // them so any connected peers see the same load. Disabled by default on
    // public deployments to avoid burning WAVE on synthetic traffic.
    let enable_generator = std::env::var("ENABLE_GENERATOR")
        .unwrap_or_else(|_| "false".to_string())
        .eq_ignore_ascii_case("true");
    if enable_generator {
        let sender = outbound.clone();
        let generator_key = local_keypair.clone();
        let osc_gen = oscillator.clone();
        let api_gen = api_state.clone();
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_millis(generator_interval_ms));
            let mut nonce = 0u64;
            let pool = [0xAB; 32];
            loop {
                ticker.tick().await;
                let shift = CommutativeShift::new(
                    &generator_key,
                    DEFAULT_DEX_DOMAIN,
                    Coordinate::from_scalar(nonce),
                    1_000_000,
                    pool,
                    nonce,
                    0,
                );
                let signal = Signal::Commutative(shift.clone());
                let registry = api_gen.key_registry();
                if let Err(e) = osc_gen.ingest(signal.clone(), &registry) {
                    warn!("local generator ingest error: {}", e);
                } else {
                    let hash = hex::encode(shift.hash());
                    api_gen.record_shift(RecentShift {
                        hash,
                        kind: "commutative".to_string(),
                        status: "accepted".to_string(),
                        domain: Some(hex::encode(shift.domain)),
                        from: Some(shift.from.to_string()),
                        to: None,
                        amount: Some(shift.delta.to_string()),
                        token: Some("WAVE".to_string()),
                        timestamp_ns: shift.timestamp_ns,
                    });
                }
                if let Err(e) = sender.send(signal).await {
                    warn!("broadcast error: {}", e);
                    return;
                }
                nonce += 1;
            }
        });
    } else {
        info!("generator loop disabled; set ENABLE_GENERATOR=true to enable synthetic commutative traffic");
    }

    // Periodic snapshot save.
    let osc_save = oscillator.clone();
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_millis(snapshot_interval_ms));
        loop {
            ticker.tick().await;
            if let Err(e) = persistence::save(&osc_save, persistence::snapshot_path()) {
                warn!("snapshot save failed: {}", e);
            }
        }
    });

    // Synthesis loop with graceful shutdown.
    let mut synth_ticker = interval(Duration::from_millis(synth_interval_ms));
    let mut shutdown = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("failed to install SIGTERM handler");

    loop {
        tokio::select! {
            _ = synth_ticker.tick() => {
                let registry = api_state.key_registry();
                let result = oscillator.synthesize(&registry);
                api_state.record_synthesis(&result);

                if !client_mode {
                    // Gossip our own certificate so peers can form a quorum.
                    let tick = oscillator.synthesis_tick.load(std::sync::atomic::Ordering::SeqCst).saturating_sub(1);
                    if let Some(cert) = oscillator.certificates.read().unwrap().get(&tick).cloned() {
                        if let Err(e) = outbound.try_send(Signal::Certificate(cert)) {
                            warn!("failed to gossip certificate: {}", e);
                        }
                    }

                    // Check for a stake-weighted quorum on the previous tick.
                    if let Some((view, stake)) = oscillator.check_quorum(tick) {
                        info!(
                            "quorum reached for tick {} with stake {}/{} on roots comm={} state={} evm={}",
                            tick,
                            stake,
                            oscillator.stake_table.total_stake(),
                            hex::encode(view.commutative_root),
                            hex::encode(view.stateful_root),
                            hex::encode(view.evm_root),
                        );
                    }
                }

                info!(
                    "synthesis: commutative={} stateful={} evm={} rejected={} latency_ms={:.2} throughput={:.1}",
                    result.commutative_applied,
                    result.stateful_applied,
                    result.evm_applied,
                    result.stateful_rejected.len(),
                    result.avg_latency_ms,
                    result.throughput_per_sec,
                );
                for err in &result.stateful_rejected {
                    tracing::warn!("stateful shift rejected: {:?}", err);
                }
            }
            _ = shutdown.recv() => {
                info!("SIGTERM received, saving snapshot and shutting down");
                if let Err(e) = persistence::save(&oscillator, persistence::snapshot_path()) {
                    warn!("final snapshot save failed: {}", e);
                }
                std::process::exit(0);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DiscoveryMode {
    Tcp,
    WebSocket,
}
