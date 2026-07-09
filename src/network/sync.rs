//! State synchronization helpers for bootstrapping a full node from a peer.
//!
//! A fresh operator that starts synthesizing immediately will diverge from the
//! network and break quorum.  Before enabling synthesis, full nodes fetch the
//! current state from an existing peer so their roots match.

use crate::consensus::certificate::SynthesisCertificate;
use crate::consensus::Oscillator;
use crate::crypto::{AccountId, DEFAULT_DEX_DOMAIN};
use crate::field::wave_field::{AccountState, AccountType, Balance};
use crate::operator::stake::OperatorEntry;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct SyncBalance {
    pub units: String,
    #[serde(default)]
    pub last_decay_tick: u64,
    #[serde(default = "default_true")]
    pub decays: bool,
    #[serde(default)]
    pub last_active_tick: u64,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct SyncState {
    pub synthesis_tick: u64,
    pub balances: HashMap<String, SyncBalance>,
    #[serde(default)]
    pub pools: HashMap<String, String>,
    pub registry: HashMap<String, String>,
    pub stake_table: std::collections::BTreeMap<String, OperatorEntry>,
    pub certificates: Vec<serde_json::Value>,
    #[serde(default)]
    pub total_burned: String,
}

fn parse_u128(s: &str) -> Result<u128, String> {
    s.parse::<u128>()
        .map_err(|e| format!("invalid u128 {}: {}", s, e))
}

fn parse_account(hex: &str) -> Result<AccountId, String> {
    let bytes = hex::decode(hex).map_err(|e| format!("invalid account hex: {}", e))?;
    if bytes.len() != 32 {
        return Err(format!("account hex length {} != 32", bytes.len()));
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(AccountId(arr))
}

fn parse_pool(hex: &str) -> Result<[u8; 32], String> {
    let bytes = hex::decode(hex).map_err(|e| format!("invalid pool hex: {}", e))?;
    if bytes.len() != 32 {
        return Err(format!("pool hex length {} != 32", bytes.len()));
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

/// Fetch the current sync snapshot from a peer's HTTP API.
pub async fn fetch_sync_state(base_url: &str) -> Result<SyncState, String> {
    let url = format!("{}/api/sync/state", base_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("failed to build http client: {}", e))?;
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("sync state request failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("sync state returned {}", resp.status()));
    }
    resp.json::<SyncState>()
        .await
        .map_err(|e| format!("failed to parse sync state: {}", e))
}

/// Apply a sync snapshot to the oscillator.  The existing stake table is
/// mutated in place so that any other Arc clones (e.g. a light client) observe
/// the updated state.
pub fn apply_sync_state(
    osc: &Oscillator,
    state: SyncState,
) -> Result<HashMap<AccountId, ed25519_dalek::VerifyingKey>, String> {
    // Apply balances.
    {
        let field = osc.wave_field.lock().map_err(|e| e.to_string())?;
        if let Some(dex) = field.domains.get(&DEFAULT_DEX_DOMAIN) {
            dex.accounts.clear();
            for (hex, bal) in state.balances {
                let account = parse_account(&hex)?;
                let units = parse_u128(&bal.units)?;
                let mut balance = Balance::zero();
                balance.units = units;
                balance.last_decay_tick = bal.last_decay_tick;
                balance.decays = bal.decays;
                balance.last_active_tick = bal.last_active_tick;
                dex.accounts.insert(
                    account,
                    AccountState {
                        balance,
                        frequency_vector: Default::default(),
                        account_type: AccountType::default(),
                        reputation: 0,
                    },
                );
            }

            // Apply pool balances.
            dex.pools.clear();
            for (hex, units_str) in state.pools {
                let pool_id = parse_pool(&hex)?;
                let units = parse_u128(&units_str)?;
                let mut balance = Balance::zero();
                balance.units = units;
                dex.pools.insert(pool_id, balance);
            }
        }
    }

    // Build registry.
    let mut registry = HashMap::new();
    for (hex, pk_hex) in state.registry {
        let account = parse_account(&hex)?;
        let bytes = hex::decode(&pk_hex).map_err(|e| format!("invalid pubkey hex: {}", e))?;
        if bytes.len() != 32 {
            return Err(format!("pubkey hex length {} != 32", bytes.len()));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        let pk = ed25519_dalek::VerifyingKey::from_bytes(&arr)
            .map_err(|e| format!("invalid ed25519 pubkey: {:?}", e))?;
        registry.insert(account, pk);
    }

    // Mutate stake table in place.
    osc.stake_table.load_from_snapshot(state.stake_table);

    // Apply certificates.
    {
        let mut certs = osc.certificates.write().map_err(|e| e.to_string())?;
        certs.clear();
        for val in state.certificates {
            let cert: SynthesisCertificate = serde_json::from_value(val)
                .map_err(|e| format!("invalid certificate in sync: {}", e))?;
            certs.insert(cert.tick, cert);
        }
    }

    // Set synthesis tick.
    osc.synthesis_tick.store(
        state.synthesis_tick,
        std::sync::atomic::Ordering::SeqCst,
    );

    // Apply total burned.
    if let Ok(burned) = parse_u128(&state.total_burned) {
        if let Ok(mut total) = osc.metabolic_engine.total_burned.lock() {
            *total = burned;
        }
    }

    Ok(registry)
}

/// Guess a reachable HTTP API URL from a gossip endpoint string.
/// TCP endpoints `host:port` are mapped to `http://host:port`.
/// WebSocket URLs are mapped to their https/http origin.
/// Already-complete http/https URLs are returned unchanged.
pub fn api_url_from_peer(peer: &str) -> Option<String> {
    let peer = peer.trim();
    if peer.starts_with("http://") || peer.starts_with("https://") {
        return Some(peer.to_string());
    }
    if peer.starts_with("tcp://") {
        return Some(format!("http://{}", &peer[6..]));
    }
    if peer.starts_with("ws://") {
        return Some(peer.replace("ws://", "http://"));
    }
    if peer.starts_with("wss://") {
        return Some(peer.replace("wss://", "https://"));
    }
    if peer.contains(':') && !peer.contains("//") {
        return Some(format!("http://{}", peer));
    }
    None
}

/// Default timeout for each peer sync attempt.  Keep this short so a fresh
/// seed/operator can fall back to its own genesis state quickly when the mesh
/// is still bootstrapping.
const SYNC_PEER_TIMEOUT: Duration = Duration::from_secs(5);

/// Try to sync from any of the provided peer endpoints concurrently.
/// Returns the first successful snapshot, or an error if every peer fails.
pub async fn sync_from_peers(
    osc: &Oscillator,
    peers: &[String],
) -> Result<(u64, HashMap<AccountId, ed25519_dalek::VerifyingKey>), String> {
    sync_from_peers_with_timeout(osc, peers, SYNC_PEER_TIMEOUT).await
}

/// Try to sync from any of the provided peer endpoints concurrently with a
/// configurable per-peer timeout.
pub async fn sync_from_peers_with_timeout(
    osc: &Oscillator,
    peers: &[String],
    timeout: Duration,
) -> Result<(u64, HashMap<AccountId, ed25519_dalek::VerifyingKey>), String> {
    if peers.is_empty() {
        return Err("no sync peers provided".to_string());
    }

    let futs: Vec<_> = peers
        .iter()
        .filter_map(|peer| api_url_from_peer(peer))
        .map(|url| {
            let url_for_log = url.clone();
            Box::pin(async move {
                match tokio::time::timeout(timeout, fetch_sync_state(&url)).await {
                    Ok(Ok(state)) => Ok(state),
                    Ok(Err(e)) => {
                        tracing::warn!("sync from {} failed: {}", url_for_log, e);
                        Err(e)
                    }
                    Err(_) => {
                        tracing::warn!("sync from {} timed out after {:?}", url_for_log, timeout);
                        Err(format!("timeout after {:?}", timeout))
                    }
                }
            })
        })
        .collect();

    if futs.is_empty() {
        return Err("no reachable sync peer URLs".to_string());
    }

    match futures_util::future::select_ok(futs).await {
        Ok((state, _)) => {
            let tick = state.synthesis_tick;
            let registry = apply_sync_state(osc, state)?;
            Ok((tick, registry))
        }
        Err(_) => Err("failed to sync from any peer".to_string()),
    }
}
