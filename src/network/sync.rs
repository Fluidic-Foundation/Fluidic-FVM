//! State synchronization helpers for bootstrapping a full node from a peer.
//!
//! A fresh operator that starts synthesizing immediately will diverge from the
//! network and break quorum.  Before enabling synthesis, full nodes fetch the
//! current state from an existing peer so their roots match.

use crate::consensus::certificate::SynthesisCertificate;
use crate::consensus::Oscillator;
use crate::crypto::AccountId;
use crate::field::wave_field::{AccountState, Balance};
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
        field.accounts.clear();
        for (hex, bal) in state.balances {
            let account = parse_account(&hex)?;
            let units = parse_u128(&bal.units)?;
            let mut balance = Balance::zero();
            balance.units = units;
            balance.last_decay_tick = bal.last_decay_tick;
            balance.decays = bal.decays;
            balance.last_active_tick = bal.last_active_tick;
            field.accounts.insert(
                account,
                AccountState {
                    balance,
                    frequency_vector: Default::default(),
                },
            );
        }

        // Apply pool balances.
        field.pools.clear();
        for (hex, units_str) in state.pools {
            let pool_id = parse_pool(&hex)?;
            let units = parse_u128(&units_str)?;
            let mut balance = Balance::zero();
            balance.units = units;
            field.pools.insert(pool_id, balance);
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

/// Try to sync from any of the provided peer endpoints.
pub async fn sync_from_peers(
    osc: &Oscillator,
    peers: &[String],
) -> Result<(u64, HashMap<AccountId, ed25519_dalek::VerifyingKey>), String> {
    for peer in peers {
        let Some(url) = api_url_from_peer(peer) else {
            continue;
        };
        match fetch_sync_state(&url).await {
            Ok(state) => {
                let tick = state.synthesis_tick;
                let registry = apply_sync_state(osc, state)?;
                return Ok((tick, registry));
            }
            Err(e) => {
                tracing::warn!("sync from {} failed: {}", url, e);
            }
        }
    }
    Err("failed to sync from any peer".to_string())
}
