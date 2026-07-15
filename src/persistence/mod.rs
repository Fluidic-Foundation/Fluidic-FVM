use crate::consensus::dag::{DagError, ShiftStatus};
use crate::consensus::Oscillator;
use crate::crypto::{AccountId, DomainId, StatefulShift};
use crate::evm::EvmTxStatus;
use crate::field::wave_field::Balance;
use crate::operator::stake::{OperatorEntry, StakeTable, StakingConfig};
use ethers_core::types::{Address as EvmAddress, H256};
use revm::InMemoryDB;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;


/// On-disk snapshot of the entire oscillator state.
#[derive(Serialize, Deserialize)]
struct Snapshot {
    version: u32,
    accounts: Vec<(String, String, AccountStateSer)>,
    pools: Vec<(String, String, u128)>,
    dag_nodes: Vec<DagNodeSer>,
    dag_balances: Vec<(String, u128)>,
    total_burned: u128,
    #[serde(default)]
    evm_db: Option<InMemoryDB>,
    #[serde(default)]
    evm_nonces: Vec<(String, u64)>,
    #[serde(default)]
    evm_statuses: Vec<(String, EvmTxStatus)>,
    #[serde(default)]
    stake_config: Option<StakingConfig>,
    #[serde(default)]
    stake_table: BTreeMap<String, OperatorEntry>,
    #[serde(default)]
    entanglements: Vec<crate::crypto::EntanglementContract>,
}

#[derive(Serialize, Deserialize)]
struct AccountStateSer {
    units: u128,
    #[serde(default = "default_true")]
    decays: bool,
    #[serde(default)]
    last_active_tick: u64,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize)]
struct DagNodeSer {
    hash: String,
    shift: StatefulShift,
    children: Vec<String>,
    inserted_at_tick: u64,
    #[serde(default = "default_finalization_depth")]
    finalization_depth: u64,
    #[serde(default)]
    first_seen_at_ns: u64,
    #[serde(default)]
    finalized_at_tick: Option<u64>,
    #[serde(default)]
    applied: bool,
    status: String,
    error: Option<String>,
}

fn default_finalization_depth() -> u64 {
    crate::consensus::dag::VectorClockDag::FINALIZATION_DEPTH
}

fn account_to_hex(a: &AccountId) -> String {
    hex::encode(a.0)
}

fn account_from_hex(s: &str) -> Option<AccountId> {
    let bytes = hex::decode(s).ok()?;
    if bytes.len() != 32 {
        return None;
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Some(AccountId(arr))
}

fn pool_to_hex(p: &crate::crypto::PoolId) -> String {
    hex::encode(p)
}

fn pool_from_hex(s: &str) -> Option<crate::crypto::PoolId> {
    let bytes = hex::decode(s).ok()?;
    if bytes.len() != 32 {
        return None;
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Some(arr)
}

fn hash_to_hex(h: &[u8; 32]) -> String {
    hex::encode(h)
}

fn hash_from_hex(s: &str) -> Option<[u8; 32]> {
    let bytes = hex::decode(s).ok()?;
    if bytes.len() != 32 {
        return None;
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Some(arr)
}

/// Persist oscillator state to `path`.
pub fn save(osc: &Oscillator, path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    // Consistent lock order with the rest of the oscillator: dag first, then wave_field.
    let dag = osc.dag.lock().map_err(|e| e.to_string())?;
    let wave = osc.wave_field.lock().map_err(|e| e.to_string())?;

    let mut accounts: Vec<(String, String, AccountStateSer)> = Vec::new();
    for domain_entry in wave.domains.iter() {
        let domain_hex = hex::encode(*domain_entry.key());
        for entry in domain_entry.value().accounts.iter() {
            accounts.push((
                account_to_hex(entry.key()),
                domain_hex.clone(),
                AccountStateSer {
                    units: entry.value().balance.units,
                    decays: entry.value().balance.decays,
                    last_active_tick: entry.value().balance.last_active_tick,
                },
            ));
        }
    }

    let mut pools: Vec<(String, String, u128)> = Vec::new();
    for domain_entry in wave.domains.iter() {
        let domain_hex = hex::encode(*domain_entry.key());
        for entry in domain_entry.value().pools.iter() {
            pools.push((pool_to_hex(entry.key()), domain_hex.clone(), entry.value().units));
        }
    }

    let dag_nodes: Vec<_> = dag
        .nodes
        .values()
        .map(|node| DagNodeSer {
            hash: hash_to_hex(&node.hash),
            shift: node.shift.clone(),
            children: node.children.iter().map(hash_to_hex).collect(),
            inserted_at_tick: node.inserted_at_tick,
            finalization_depth: node.finalization_depth,
            first_seen_at_ns: node.first_seen_at_ns,
            finalized_at_tick: node.finalized_at_tick,
            applied: node.applied,
            status: match node.status {
                ShiftStatus::Accepted => "accepted".to_string(),
                ShiftStatus::Finalized => "finalized".to_string(),
                ShiftStatus::Rejected(ref err) => format!("rejected:{}", dag_error_code(err)),
            },
            error: match node.status {
                ShiftStatus::Rejected(ref err) => Some(dag_error_code(err)),
                _ => None,
            },
        })
        .collect();

    let dag_balances: Vec<_> = dag
        .balances
        .iter()
        .map(|(k, v)| (account_to_hex(k), *v))
        .collect();

    let total_burned = *osc
        .metabolic_engine
        .total_burned
        .lock()
        .map_err(|e| e.to_string())?;

    let evm = osc.evm_pool.lock().map_err(|e| e.to_string())?;
    let evm_nonces: Vec<_> = evm
        .nonces
        .iter()
        .map(|(addr, nonce)| (format!("0x{}", hex::encode(addr.as_bytes())), *nonce))
        .collect();
    let evm_statuses: Vec<_> = evm
        .statuses
        .iter()
        .map(|(hash, status)| (format!("0x{}", hex::encode(hash.as_bytes())), status.clone()))
        .collect();
    let evm_db = evm.db.clone();
    drop(evm);

    let stake_config = Some(osc.stake_table.config().clone());
    let stake_table = osc.stake_table.to_snapshot();
    let entanglements: Vec<_> = osc
        .entanglements
        .read()
        .map_err(|e| e.to_string())?
        .values()
        .cloned()
        .collect();

    let snapshot = Snapshot {
        version: 3,
        accounts,
        pools,
        dag_nodes,
        dag_balances,
        total_burned,
        evm_db: Some(evm_db),
        evm_nonces,
        evm_statuses,
        stake_config,
        stake_table,
        entanglements,
    };

    let tmp = path.with_extension("tmp");
    let json = serde_json::to_string_pretty(&snapshot).map_err(|e| e.to_string())?;
    fs::write(&tmp, json).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())?;

    drop(wave);
    drop(dag);
    Ok(())
}

/// Load oscillator state from `path` into an existing oscillator.
pub fn load(osc: &mut Oscillator, path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(());
    }

    let json = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let snapshot: Snapshot = serde_json::from_str(&json).map_err(|e| e.to_string())?;

    if snapshot.version != 2 && snapshot.version != 3 {
        return Err(format!(
            "unsupported snapshot version {} (expected 2 or 3)",
            snapshot.version
        ));
    }

    // Consistent lock order with synthesis: dag first, then wave_field.
    let mut dag = osc.dag.lock().map_err(|e| e.to_string())?;
    let wave = osc.wave_field.lock().map_err(|e| e.to_string())?;

    wave.domains.clear();
    for (hex, domain_hex, state) in snapshot.accounts {
        if let (Some(id), Some(domain_bytes)) = (
            account_from_hex(&hex),
            hex::decode(&domain_hex).ok().and_then(|b| {
                if b.len() == 32 {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&b);
                    Some(arr)
                } else {
                    None
                }
            }),
        ) {
            let domain: DomainId = domain_bytes;
            wave.ensure_account_in_domain(domain, id);
            if let Some(domain_state) = wave.domains.get(&domain) {
                if let Some(mut account) = domain_state.accounts.get_mut(&id) {
                    account.balance.units = state.units;
                    account.balance.decays = state.decays;
                    account.balance.last_active_tick = state.last_active_tick;
                }
            }
        }
    }

    for (hex, domain_hex, units) in snapshot.pools {
        if let (Some(id), Some(domain_bytes)) = (
            pool_from_hex(&hex),
            hex::decode(&domain_hex).ok().and_then(|b| {
                if b.len() == 32 {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&b);
                    Some(arr)
                } else {
                    None
                }
            }),
        ) {
            let domain: DomainId = domain_bytes;
            wave.ensure_domain(domain);
            if let Some(domain_state) = wave.domains.get(&domain) {
                domain_state
                    .pools
                    .insert(id, Balance { units, ..Default::default() });
            }
        }
    }

    dag.nodes.clear();
    dag.roots.clear();
    dag.tips.clear();
    dag.balances.clear();
    dag.rejected.clear();

    // First pass: insert nodes.
    for node in &snapshot.dag_nodes {
        if let Some(hash) = hash_from_hex(&node.hash) {
            dag.nodes.insert(
                hash,
                crate::consensus::dag::DagNode {
                    hash,
                    shift: node.shift.clone(),
                    children: node.children.iter().filter_map(|h| hash_from_hex(h)).collect(),
                    inserted_at_tick: node.inserted_at_tick,
                    finalization_depth: node.finalization_depth,
                    first_seen_at_ns: node.first_seen_at_ns,
                    finalized_at_tick: node.finalized_at_tick,
                    applied: node.applied,
                    status: parse_status(&node.status, &node.error),
                },
            );
        }
    }

    // Rebuild roots.
    let roots: Vec<_> = dag
        .nodes
        .iter()
        .filter(|(_, node)| node.shift.predecessors.is_empty())
        .map(|(hash, _)| *hash)
        .collect();
    for hash in roots {
        dag.roots.insert(hash);
    }

    // Rebuild tips.
    for node in snapshot.dag_nodes {
        if let Some(hash) = hash_from_hex(&node.hash) {
            let from = dag.nodes.get(&hash).map(|n| n.shift.from);
            if let Some(from) = from {
                dag.tips.insert(from, hash);
            }
        }
    }

    for (hex, units) in snapshot.dag_balances {
        if let Some(id) = account_from_hex(&hex) {
            dag.balances.insert(id, units);
        }
    }

    drop(wave);
    drop(dag);

    if let Ok(mut burned) = osc.metabolic_engine.total_burned.lock() {
        *burned = snapshot.total_burned;
    }

    if let Ok(mut evm) = osc.evm_pool.lock() {
        evm.db = snapshot.evm_db.unwrap_or_default();
        evm.nonces.clear();
        for (hex, nonce) in snapshot.evm_nonces {
            if let Ok(addr) = hex.parse::<EvmAddress>() {
                evm.nonces.insert(addr, nonce);
            }
        }
        evm.statuses.clear();
        for (hex, status) in snapshot.evm_statuses {
            if let Ok(bytes) = hex::decode(hex.trim_start_matches("0x")) {
                if bytes.len() == 32 {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    evm.statuses.insert(H256::from(arr), status);
                }
            }
        }
    }

    osc.stake_table = Arc::new(StakeTable::from_snapshot(
        snapshot.stake_config.unwrap_or_default(),
        snapshot.stake_table,
    ));

    {
        let mut entanglements = osc.entanglements.write().map_err(|e| e.to_string())?;
        entanglements.clear();
        for contract in snapshot.entanglements {
            entanglements.insert(contract.id, contract);
        }
    }

    Ok(())
}

fn parse_status(status: &str, error: &Option<String>) -> ShiftStatus {
    if let Some(code) = error.as_deref() {
        return ShiftStatus::Rejected(parse_dag_error(code));
    }
    if let Some(code) = status.strip_prefix("rejected:") {
        return ShiftStatus::Rejected(parse_dag_error(code));
    }
    match status {
        "finalized" => ShiftStatus::Finalized,
        _ => ShiftStatus::Accepted,
    }
}

fn dag_error_code(err: &DagError) -> String {
    match err {
        DagError::MissingPredecessor(_) => "missing_predecessor",
        DagError::InvalidSignature(_) => "invalid_signature",
        DagError::InsufficientBalance(_) => "insufficient_balance",
        DagError::DoubleSpend(_) => "double_spend",
        DagError::CausalCycle(_) => "causal_cycle",
        DagError::EntanglementThresholdNotMet(_) => "entanglement_threshold_not_met",
    }
    .to_string()
}

fn parse_dag_error(code: &str) -> DagError {
    match code {
        "invalid_signature" => DagError::InvalidSignature([0u8; 32]),
        "insufficient_balance" => DagError::InsufficientBalance([0u8; 32]),
        "double_spend" => DagError::DoubleSpend([0u8; 32]),
        "causal_cycle" => DagError::CausalCycle([0u8; 32]),
        "entanglement_threshold_not_met" => DagError::EntanglementThresholdNotMet([0u8; 32]),
        _ => DagError::MissingPredecessor([0u8; 32]),
    }
}

pub fn snapshot_path() -> PathBuf {
    let dir = std::env::var("FLUIDIC_DATA_DIR").unwrap_or_else(|_| "./data".to_string());
    PathBuf::from(dir).join("snapshot.json")
}
