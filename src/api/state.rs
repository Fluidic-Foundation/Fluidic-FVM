use crate::consensus::{Oscillator, ShiftStatus, SynthesisResult};
use crate::crypto::{
    AccountId, AgentRegistrationShift, CommutativeShift, EntanglementAttestShift,
    EntanglementBreakShift, EntanglementCreateShift, IntentFillShift, IntentShift, KeyPair,
    PhysicalAttestation, RegistrationShift, Signal, StakeShift, StatefulShift, VectorClock,
};
use crate::network::PeerDirectory;
use ed25519_dalek::VerifyingKey;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::{broadcast, mpsc};

/// Derive a deterministic account for a given base and domain salt.
pub fn derive_account(base: AccountId, salt: &[u8]) -> AccountId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"fluidic:derived-account:v1");
    hasher.update(&base.0);
    hasher.update(salt);
    AccountId(hasher.finalize().into())
}

/// Snapshot broadcast to WebSocket clients.
#[derive(Clone, Debug)]
pub struct StateSnapshot {
    pub wave_reserve: u128,
    pub usdc_reserve: u128,
    pub price: f64,
    /// Signals processed per second during the last synthesis cycle.
    pub throughput: f64,
    /// Average latency (ms) from first seen to finalized for stateful + EVM signals.
    pub latency_ms: f64,
    /// Estimated network round-trip latency (ms) between peers.
    pub network_ms: f64,
    pub metabolic_burned: u128,
    pub commutative_applied: usize,
    pub stateful_applied: usize,
    pub evm_applied: usize,
    pub intents_matched: usize,
    /// Physical attestations ingested since the last snapshot.
    pub physical_attestations_ingested: usize,
    /// Physical-state intents matched since the last snapshot.
    pub physical_intents_matched: usize,
    pub accounts: HashMap<String, u128>,
}

#[derive(Clone, Copy, Default)]
pub struct SynthesisStats {
    pub commutative_applied: usize,
    pub stateful_applied: usize,
    pub evm_applied: usize,
    pub intents_matched: usize,
    /// Physical attestations ingested since the last snapshot.
    pub physical_attestations_ingested: usize,
    /// Physical-state intents matched since the last snapshot.
    pub physical_intents_matched: usize,
    pub avg_latency_ms: f64,
    pub throughput_per_sec: f64,
    pub network_ms: f64,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct RecentShift {
    pub hash: String,
    pub kind: String,
    pub status: String,
    pub domain: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub amount: Option<String>,
    pub token: Option<String>,
    pub timestamp_ns: u64,
}

pub struct ApiState {
    pub oscillator: Arc<Oscillator>,
    pub registry: Arc<RwLock<HashMap<AccountId, VerifyingKey>>>,
    /// Maps a derived token account back to its owner main account.
    pub derived_to_main: Arc<RwLock<HashMap<AccountId, AccountId>>>,
    pub pool_keypair: KeyPair,
    pub pool_wave_account: AccountId,
    pub pool_usdc_account: AccountId,
    pub ws_tx: broadcast::Sender<StateSnapshot>,
    pub stats: Arc<Mutex<SynthesisStats>>,
    /// Optional outbound gossip channel for broadcasting registrations.
    pub gossip: Arc<Mutex<Option<mpsc::Sender<Signal>>>>,
    /// Optional local operator keypair exposed via the operator API.
    pub operator_keypair: Mutex<Option<KeyPair>>,
    /// Recently submitted shifts for the explorer.
    pub recent_shifts: Arc<Mutex<Vec<RecentShift>>>,
    /// Signed peer endpoint directory used for decentralized discovery.
    pub peer_directory: PeerDirectory,
}

impl ApiState {
    pub fn new(oscillator: Arc<Oscillator>) -> Self {
        // Use a deterministic pool keypair across all nodes so every mesh member
        // shares the same DEX reserves and account roots.
        let pool_keypair = KeyPair::from_seed(&[0u8; 32]);
        let pool_account = pool_keypair.account_id();
        let pool_wave_account = derive_account(pool_account, b"WAVE");
        let pool_usdc_account = derive_account(pool_account, b"USDC");
        let (ws_tx, _ws_rx) = broadcast::channel(64);

        let state = Self {
            oscillator,
            registry: Arc::new(RwLock::new(HashMap::new())),
            derived_to_main: Arc::new(RwLock::new(HashMap::new())),
            pool_keypair,
            pool_wave_account,
            pool_usdc_account,
            ws_tx,
            stats: Arc::new(Mutex::new(SynthesisStats::default())),
            gossip: Arc::new(Mutex::new(None)),
            operator_keypair: Mutex::new(None),
            recent_shifts: Arc::new(Mutex::new(Vec::with_capacity(200))),
            peer_directory: PeerDirectory::new(),
        };

        // Seed the DEX pool.
        state.oscillator.seed_account(pool_wave_account, 100_000_000_000_000_000); // 100k WAVE
        state.oscillator.seed_account(pool_usdc_account, 100_000_000_000_000_000); // 100k USDC
        // Both pool reserves are protocol liquidity, not circulating user balances,
        // so they must be exempt from metabolic decay. Otherwise the WAVE reserve
        // shrinks over time and the pool price drifts to infinity.
        state.oscillator.mark_non_decaying(pool_wave_account);
        state.oscillator.mark_non_decaying(pool_usdc_account);

        // Register pool accounts so their signed shifts verify in the DAG.
        state.register_key(pool_wave_account, state.pool_keypair.public_key());
        state.register_key(pool_usdc_account, state.pool_keypair.public_key());

        state
    }

    pub fn set_gossip(&self, sender: mpsc::Sender<Signal>) {
        *self.gossip.lock().unwrap() = Some(sender);
    }

    pub fn set_operator_keypair(&self, keypair: KeyPair) {
        *self.operator_keypair.lock().unwrap() = Some(keypair);
    }

    pub fn broadcast_stake(&self, stake: StakeShift) {
        if let Some(sender) = self.gossip.lock().unwrap().as_ref() {
            let sender = sender.clone();
            let signal = Signal::Stake(stake);
            tokio::spawn(async move {
                if let Err(e) = sender.send(signal).await {
                    tracing::warn!("failed to broadcast stake: {}", e);
                }
            });
        }
    }

    pub fn broadcast_registration(&self, reg: RegistrationShift) {
        if let Some(sender) = self.gossip.lock().unwrap().as_ref() {
            let sender = sender.clone();
            let signal = Signal::Registration(reg);
            tokio::spawn(async move {
                if let Err(e) = sender.send(signal).await {
                    tracing::warn!("failed to broadcast registration: {}", e);
                }
            });
        }
    }

    pub fn broadcast_agent_registration(&self, reg: AgentRegistrationShift) {
        if let Some(sender) = self.gossip.lock().unwrap().as_ref() {
            let sender = sender.clone();
            let signal = Signal::AgentRegistration(reg);
            tokio::spawn(async move {
                if let Err(e) = sender.send(signal).await {
                    tracing::warn!("failed to broadcast agent registration: {}", e);
                }
            });
        }
    }

    pub fn broadcast_intent(&self, intent: IntentShift) {
        if let Some(sender) = self.gossip.lock().unwrap().as_ref() {
            let sender = sender.clone();
            let signal = Signal::Intent(intent);
            tokio::spawn(async move {
                if let Err(e) = sender.send(signal).await {
                    tracing::warn!("failed to broadcast intent: {}", e);
                }
            });
        }
    }

    pub fn broadcast_intent_fill(&self, fill: IntentFillShift) {
        if let Some(sender) = self.gossip.lock().unwrap().as_ref() {
            let sender = sender.clone();
            let signal = Signal::IntentFill(fill);
            tokio::spawn(async move {
                if let Err(e) = sender.send(signal).await {
                    tracing::warn!("failed to broadcast intent fill: {}", e);
                }
            });
        }
    }

    /// Broadcast a physical-state attestation to mesh peers.
    pub fn broadcast_physical_attestation(&self, attestation: PhysicalAttestation) {
        if let Some(sender) = self.gossip.lock().unwrap().as_ref() {
            let sender = sender.clone();
            let signal = Signal::PhysicalAttestation(attestation);
            tokio::spawn(async move {
                if let Err(e) = sender.send(signal).await {
                    tracing::warn!("failed to broadcast physical attestation: {}", e);
                }
            });
        }
    }

    /// Broadcast a Causal Agent Entanglement creation to mesh peers.
    pub fn broadcast_entanglement_create(&self, shift: EntanglementCreateShift) {
        if let Some(sender) = self.gossip.lock().unwrap().as_ref() {
            let sender = sender.clone();
            let signal = Signal::EntanglementCreate(shift);
            tokio::spawn(async move {
                if let Err(e) = sender.send(signal).await {
                    tracing::warn!("failed to broadcast entanglement create: {}", e);
                }
            });
        }
    }

    /// Broadcast a Causal Agent Entanglement witness attestation to mesh peers.
    pub fn broadcast_entanglement_attest(&self, shift: EntanglementAttestShift) {
        if let Some(sender) = self.gossip.lock().unwrap().as_ref() {
            let sender = sender.clone();
            let signal = Signal::EntanglementAttest(shift);
            tokio::spawn(async move {
                if let Err(e) = sender.send(signal).await {
                    tracing::warn!("failed to broadcast entanglement attestation: {}", e);
                }
            });
        }
    }

    /// Broadcast a Causal Agent Entanglement break to mesh peers.
    pub fn broadcast_entanglement_break(&self, shift: EntanglementBreakShift) {
        if let Some(sender) = self.gossip.lock().unwrap().as_ref() {
            let sender = sender.clone();
            let signal = Signal::EntanglementBreak(shift);
            tokio::spawn(async move {
                if let Err(e) = sender.send(signal).await {
                    tracing::warn!("failed to broadcast entanglement break: {}", e);
                }
            });
        }
    }

    /// Broadcast a stateful shift to mesh peers so other full nodes can apply
    /// the same transaction and converge on identical synthesis roots.
    pub fn broadcast_stateful_shift(&self, shift: StatefulShift) {
        if let Some(sender) = self.gossip.lock().unwrap().as_ref() {
            let sender = sender.clone();
            let signal = Signal::Stateful(shift);
            tokio::spawn(async move {
                if let Err(e) = sender.send(signal).await {
                    tracing::warn!("failed to broadcast stateful shift: {}", e);
                }
            });
        }
    }

    /// Broadcast a commutative shift to mesh peers so other full nodes can
    /// apply the same delta and converge on identical synthesis roots.
    pub fn broadcast_commutative_shift(&self, shift: CommutativeShift) {
        if let Some(sender) = self.gossip.lock().unwrap().as_ref() {
            let sender = sender.clone();
            let signal = Signal::Commutative(shift);
            tokio::spawn(async move {
                if let Err(e) = sender.send(signal).await {
                    tracing::warn!("failed to broadcast commutative shift: {}", e);
                }
            });
        }
    }

    pub fn register_key(&self, account: AccountId, key: VerifyingKey) {
        self.registry.write().unwrap().insert(account, key);
    }

    pub fn register_derived(&self, derived: AccountId, main: AccountId) {
        self.derived_to_main.write().unwrap().insert(derived, main);
    }

    pub fn main_account(&self, derived: AccountId) -> Option<AccountId> {
        self.derived_to_main.read().unwrap().get(&derived).copied()
    }

    pub fn key_registry(&self) -> HashMap<AccountId, VerifyingKey> {
        self.registry.read().unwrap().clone()
    }

    pub fn record_synthesis(&self, result: &SynthesisResult) {
        let mut stats = self.stats.lock().unwrap();
        stats.commutative_applied += result.commutative_applied;
        stats.stateful_applied += result.stateful_applied;
        stats.evm_applied += result.evm_applied;
        stats.intents_matched += result.intents_matched;
        stats.physical_attestations_ingested += result.physical_attestations_ingested;
        stats.physical_intents_matched += result.physical_intents_matched;
        stats.avg_latency_ms = result.avg_latency_ms;
        stats.throughput_per_sec = result.throughput_per_sec;
    }

    pub fn record_shift(&self, shift: RecentShift) {
        let mut buf = self.recent_shifts.lock().unwrap();
        buf.insert(0, shift);
        if buf.len() > 200 {
            buf.truncate(200);
        }
    }

    /// Record a gossiped shift in the explorer's recent-shifts buffer so nodes
    /// that did not submit the shift themselves still display network activity.
    pub fn record_signal(&self,
        signal: &Signal,
        status: &str,
    ) {
        match signal {
            Signal::Stateful(shift) => {
                let is_wave_to_pool = shift.to == self.pool_wave_account;
                let is_usdc_to_pool = shift.to == self.pool_usdc_account;
                let is_wave_from_pool = shift.from == self.pool_wave_account;
                let is_usdc_from_pool = shift.from == self.pool_usdc_account;
                let token = if is_wave_to_pool || is_wave_from_pool {
                    "WAVE"
                } else if is_usdc_to_pool || is_usdc_from_pool {
                    "USDC"
                } else {
                    "units"
                };
                self.record_shift(RecentShift {
                    hash: hex::encode(shift.hash()),
                    kind: "stateful".to_string(),
                    status: status.to_string(),
                    domain: Some(hex::encode(shift.domain)),
                    from: Some(shift.from.to_string()),
                    to: Some(shift.to.to_string()),
                    amount: Some(shift.amount.to_string()),
                    token: Some(token.to_string()),
                    timestamp_ns: shift.timestamp_ns,
                });
            }
            Signal::Commutative(shift) => {
                self.record_shift(RecentShift {
                    hash: hex::encode(shift.hash()),
                    kind: "commutative".to_string(),
                    status: status.to_string(),
                    domain: Some(hex::encode(shift.domain)),
                    from: Some(shift.from.to_string()),
                    to: None,
                    amount: Some(shift.delta.to_string()),
                    token: Some("units".to_string()),
                    timestamp_ns: shift.timestamp_ns,
                });
            }
            _ => {}
        }
    }

    pub fn token_accounts(&self, user_account: AccountId) -> (AccountId, AccountId) {
        (derive_account(user_account, b"WAVE"), derive_account(user_account, b"USDC"))
    }

    /// Fixed-point price used only for display.  Consensus-critical payouts
    /// should use `wave_to_usdc_out` / `usdc_to_wave_out` to avoid `f64`
    /// precision loss.
    pub fn pool_price(&self) -> f64 {
        let field = self.oscillator.wave_field.lock().unwrap();
        let wave = field.account_balance(self.pool_wave_account).units;
        let usdc = field.account_balance(self.pool_usdc_account).units;
        drop(field);
        if wave == 0 {
            return 0.0;
        }
        usdc as f64 / wave as f64
    }

    /// Integer payout for swapping `wave_in` into the pool, returning USDC.
    pub fn wave_to_usdc_out(&self, wave_in: u128) -> u128 {
        let field = self.oscillator.wave_field.lock().unwrap();
        let wave = field.account_balance(self.pool_wave_account).units;
        let usdc = field.account_balance(self.pool_usdc_account).units;
        drop(field);
        if wave == 0 {
            return 0;
        }
        wave_in.saturating_mul(usdc) / wave
    }

    /// Integer payout for swapping `usdc_in` into the pool, returning WAVE.
    pub fn usdc_to_wave_out(&self, usdc_in: u128) -> u128 {
        let field = self.oscillator.wave_field.lock().unwrap();
        let wave = field.account_balance(self.pool_wave_account).units;
        let usdc = field.account_balance(self.pool_usdc_account).units;
        drop(field);
        if usdc == 0 {
            return 0;
        }
        usdc_in.saturating_mul(wave) / usdc
    }

    pub fn shift_status(&self, hash: &[u8; 32]) -> Option<ShiftStatus> {
        self.oscillator.dag.lock().unwrap().shift_status(hash)
    }

    pub fn snapshot(&self) -> StateSnapshot {
        let (wave_reserve, usdc_reserve) = {
            let field = self.oscillator.wave_field.lock().unwrap();
            (
                field.account_balance(self.pool_wave_account).units,
                field.account_balance(self.pool_usdc_account).units,
            )
        };

        let price = if wave_reserve == 0 {
            0.0
        } else {
            usdc_reserve as f64 / wave_reserve as f64
        };

        let stats = *self.stats.lock().unwrap();

        StateSnapshot {
            wave_reserve,
            usdc_reserve,
            price,
            throughput: stats.throughput_per_sec,
            latency_ms: stats.avg_latency_ms,
            network_ms: 0.0, // populated by the WebSocket task once gossip probes exist
            metabolic_burned: *self.oscillator.metabolic_engine.total_burned.lock().unwrap(),
            commutative_applied: stats.commutative_applied,
            stateful_applied: stats.stateful_applied,
            evm_applied: stats.evm_applied,
            intents_matched: stats.intents_matched,
            physical_attestations_ingested: stats.physical_attestations_ingested,
            physical_intents_matched: stats.physical_intents_matched,
            accounts: HashMap::new(),
        }
    }

    pub fn record_network_latency_ms(&self, ms: f64) {
        let mut stats = self.stats.lock().unwrap();
        // Simple exponential moving average with alpha = 0.2.
        if stats.network_ms == 0.0 {
            stats.network_ms = ms;
        } else {
            stats.network_ms = stats.network_ms * 0.8 + ms * 0.2;
        }
    }

    pub fn load_peer_directory(&mut self, path: &std::path::Path) {
        self.peer_directory = PeerDirectory::load(path);
    }
}

pub fn build_pool_payout_shift(
    pool_keypair: &KeyPair,
    from: AccountId,
    to: AccountId,
    amount: u128,
    nonce: u64,
    tip: &VectorClock,
) -> StatefulShift {
    let mut vc = tip.clone();
    vc.tick(from.0);
    let mut shift = StatefulShift {
        domain: crate::crypto::DEFAULT_DEX_DOMAIN,
        from,
        to,
        amount,
        vector_clock: vc,
        predecessors: vec![],
        nonce,
        timestamp_ns: 0,
        first_seen_at_ns: 0,
        signature: vec![],
    };
    let sig = pool_keypair.sign(&shift.signing_bytes());
    shift.signature = sig.to_bytes().to_vec();
    shift
}
