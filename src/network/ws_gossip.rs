use crate::api::state::ApiState;
use crate::crypto::Signal;
use crate::network::discovery::PeerAnnouncement;
use crate::network::directory::PeerDirectory;
use crate::network::node::encode_packet;
use bytes::{Buf, Bytes, BytesMut};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tungstenite::Message;
use futures_util::{SinkExt, StreamExt};
use tracing::{error, info, trace, warn};

const AUTH_CHALLENGE: &[u8] = b"fluidic:gossip:auth:v1";
const MAX_PACKET_SIZE: usize = 64 * 1024;
const PEER_EXCHANGE_BATCH_SIZE: usize = 16;

/// Read buffer used while decoding length-prefixed packets from a byte stream.
struct LengthReader {
    buf: BytesMut,
    want: Option<usize>,
}

impl LengthReader {
    fn new() -> Self {
        Self {
            buf: BytesMut::with_capacity(8 * 1024),
            want: None,
        }
    }

    fn push(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    /// Try to read the next complete packet. Returns `None` if more bytes are
    /// needed, or an oversized/invalid packet is encountered.
    fn next_packet(&mut self) -> Option<Bytes> {
        if let Some(want) = self.want {
            if self.buf.len() < want {
                return None;
            }
            self.want = None;
            return Some(self.buf.split_to(want).freeze());
        }
        if self.buf.len() < 4 {
            return None;
        }
        let len = u32::from_le_bytes([self.buf[0], self.buf[1], self.buf[2], self.buf[3]]) as usize;
        self.buf.advance(4);
        if len > MAX_PACKET_SIZE {
            return None;
        }
        self.want = Some(len);
        self.next_packet()
    }
}

/// Gossip transport that wraps the mesh node API's `/api/ws` WebSocket endpoint.
/// This lets user-run nodes connect to a publicly-hosted Fluidic node (e.g. on
/// Railway) even when raw TCP port 7000 is not exposed.
pub struct WsGossipNode {
    /// Send channel for outbound phase-shifts to all connected peers.
    pub outbound: mpsc::Sender<Signal>,
    peer_tx: mpsc::Sender<String>,
    /// Receive channel for inbound phase-shifts from all connected peers.
    pub inbound: mpsc::Receiver<Signal>,
    local_announcement: Option<PeerAnnouncement>,
    peer_directory: Option<PeerDirectory>,
}

impl WsGossipNode {
    pub async fn new(psk: Option<[u8; 32]>) -> Self {
        Self::new_with_discovery(psk, None, None).await
    }

    pub async fn new_with_discovery(
        psk: Option<[u8; 32]>,
        local_announcement: Option<PeerAnnouncement>,
        peer_directory: Option<PeerDirectory>,
    ) -> Self {
        let (outbound_tx, outbound_rx) = mpsc::channel(4096);
        let (peer_tx, peer_rx) = mpsc::channel(64);
        let (inbound_tx, inbound_rx) = mpsc::channel(4096);

        tokio::spawn(run_gossip(
            inbound_tx,
            outbound_rx,
            peer_rx,
            psk,
            local_announcement.clone(),
            peer_directory.clone(),
        ));

        Self {
            outbound: outbound_tx,
            peer_tx,
            inbound: inbound_rx,
            local_announcement,
            peer_directory,
        }
    }

    pub async fn add_peer(&self, url: String) -> Result<(), String> {
        self.peer_tx
            .send(url)
            .await
            .map_err(|e| format!("peer channel closed: {}", e))
    }

    pub async fn broadcast(&self, shift: Signal) -> Result<(), String> {
        self.outbound
            .send(shift)
            .await
            .map_err(|e| format!("outbound channel closed: {}", e))
    }
}

async fn run_gossip(
    inbound_tx: mpsc::Sender<Signal>,
    mut outbound_rx: mpsc::Receiver<Signal>,
    mut peer_rx: mpsc::Receiver<String>,
    psk: Option<[u8; 32]>,
    local_announcement: Option<PeerAnnouncement>,
    peer_directory: Option<PeerDirectory>,
) {
    let (writer_tx, mut writer_rx) = mpsc::channel::<mpsc::Sender<Bytes>>(64);
    let dial_peers: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
    let active_peers: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));

    let auth_proof = psk.map(|key| {
        let mut hasher = blake3::Hasher::new_keyed(&key);
        hasher.update(AUTH_CHALLENGE);
        *hasher.finalize().as_bytes()
    });

    // Dial outbound peers requested via add_peer and reconnect automatically.
    let tx = writer_tx.clone();
    let inbound = inbound_tx.clone();
    let dial_peers_task = dial_peers.clone();
    let active_peers_task = active_peers.clone();
    let outbound_psk = auth_proof;
    let outbound_local_announcement = local_announcement.clone();
    let outbound_peer_directory = peer_directory.clone();
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(5));
        loop {
            tokio::select! {
                Some(peer) = peer_rx.recv() => {
                    dial_peers_task.lock().unwrap().insert(peer.clone());
                    try_connect(&peer, tx.clone(), inbound.clone(), active_peers_task.clone(),
                        outbound_psk, outbound_local_announcement.clone(), outbound_peer_directory.clone()
                    ).await;
                }
                _ = ticker.tick() => {
                    let to_retry: Vec<String> = {
                        let dial = dial_peers_task.lock().unwrap();
                        let active = active_peers_task.lock().unwrap();
                        dial.difference(&active).cloned().collect()
                    };
                    for peer in to_retry {
                        try_connect(&peer, tx.clone(), inbound.clone(), active_peers_task.clone(),
                            outbound_psk, outbound_local_announcement.clone(), outbound_peer_directory.clone()
                        ).await;
                    }
                }
                else => break,
            }
        }
    });

    // Fan-out loop: distribute outbound shifts to all connected peers.
    let mut writers: Vec<mpsc::Sender<Bytes>> = Vec::new();
    loop {
        tokio::select! {
            Some(writer) = writer_rx.recv() => {
                writers.push(writer);
            }
            Some(shift) = outbound_rx.recv() => {
                let packet = match encode_packet(&shift) {
                    Ok(p) => Bytes::from(p),
                    Err(e) => {
                        error!("encode error: {}", e);
                        continue;
                    }
                };
                let mut disconnected = Vec::new();
                for (i, writer) in writers.iter_mut().enumerate() {
                    if writer.send(packet.clone()).await.is_err() {
                        disconnected.push(i);
                    }
                }
                for i in disconnected.into_iter().rev() {
                    writers.remove(i);
                }
            }
            else => break,
        }
    }
}

async fn try_connect(
    url: &str,
    writer_tx: mpsc::Sender<mpsc::Sender<Bytes>>,
    inbound_tx: mpsc::Sender<Signal>,
    active_peers: Arc<Mutex<HashSet<String>>>,
    auth_proof: Option<[u8; 32]>,
    local_announcement: Option<PeerAnnouncement>,
    peer_directory: Option<PeerDirectory>,
) {
    {
        let active = active_peers.lock().unwrap();
        if active.contains(url) {
            return;
        }
    }

    let url_owned = url.to_string();

    let (ws_stream, _) = match tokio_tungstenite::connect_async(url).await {
        Ok(v) => v,
        Err(e) => {
            trace!("failed to connect to websocket peer {}: {}", url, e);
            return;
        }
    };
    info!("connected to websocket peer {}", url);

    active_peers.lock().unwrap().insert(url.to_string());
    let active = active_peers.clone();
    let url_writer = url_owned.clone();

    let (mut write, mut read) = ws_stream.split();

    // If a PSK is configured, send an Auth signal as the first binary message.
    if let Some(proof) = auth_proof {
        let signal = Signal::Auth { proof };
        match encode_packet(&signal) {
            Ok(p) => {
                if write.send(Message::Binary(p.into())).await.is_err() {
                    active.lock().unwrap().remove(&url_writer);
                    return;
                }
            }
            Err(e) => {
                warn!("failed to encode auth for {}: {}", url_writer, e);
                active.lock().unwrap().remove(&url_writer);
                return;
            }
        }
    }

    // Send peer exchange immediately after auth (or first, if no auth).
    send_ws_peer_announce(
        &mut write,
        local_announcement.clone(),
        peer_directory.as_ref(),
        Some(url),
    ).await;

    // Channel from fan-out loop to this peer's websocket writer.
    let (tx, mut rx) = mpsc::channel::<Bytes>(128);
    let _ = writer_tx.send(tx).await;

    // Writer task.
    let url_writer = url_owned.clone();
    tokio::spawn(async move {
        while let Some(packet) = rx.recv().await {
            if write.send(Message::Binary(packet)).await.is_err() {
                break;
            }
        }
        info!("websocket writer for {} closed", url_writer);
        active.lock().unwrap().remove(&url_writer);
    });

    // Reader task.
    tokio::spawn(async move {
        let mut reader = LengthReader::new();
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    reader.push(&data);
                    while let Some(packet) = reader.next_packet() {
                        match bincode::deserialize::<Signal>(&packet) {
                            Ok(Signal::Auth { proof }) => {
                                let _ = proof;
                            }
                            Ok(shift) => {
                                if inbound_tx.send(shift).await.is_err() {
                                    return;
                                }
                            }
                            Err(e) => warn!("deserialize error from {}: {}", url_owned, e),
                        }
                    }
                }
                Ok(Message::Close(_)) => break,
                Ok(_) => {}
                Err(e) => {
                    warn!("websocket read error from {}: {}", url_owned, e);
                    break;
                }
            }
        }
        info!("websocket reader for {} closed", url_owned);
    });
}

async fn send_ws_peer_announce<M: From<Vec<u8>>>(
    write: &mut (impl futures_util::Sink<M, Error = impl std::fmt::Debug> + Unpin),
    local_announcement: Option<PeerAnnouncement>,
    peer_directory: Option<&PeerDirectory>,
    exclude_endpoint: Option<&str>,
) {
    let mut announcements = Vec::with_capacity(PEER_EXCHANGE_BATCH_SIZE);
    if let Some(local) = local_announcement {
        if local.ttl > 0 {
            announcements.push(local);
        }
    }
    if let Some(dir) = peer_directory {
        let sample = dir.sample_for_forward(
            PEER_EXCHANGE_BATCH_SIZE.saturating_sub(announcements.len()),
            exclude_endpoint,
        );
        let forwarded: Vec<PeerAnnouncement> = sample
            .into_iter()
            .filter_map(|mut ann| {
                if ann.ttl == 0 {
                    return None;
                }
                ann.ttl = ann.ttl.saturating_sub(1);
                Some(ann)
            })
            .collect();
        announcements.extend(forwarded);
    }
    if announcements.is_empty() {
        return;
    }
    let signal = Signal::PeerAnnounce(announcements);
    if let Ok(packet) = encode_packet(&signal) {
        let vec: Vec<u8> = packet.into();
        let _ = write.send(vec.into()).await;
    }
}

/// Accept an inbound WebSocket upgrade and bridge it into the gossip layer.
/// Called from `/api/ws` in `routes.rs` when the request has the `x-fluidic-gossip`
/// header (or always, for the public seed node).
pub async fn handle_gossip_socket(
    ws: axum::extract::ws::WebSocket,
    state: Arc<ApiState>,
    psk: Option<[u8; 32]>,
) {
    let (mut write, mut read) = ws.split();

    let expected_proof = psk.map(|key| {
        let mut hasher = blake3::Hasher::new_keyed(&key);
        hasher.update(AUTH_CHALLENGE);
        *hasher.finalize().as_bytes()
    });

    let auth_ok = if let Some(expected) = expected_proof {
        let mut reader = LengthReader::new();
        let mut ok = false;
        while let Some(Ok(msg)) = read.next().await {
            if let axum::extract::ws::Message::Binary(data) = msg {
                reader.push(&data);
                if let Some(packet) = reader.next_packet() {
                    match bincode::deserialize::<Signal>(&packet) {
                        Ok(Signal::Auth { proof }) if proof == expected => {
                            ok = true;
                            break;
                        }
                        _ => {
                            warn!("inbound gossip auth failed");
                            return;
                        }
                    }
                }
            }
        }
        ok
    } else {
        true
    };

    if !auth_ok {
        return;
    }

    let gossip = match state.gossip.lock().unwrap().clone() {
        Some(g) => g,
        None => return,
    };

    // Reply with our peer announcements so inbound clients can discover peers.
    let local_announcement = state.peer_directory.get(
        &state.peer_directory.endpoints(None, None).first().cloned().unwrap_or_default());
    send_ws_peer_announce(
        &mut write,
        local_announcement,
        Some(&state.peer_directory),
        None,
    ).await;

    let (_tx, mut rx) = mpsc::channel::<Bytes>(128);
    tokio::spawn(async move {
        while let Some(packet) = rx.recv().await {
            if write
                .send(axum::extract::ws::Message::Binary(packet.to_vec()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Reader task: forward length-prefixed signals into the ingest pipeline.
    tokio::spawn(async move {
        let mut reader = LengthReader::new();
        while let Some(Ok(msg)) = read.next().await {
            match msg {
                axum::extract::ws::Message::Binary(data) => {
                    reader.push(&data);
                    while let Some(packet) = reader.next_packet() {
                        match bincode::deserialize::<Signal>(&packet) {
                            Ok(shift) => {
                                if gossip.send(shift).await.is_err() {
                                    return;
                                }
                            }
                            Err(e) => warn!("deserialize error from inbound gossip: {}", e),
                        }
                    }
                }
                axum::extract::ws::Message::Close(_) => break,
                _ => {}
            }
        }
    });
}
