use crate::consensus::certificate::{
    CertificateTracker, SlashingReason, SynthesisCertificate, balances_root, commutative_root,
    evm_root, stateful_root,
};
use crate::consensus::dag::{DagError, RejectionProof, RejectionReason, ShiftStatus, VectorClockDag};
use crate::consensus::domain::{DomainRegistry, StatefulOrdering};
use crate::crypto::{
    AccountId, AgentRegistrationShift, CommutativeShift, DomainId, IntentConstraint, IntentFillShift,
    IntentShift, KeyPair, PoolId, RegistrationShift, Signal, StakeShift, StatefulShift, TxHash,
    VectorClock, DEFAULT_DEX_DOMAIN,
};
use crate::evm::EvmPool;
use crate::field::coordinates::Coordinate;
use crate::field::wave_field::{AccountType, WaveField};
use crate::operator::{RewardPool, StakeTable, StakingConfig};
use crate::value::metabolic::MetabolicDecayEngine;
use crate::value::SupplyTracker;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tracing::warn;

fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

/// The result of applying a batch of phase-shifts.
#[derive(Clone, Debug, Default)]
pub struct SynthesisResult {
    pub commutative_applied: usize,
    pub stateful_applied: usize,
    pub evm_applied: usize,
    pub stateful_rejected: Vec<DagError>,
    pub final_balances: HashMap<AccountId, u128>,
    pub metabolic_burned: u128,
    /// Number of intents matched and settled this synthesis tick.
    pub intents_matched: usize,
    /// Average latency (ms) from first seen to finalized for stateful + EVM shifts.
    pub avg_latency_ms: f64,
    /// Shifts processed per second during this synthesis cycle.
    pub throughput_per_sec: f64,
    /// Wall-clock duration of this synthesis cycle (ms).
    pub elapsed_ms: f64,
}

/// An oscillator node ingests phase-shifts, validates them, and synthesizes
/// the global wave-field. Commutative shifts are aggregated with NTT; stateful
/// shifts are ordered by the vector-clock DAG.
pub struct Oscillator {
    pub id: [u8; 32],
    pub wave_field: Arc<Mutex<WaveField>>,
    pub dag: Arc<Mutex<VectorClockDag>>,
    pub keypair: KeyPair,
    pub vector_clock: Arc<Mutex<VectorClock>>,
    /// Pending commutative shifts waiting for the next NTT synthesis window.
    /// Each entry carries its target domain so per-domain policy is respected,
    /// and the signed shift is kept so signatures can be verified before the
    /// batch is applied.
    pub pending_commutative: Arc<Mutex<Vec<CommutativeShift>>>,
    /// Pending stateful shifts awaiting DAG insertion during synthesis.
    pub pending_stateful: Arc<Mutex<Vec<StatefulShift>>>,
    pub seen_signatures: DashMap<Vec<u8>, ()>,
    /// Highest observed nonce per (account, domain) for replay protection.
    /// Global signals (registration, stake) use a zeroed DomainId sentinel.
    pub seen_nonces: DashMap<(AccountId, DomainId), u64>,
    pub metabolic_engine: Arc<MetabolicDecayEngine>,
    /// Monotonically increasing synthesis tick counter.
    pub synthesis_tick: AtomicU64,
    /// Known concurrency domains and their policies.
    pub domain_registry: Arc<RwLock<DomainRegistry>>,
    /// Pending intents awaiting matching and execution during synthesis.
    pub pending_intents: Arc<Mutex<Vec<IntentShift>>>,
    /// Pending intent fills submitted by solvers.
    pub pending_intent_fills: Arc<Mutex<Vec<IntentFillShift>>>,
    /// Optional operator keypair used to sign synthesis certificates.
    pub operator_keypair: Option<KeyPair>,
    /// Signed synthesis certificates indexed by tick.
    pub certificates: Arc<RwLock<HashMap<u64, SynthesisCertificate>>>,
    /// Operator stake table controlling certificate eligibility.
    pub stake_table: Arc<StakeTable>,
    /// Tracks observed peer certificates and detects equivocation.
    pub certificate_tracker: Arc<CertificateTracker>,
    /// Accrued operator rewards from metabolic burn.
    pub reward_pool: Arc<RwLock<RewardPool>>,
    /// EVM transaction pool.
    pub evm_pool: Arc<Mutex<EvmPool>>,
    /// Tracks circulating and burned WAVE supply.
    pub supply_tracker: Arc<SupplyTracker>,
}

impl Oscillator {
    pub fn new(id: [u8; 32], ntt_size: usize) -> Self {
        Self::new_with_keypair(id, ntt_size, KeyPair::generate())
    }

    pub fn new_with_keypair(id: [u8; 32], ntt_size: usize, keypair: KeyPair) -> Self {
        Self {
            id,
            wave_field: Arc::new(Mutex::new(WaveField::new(ntt_size))),
            dag: Arc::new(Mutex::new(VectorClockDag::new())),
            keypair,
            vector_clock: Arc::new(Mutex::new(VectorClock::new())),
            pending_commutative: Arc::new(Mutex::new(Vec::new())),
            pending_stateful: Arc::new(Mutex::new(Vec::new())),
            pending_intents: Arc::new(Mutex::new(Vec::new())),
            pending_intent_fills: Arc::new(Mutex::new(Vec::new())),
            seen_signatures: DashMap::new(),
            seen_nonces: DashMap::new(),
            metabolic_engine: Arc::new(MetabolicDecayEngine::new()),
            synthesis_tick: AtomicU64::new(0),
            domain_registry: Arc::new(RwLock::new(DomainRegistry::new())),
            operator_keypair: None,
            certificates: Arc::new(RwLock::new(HashMap::new())),
            stake_table: Arc::new(StakeTable::new(StakingConfig::default())),
            certificate_tracker: Arc::new(CertificateTracker::new()),
            reward_pool: Arc::new(RwLock::new(RewardPool::new())),
            evm_pool: Arc::new(Mutex::new(EvmPool::new())),
            supply_tracker: Arc::new(SupplyTracker::new()),
        }
    }

    pub fn new_with_stake(
        id: [u8; 32],
        ntt_size: usize,
        keypair: KeyPair,
        stake_table: StakeTable,
    ) -> Self {
        let mut osc = Self::new_with_keypair(id, ntt_size, keypair.clone());
        osc.operator_keypair = Some(keypair);
        osc.stake_table = Arc::new(stake_table);
        osc
    }

    pub fn set_operator_keypair(&mut self, keypair: KeyPair) {
        self.operator_keypair = Some(keypair);
    }

    /// Check and record a sender-domain nonce for replay protection.
    fn check_and_record_nonce(&self, account: AccountId, domain: DomainId, nonce: u64) -> Result<(), String> {
        let key = (account, domain);
        match self.seen_nonces.entry(key) {
            dashmap::mapref::entry::Entry::Occupied(mut entry) => {
                if nonce <= *entry.get() {
                    return Err(format!(
                        "replay: nonce {} is not greater than highest seen {} for account {} in domain {}",
                        nonce, entry.get(), account, hex::encode(domain)
                    ));
                }
                *entry.get_mut() = nonce;
            }
            dashmap::mapref::entry::Entry::Vacant(entry) => {
                entry.insert(nonce);
            }
        }
        Ok(())
    }

    /// Ingest a peer synthesis certificate.  Conflicting certificates from the
    /// same operator and tick slash the operator.
    pub fn ingest_certificate(
        &self,
        cert: SynthesisCertificate,
        key_registry: &HashMap<AccountId, ed25519_dalek::VerifyingKey>,
    ) -> Result<(), SlashingReason> {
        let stake_table = self.stake_table.clone();
        let supply_tracker = self.supply_tracker.clone();
        let stake_checker = |op: &AccountId| stake_table.is_staked(op);
        let stake_amount = |op: &AccountId| stake_table.get_stake(op);
        let mut slash = |op: AccountId| {
            let (_, burned) = stake_table.slash(op);
            supply_tracker.burn(burned);
        };
        self.certificate_tracker
            .apply(cert, key_registry, &stake_checker, &stake_amount, &mut slash)
    }

    /// Check whether a stake-weighted quorum of certificates exists for `tick`.
    pub fn check_quorum(&self,
        tick: u64,
    ) -> Option<(crate::consensus::certificate::QuorumView, u128)> {
        let threshold = self.stake_table.quorum_threshold();
        self.certificate_tracker.check_quorum(tick, threshold)
    }

    /// Sign and store a verifiable rejection proof when an operator keypair is
    /// configured.  Returns the proof so callers can also report it immediately.
    fn sign_rejection_proof(
        &self,
        dag: &mut VectorClockDag,
        shift_hash: TxHash,
        reason: RejectionReason,
        tick: u64,
    ) {
        let Some(operator) = self.operator_keypair.as_ref() else {
            return;
        };
        let mut proof = RejectionProof {
            shift_hash,
            reason,
            rejected_at_tick: tick,
            operator_id: operator.account_id(),
            signature: Vec::new(),
        };
        let sig = operator.sign(&proof.signing_bytes());
        proof.signature = sig.to_bytes().to_vec();
        dag.rejection_proofs.insert(shift_hash, proof);
    }

    pub fn seed_account(&self, account: AccountId, amount: u128) {
        // Enforce the fixed 1B WAVE supply cap at the point of minting.
        if !self.supply_tracker.mint(amount) {
            tracing::warn!(
                "seed_account for {} rejected: would exceed {} WAVE supply cap",
                account,
                crate::value::supply::TOTAL_WAVE_SUPPLY / crate::field::wave_field::WAVE_PRECISION
            );
            return;
        }
        // Always acquire dag before wave_field to keep a consistent lock order
        // with synthesis (which locks dag then wave_field).
        let mut dag = self.dag.lock().unwrap();
        dag.seed_balance(account, amount);
        drop(dag);
        let field = self.wave_field.lock().unwrap();
        field.credit_account(account, amount);
    }

    /// Mark an account as holding non-WAVE value (e.g. USDC or a bridged asset)
    /// so it is exempt from metabolic decay.  Metabolic decay is WAVE's monetary
    /// policy; foreign value must hold its worth.
    pub fn mark_non_decaying(&self, account: AccountId) {
        let field = self.wave_field.lock().unwrap();
        field.set_non_decaying(account);
    }

    /// Register a new concurrency domain. The registrant pays a one-time
    /// reservation fee in WAVE, which is redistributed to operators and LPs.
    /// Returns `Ok(())` on success or an error if the policy is invalid, the
    /// domain already exists, or the registrant cannot afford the fee.
    pub fn register_domain(
        &self,
        policy: crate::consensus::domain::DomainPolicy,
        registrant: AccountId,
    ) -> Result<(), String> {
        let fee = crate::consensus::domain::domain_reservation_fee_units();

        // Validate and freeze the policy first.
        let policy = crate::consensus::domain::DomainPolicy::new(
            policy.domain,
            policy.commutative,
            policy.stateful,
            policy.ordering,
            policy.finalization_depth,
            policy.metabolic_lambda_ppm,
            policy.fee_policy,
        )?;

        {
            let registry = self.domain_registry.read().unwrap();
            if registry.contains(&policy.domain) {
                return Err(format!(
                    "domain {} is already registered",
                    hex::encode(policy.domain)
                ));
            }
        }

        // Deduct reservation fee and credit reward pool.
        {
            let field = self.wave_field.lock().unwrap();
            if field.account_balance(registrant).units < fee {
                return Err(format!(
                    "insufficient balance to reserve domain {}: need {}, have {}",
                    hex::encode(policy.domain),
                    fee,
                    field.account_balance(registrant).units
                ));
            }
            if !field.debit_account(registrant, fee) {
                return Err("failed to debit domain reservation fee".to_string());
            }
        }
        {
            let reward_pool = self.reward_pool.read().unwrap();
            reward_pool.distribute_fees(fee, &self.stake_table);
        }

        // Register the domain.
        let mut registry = self.domain_registry.write().unwrap();
        registry.register(policy);
        Ok(())
    }

    /// Ingest a single phase-shift. Deduplicates and queues for the next
    /// synthesis cycle.
    pub fn ingest(
        &self,
        shift: Signal,
        key_registry: &HashMap<AccountId, ed25519_dalek::VerifyingKey>,
    ) -> Result<(), String> {
        match shift {
            Signal::Commutative(c) => self.ingest_commutative(c, key_registry),
            Signal::Stateful(s) => self.ingest_stateful(s),
            Signal::Registration(_) => Ok(()), // registrations are applied immediately
            Signal::Stake(_) => Err("stake signals must be applied via apply_stake".to_string()),
            Signal::Ping { .. } | Signal::Pong { .. } => Ok(()), // network probes, not state
            Signal::Certificate(_) => Ok(()), // certificates are applied via ingest_certificate
            Signal::Auth { .. } => Ok(()),     // gossip-layer authentication, not state
            Signal::PeerAnnounce(_) => Ok(()), // peer discovery is handled by the gossip layer
            Signal::AgentRegistration(reg) => {
                if self.apply_agent_registration(&reg, key_registry) {
                    Ok(())
                } else {
                    Err("agent registration rejected".to_string())
                }
            }
            Signal::Intent(intent) => self.ingest_intent(intent, key_registry),
            Signal::IntentFill(fill) => self.ingest_intent_fill(fill, key_registry),
            Signal::Encrypted(_) => {
                Err("encrypted signals must be decrypted before ingestion".to_string())
            }
        }
    }

    /// Apply a stake event.  Verifies the operator signature and updates the
    /// local stake table.  In the current testnet implementation the signed
    /// stake announcement is trusted; nodes that have not yet synced the
    /// operator's on-chain balance still learn the stake so they can verify
    /// synthesis certificates from peers that join before them.
    pub fn apply_stake(&self, stake: &StakeShift) -> bool {
        if !stake.verify() {
            tracing::warn!("stake rejected for {}: invalid signature", stake.operator);
            return false;
        }
        if let Err(e) = self.check_and_record_nonce(stake.operator, DomainId::default(), stake.nonce) {
            tracing::warn!("stake rejected for {}: {}", stake.operator, e);
            return false;
        }

        let previous_locked = self.stake_table.get_stake(&stake.operator);
        if stake.amount == previous_locked {
            return true;
        }

        // Lock order: dag then wave_field, consistent with synthesis.
        let mut dag = self.dag.lock().unwrap();
        let field = self.wave_field.lock().unwrap();

        if stake.amount > previous_locked {
            let additional = stake.amount - previous_locked;
            if field.account_balance(stake.operator).units < additional {
                tracing::warn!(
                    "stake rejected for {}: insufficient liquid balance (need {}, have {})",
                    stake.operator,
                    additional,
                    field.account_balance(stake.operator).units
                );
                return false;
            }
            if !field.debit_account(stake.operator, additional) {
                return false;
            }
            *dag.balances.entry(stake.operator).or_insert(0) = dag
                .balances
                .get(&stake.operator)
                .copied()
                .unwrap_or(0)
                .saturating_sub(additional);
        } else {
            let refund = previous_locked - stake.amount;
            field.credit_account(stake.operator, refund);
            *dag.balances.entry(stake.operator).or_insert(0) += refund;
        }

        drop(field);
        drop(dag);

        self.stake_table.stake(stake.operator, stake.amount);
        true
    }

    /// Apply an agent-registration event.  Verifies the owner signature, checks
    /// replay nonce, and marks the agent account in the wave-field.
    pub fn apply_agent_registration(
        &self,
        reg: &AgentRegistrationShift,
        key_registry: &HashMap<AccountId, ed25519_dalek::VerifyingKey>,
    ) -> bool {
        let Some(owner_pk) = key_registry.get(&reg.owner) else {
            tracing::warn!("agent registration rejected: unknown owner {}", reg.owner);
            return false;
        };
        if !reg.verify(owner_pk) {
            tracing::warn!("agent registration rejected: invalid owner signature");
            return false;
        }
        if let Err(e) = self.check_and_record_nonce(reg.owner, DomainId::default(), reg.nonce) {
            tracing::warn!("agent registration rejected: {}", e);
            return false;
        }
        let field = self.wave_field.lock().unwrap();
        field.set_account_type(
            reg.agent,
            AccountType::Agent {
                owner: reg.owner,
                expiry_tick: reg.expiry_tick,
            },
        );
        true
    }

    /// Validate and queue an intent.
    pub fn ingest_intent(
        &self,
        intent: IntentShift,
        key_registry: &HashMap<AccountId, ed25519_dalek::VerifyingKey>,
    ) -> Result<(), String> {
        let Some(owner_pk) = key_registry.get(&intent.owner) else {
            return Err(format!("intent rejected: unknown owner {}", intent.owner));
        };
        if !intent.verify(owner_pk) {
            return Err("intent rejected: invalid owner signature".to_string());
        }
        if let Err(e) = self.check_and_record_nonce(intent.owner, intent.domain, intent.nonce) {
            return Err(format!("intent rejected: {}", e));
        }
        let tick = self.synthesis_tick.load(Ordering::SeqCst);
        if intent.deadline_tick <= tick {
            return Err(format!(
                "intent rejected: deadline {} is not in the future (current tick {})",
                intent.deadline_tick, tick
            ));
        }
        // Ensure the owner can cover the solver reward.
        {
            let field = self.wave_field.lock().unwrap();
            let balance = field.account_balance_in_domain(intent.domain, intent.owner).units;
            if balance < intent.solver_reward {
                return Err(format!(
                    "intent rejected: insufficient balance for solver reward (need {}, have {})",
                    intent.solver_reward, balance
                ));
            }
        }
        self.pending_intents.lock().unwrap().push(intent);
        Ok(())
    }

    /// Validate and queue an intent fill from a solver.
    pub fn ingest_intent_fill(
        &self,
        fill: IntentFillShift,
        key_registry: &HashMap<AccountId, ed25519_dalek::VerifyingKey>,
    ) -> Result<(), String> {
        let Some(solver_pk) = key_registry.get(&fill.solver) else {
            return Err(format!("intent fill rejected: unknown solver {}", fill.solver));
        };
        if !fill.verify(solver_pk) {
            return Err("intent fill rejected: invalid solver signature".to_string());
        }
        if let Err(e) = self.check_and_record_nonce(fill.solver, DomainId::default(), fill.nonce) {
            return Err(format!("intent fill rejected: {}", e));
        }
        self.pending_intent_fills.lock().unwrap().push(fill);
        Ok(())
    }

    /// Compute the execution fee for a stateful signal according to the fee
    /// policy of its domain.  Returns the fee in sub-units and the post-fee
    /// transfer amount.
    pub fn compute_signal_fee(
        &self,
        shift: &StatefulShift,
    ) -> Result<(u128, u128), String> {
        use crate::consensus::domain::FeePolicy;
        let policy = self
            .domain_registry
            .read()
            .unwrap()
            .get(&shift.domain)
            .cloned()
            .ok_or_else(|| format!("unknown domain {}", hex::encode(shift.domain)))?;
        match policy.fee_policy {
            FeePolicy::Flat(fee) => {
                if shift.amount < fee {
                    return Err("transfer amount does not cover flat fee".to_string());
                }
                Ok((fee, shift.amount - fee))
            }
            FeePolicy::Percentage(bp) => {
                let fee = shift.amount.saturating_mul(bp as u128) / 10_000;
                Ok((fee, shift.amount - fee))
            }
            FeePolicy::MetabolicOnly => Ok((0, shift.amount)),
        }
    }

    /// Verify a batch of stateful shift signatures.  Verification is performed in
    /// chunks so a single bad signature does not force individual re-verification
    /// of the entire set.  Returns a vector aligned with `shifts` indicating
    /// whether each shift's signature is valid.
    fn batch_verify_stateful(
        shifts: &[StatefulShift],
        keys: &HashMap<AccountId, ed25519_dalek::VerifyingKey>,
    ) -> Vec<bool> {
        const BATCH_SIZE: usize = 64;
        let mut results = vec![false; shifts.len()];

        for (chunk_idx, chunk) in shifts.chunks(BATCH_SIZE).enumerate() {
            let mut messages: Vec<Vec<u8>> = Vec::with_capacity(chunk.len());
            let mut signatures: Vec<ed25519_dalek::Signature> = Vec::with_capacity(chunk.len());
            let mut keys_chunk: Vec<ed25519_dalek::VerifyingKey> = Vec::with_capacity(chunk.len());
            let mut valid_indices: Vec<usize> = Vec::with_capacity(chunk.len());

            for (i, shift) in chunk.iter().enumerate() {
                let idx = chunk_idx * BATCH_SIZE + i;
                let Some(pk) = keys.get(&shift.from) else {
                    results[idx] = false;
                    continue;
                };
                let Ok(sig) = ed25519_dalek::Signature::from_slice(&shift.signature) else {
                    results[idx] = false;
                    continue;
                };
                messages.push(shift.signing_bytes());
                signatures.push(sig);
                keys_chunk.push(*pk);
                valid_indices.push(idx);
            }

            if messages.is_empty() {
                continue;
            }

            let msg_refs: Vec<&[u8]> = messages.iter().map(|m| m.as_slice()).collect();

            match ed25519_dalek::verify_batch(&msg_refs, &signatures, &keys_chunk) {
                Ok(()) => {
                    for idx in valid_indices {
                        results[idx] = true;
                    }
                }
                Err(_) => {
                    // Batch failed; fall back to individual verification to identify
                    // the bad signature(s) without rejecting the whole chunk.
                    for (i, shift) in chunk.iter().enumerate() {
                        let idx = chunk_idx * BATCH_SIZE + i;
                        if let Some(pk) = keys.get(&shift.from) {
                            results[idx] = shift.verify_signature(pk);
                        }
                    }
                }
            }
        }

        results
    }

    /// Apply a registration event directly so every node learns the account.
    /// The caller must register the public key in the API registry separately.
    pub fn apply_registration(&self, reg: &RegistrationShift) {
        if let Err(e) = self.check_and_record_nonce(reg.account, DomainId::default(), reg.nonce) {
            tracing::warn!("registration rejected for {}: {}", reg.account, e);
            return;
        }

        // Keep lock order consistent with synthesis: dag first, then wave_field.
        let mut dag = self.dag.lock().unwrap();
        dag.seed_balance(reg.wave_account, 10_000_000_000_000);
        dag.seed_balance(reg.usdc_account, 10_000_000_000_000);
        drop(dag);
        let field = self.wave_field.lock().unwrap();
        field.ensure_account(reg.account);
        field.ensure_account(reg.wave_account);
        field.ensure_account(reg.usdc_account);
        // USDC is foreign value and must not metabolically decay.
        field.set_non_decaying(reg.usdc_account);
        if field.account_balance(reg.wave_account).units == 0 {
            field.credit_account(reg.wave_account, 10_000_000_000_000);
        }
        if field.account_balance(reg.usdc_account).units == 0 {
            field.credit_account(reg.usdc_account, 10_000_000_000_000);
        }
    }

    fn ingest_commutative(&self,
        mut shift: CommutativeShift,
        key_registry: &HashMap<AccountId, ed25519_dalek::VerifyingKey>,
    ) -> Result<(), String> {
        let policy = self
            .domain_registry
            .read()
            .unwrap()
            .get(&shift.domain)
            .cloned()
            .ok_or_else(|| format!("unknown domain {}", hex::encode(shift.domain)))?;
        if !policy.commutative {
            return Err(format!(
                "domain {} does not allow commutative signals",
                hex::encode(shift.domain)
            ));
        }

        // Verify the commutative shift is signed by its claimed sender.
        let Some(pk) = key_registry.get(&shift.from) else {
            return Err(format!(
                "unknown sender {} for commutative shift",
                hex::encode(shift.from.0)
            ));
        };
        if !shift.verify(pk) {
            return Err(format!(
                "invalid signature for commutative shift from {}",
                hex::encode(shift.from.0)
            ));
        }

        self.check_and_record_nonce(shift.from, shift.domain, shift.nonce)?;

        if self.seen_signatures.contains_key(&shift.signature) {
            return Ok(()); // already processed
        }
        self.seen_signatures.insert(shift.signature.clone(), ());
        if shift.first_seen_at_ns == 0 {
            shift.first_seen_at_ns = now_ns();
        }
        let mut pending = self.pending_commutative.lock().unwrap();
        pending.push(shift);
        Ok(())
    }

    fn ingest_stateful(&self, mut shift: StatefulShift) -> Result<(), String> {
        let policy = self
            .domain_registry
            .read()
            .unwrap()
            .get(&shift.domain)
            .cloned()
            .ok_or_else(|| format!("unknown domain {}", hex::encode(shift.domain)))?;
        if !policy.stateful {
            return Err(format!(
                "domain {} does not allow stateful signals",
                hex::encode(shift.domain)
            ));
        }
        self.check_and_record_nonce(shift.from, shift.domain, shift.nonce)?;
        if self.seen_signatures.contains_key(&shift.signature) {
            return Ok(());
        }
        if shift.amount == 0 {
            return Err("stateful shift with zero amount".to_string());
        }
        if shift.first_seen_at_ns == 0 {
            shift.first_seen_at_ns = now_ns();
        }
        self.seen_signatures.insert(shift.signature.clone(), ());
        let mut pending = self.pending_stateful.lock().unwrap();
        pending.push(shift);
        Ok(())
    }

    /// Match queued intent fills against open intents and settle them atomically
    /// in the wave-field.  Unmatched intents and fills are re-queued for the
    /// next synthesis tick; expired intents are dropped.
    fn process_intents(
        &self,
        tick: u64,
        result: &mut SynthesisResult,
    ) {
        let intents: Vec<IntentShift> = self.pending_intents.lock().unwrap().drain(..).collect();
        let fills: Vec<IntentFillShift> = self.pending_intent_fills.lock().unwrap().drain(..).collect();
        if intents.is_empty() && fills.is_empty() {
            return;
        }

        let mut unmatched_intents: Vec<IntentShift> = Vec::new();
        let mut unmatched_fills: Vec<IntentFillShift> = Vec::new();
        let mut matched_intent_ids = std::collections::HashSet::new();

        // Index open intents by id.
        let mut intent_by_id: HashMap<TxHash, IntentShift> = HashMap::new();
        for intent in intents {
            if tick > intent.deadline_tick {
                tracing::trace!(
                    "intent {} expired at tick {}",
                    hex::encode(intent.intent_id),
                    tick
                );
                continue;
            }
            intent_by_id.insert(intent.intent_id, intent);
        }

        let field = self.wave_field.lock().unwrap();

        for fill in fills {
            let Some(intent) = intent_by_id.get(&fill.intent_id) else {
                unmatched_fills.push(fill);
                continue;
            };
            if matched_intent_ids.contains(&intent.intent_id) {
                unmatched_fills.push(fill);
                continue;
            }

            // Owner must still be able to cover the reward.
            let owner_balance = field.account_balance_in_domain(intent.domain, intent.owner).units;
            if owner_balance < intent.solver_reward {
                unmatched_fills.push(fill);
                continue;
            }

            let settled = match &intent.constraint {
                IntentConstraint::Transfer { to, min_amount } => {
                    if fill.fill_amount < *min_amount {
                        false
                    } else if owner_balance < intent.solver_reward.saturating_add(fill.fill_amount) {
                        false
                    } else {
                        // owner -> beneficiary
                        field.debit_account_in_domain(intent.domain, intent.owner, fill.fill_amount);
                        field.credit_account_in_domain(intent.domain, *to, fill.fill_amount);
                        // owner -> solver reward
                        field.debit_account_in_domain(intent.domain, intent.owner, intent.solver_reward);
                        field.credit_account_in_domain(intent.domain, fill.solver, intent.solver_reward);
                        field.mark_active_in_domain(intent.domain, intent.owner, tick);
                        field.mark_active_in_domain(intent.domain, *to, tick);
                        field.mark_active_in_domain(intent.domain, fill.solver, tick);
                        true
                    }
                }
                IntentConstraint::Swap {
                    from_token: _,
                    to_token: _,
                    min_out,
                    max_slippage_bp: _,
                } => {
                    // Direct atomic exchange: owner gives fill_amount of the input
                    // asset to the solver, solver gives min_out of the output asset
                    // to the owner.  We debit/credit the same account for both assets
                    // because the current domain model uses one balance per account;
                    // multi-token settlement will use domain-isolated token accounts.
                    let owner_from = field.account_balance_in_domain(intent.domain, intent.owner).units;
                    let solver_to = field.account_balance_in_domain(intent.domain, fill.solver).units;
                    if owner_from < fill.fill_amount || solver_to < *min_out {
                        false
                    } else {
                        field.debit_account_in_domain(intent.domain, intent.owner, fill.fill_amount);
                        field.credit_account_in_domain(intent.domain, fill.solver, fill.fill_amount);
                        field.debit_account_in_domain(intent.domain, fill.solver, *min_out);
                        field.credit_account_in_domain(intent.domain, intent.owner, *min_out);
                        field.debit_account_in_domain(intent.domain, intent.owner, intent.solver_reward);
                        field.credit_account_in_domain(intent.domain, fill.solver, intent.solver_reward);
                        field.mark_active_in_domain(intent.domain, intent.owner, tick);
                        field.mark_active_in_domain(intent.domain, fill.solver, tick);
                        true
                    }
                }
            };

            if settled {
                matched_intent_ids.insert(intent.intent_id);
                field.adjust_reputation_in_domain(intent.domain, fill.solver, 1);
                result.intents_matched += 1;
            } else {
                unmatched_fills.push(fill);
            }
        }

        // Return unmatched intents to the pool.
        for (_, intent) in intent_by_id {
            if !matched_intent_ids.contains(&intent.intent_id) {
                unmatched_intents.push(intent);
            }
        }
        drop(field);

        if !unmatched_intents.is_empty() {
            self.pending_intents.lock().unwrap().extend(unmatched_intents);
        }
        if !unmatched_fills.is_empty() {
            self.pending_intent_fills.lock().unwrap().extend(unmatched_fills);
        }
    }

    /// Synthesize all pending commutative deltas via NTT and apply stateful
    /// transactions from the DAG in topological order.
    pub fn synthesize(
        &self,
        key_registry: &HashMap<AccountId, ed25519_dalek::VerifyingKey>,
    ) -> SynthesisResult {
        let mut result = SynthesisResult::default();
        let start = Instant::now();
        let finalized_at = now_ns();

        // Increment monotonic synthesis tick at the start of each cycle.
        let tick = self.synthesis_tick.fetch_add(1, Ordering::SeqCst);

        // 0. Metabolic decay: exponentially decay every wave-field balance by
        //    B(t) = B(0) * e^(-λt), using the DEX domain's λ.  Staked operators
        //    are immune (their locked balances back the network).  Of the value
        //    that decays away, a fixed fraction (`METABOLIC_BURN_BP`) is
        //    permanently burned and the remainder is redistributed to operators
        //    and liquidity providers below.
        let immune_accounts: std::collections::HashSet<AccountId> = self
            .stake_table
            .staked_operators()
            .into_iter()
            .map(|(operator, _)| operator)
            .collect();
        let dex_lambda = self
            .domain_registry
            .read()
            .unwrap()
            .get(&crate::crypto::DEFAULT_DEX_DOMAIN)
            .map(|p| p.metabolic_lambda_ppm)
            .unwrap_or(crate::value::metabolic::DEFAULT_DEX_LAMBDA_PPM);
        let decayed = {
            let mut field = self.wave_field.lock().unwrap();
            field.apply_metabolic_decay(tick, dex_lambda, &immune_accounts)
        };
        result.metabolic_burned = decayed;
        // Record the total decayed value into the engine's running total for
        // reporting surfaces (API / persistence).
        self.metabolic_engine.record_burn(decayed);

        // Deterministic integer split: burn a fixed fraction, redistribute the
        // rest.  The remainder (and any rounding) always goes to rewards so no
        // value is lost and every honest node computes the same partition.
        let burn_share = decayed
            .saturating_mul(crate::value::metabolic::METABOLIC_BURN_BP as u128)
            / crate::value::metabolic::BASIS_POINTS_DENOMINATOR as u128;
        let reward_share = decayed - burn_share;
        if burn_share > 0 {
            self.supply_tracker.burn(burn_share);
        }
        if reward_share > 0 {
            let reward_pool = self.reward_pool.read().unwrap();
            reward_pool.distribute(reward_share, &self.stake_table);
        }

        // 0b. Sync decayed wave-field balances into the DAG so that stateful
        //     simulation and double-spend detection operate on the true,
        //     metabolically-decayed available balances.
        //     Lock order: dag first, then wave_field (consistent with the rest
        //     of the oscillator and persistence::save).
        {
            let mut dag = self.dag.lock().unwrap();
            let field = self.wave_field.lock().unwrap();
            for domain_entry in field.domains.iter() {
                for entry in domain_entry.value().accounts.iter() {
                    dag.balances
                        .insert(*entry.key(), entry.value().balance.units);
                }
            }
        }

        // 1. Move pending stateful shifts into the DAG.
        let mut finalized_latency_ms = 0.0f64;
        let mut finalized_count = 0usize;
        {
            let mut pending = self.pending_stateful.lock().unwrap();
            let shifts: Vec<StatefulShift> = pending.drain(..).collect();
            drop(pending);

            let mut dag = self.dag.lock().unwrap();
            for shift in shifts {
                let hash = shift.hash();
                let Some(pk) = key_registry.get(&shift.from) else {
                    let err = DagError::InvalidSignature(hash);
                    let reason = RejectionReason::from(&err);
                    dag.rejected.insert(hash, err.clone());
                    self.sign_rejection_proof(&mut dag, hash, reason, tick);
                    result.stateful_rejected.push(err);
                    continue;
                };
                let depth = self
                    .domain_registry
                    .read()
                    .unwrap()
                    .get(&shift.domain)
                    .map(|p| p.finalization_depth)
                    .unwrap_or(VectorClockDag::FINALIZATION_DEPTH);
                if let Err(e) = dag.insert(shift, pk, tick, depth) {
                    let reason = RejectionReason::from(&e);
                    self.sign_rejection_proof(&mut dag, e.shift_hash(), reason, tick);
                    result.stateful_rejected.push(e);
                }
            }

            // Detect and mark double-spend attempts.
            let double_spends = dag.detect_double_spends();
            for err in &double_spends {
                if let DagError::DoubleSpend(hash) = err {
                    if let Some(node) = dag.nodes.get_mut(hash) {
                        node.status = ShiftStatus::Rejected(DagError::DoubleSpend(*hash));
                    }
                    self.sign_rejection_proof(
                        &mut dag,
                        *hash,
                        RejectionReason::DoubleSpend,
                        tick,
                    );
                }
            }
            result.stateful_rejected.extend(double_spends);

            // Promote accepted shifts to finalized after K subsequent ticks.
            let (promoted, promoted_latency_ms) = dag.promote_to_finalized(tick, finalized_at);
            finalized_count += promoted;
            finalized_latency_ms += promoted_latency_ms;
        }

        // 2. Synthesize commutative batches, one per registered domain that allows
        //    commutative signals. This respects per-domain policy instead of
        //    aggregating all commutative deltas into a single global batch.
        let mut comm_root = [0u8; 32];
        {
            let mut pending = self.pending_commutative.lock().unwrap();
            if !pending.is_empty() {
                let shifts: Vec<CommutativeShift> = pending.drain(..).collect();
                drop(pending);

                let registry = self.domain_registry.read().unwrap();
                let mut by_domain: std::collections::HashMap<DomainId, (Vec<(Coordinate, i128, PoolId)>, Vec<CommutativeShift>)> =
                    std::collections::HashMap::new();
                for shift in shifts {
                    match registry.get(&shift.domain) {
                        Some(policy) if policy.commutative => {
                            let entry = by_domain.entry(shift.domain).or_default();
                            entry.0.push((shift.coordinate, shift.delta, shift.pool_id));
                            entry.1.push(shift);
                        }
                        Some(_) => {
                            tracing::warn!(
                                "commutative shift for non-commutative domain {} dropped",
                                hex::encode(shift.domain)
                            );
                        }
                        None => {
                            tracing::warn!(
                                "commutative shift for unknown domain {} dropped",
                                hex::encode(shift.domain)
                            );
                        }
                    }
                }
                drop(registry);

                let mut field = self.wave_field.lock().unwrap();
                let mut total_applied = 0usize;
                for (domain, (domain_deltas, mut original_shifts)) in by_domain {
                    if domain_deltas.is_empty() {
                        continue;
                    }
                    comm_root = commutative_root(tick, &domain_deltas);
                    if let Err(e) = field.synthesize_commutative_batch(&domain_deltas) {
                        warn!("commutative synthesis failed for domain {}: {}", hex::encode(domain), e);
                        // Re-queue only the failed domain's original shifts so others are not lost.
                        let mut pending = self.pending_commutative.lock().unwrap();
                        pending.append(&mut original_shifts);
                        comm_root = [0u8; 32];
                    } else {
                        total_applied += domain_deltas.len();
                    }
                }
                result.commutative_applied = total_applied;
            }
        }

        // 3. Apply stateful DAG in topological order.
        let mut dag = self.dag.lock().unwrap();
        let order = match dag.topological_order() {
            Ok(o) => o,
            Err(e) => {
                let hash = e.shift_hash();
                let reason = RejectionReason::from(&e);
                self.sign_rejection_proof(&mut dag, hash, reason, tick);
                result.stateful_rejected.push(e);
                return result;
            }
        };

        // Start from the cumulative DAG balances (already decayed) and apply only
        // shifts that have not yet been applied.  Marking applied shifts prevents
        // them from being replayed on subsequent ticks.  Fees are deducted from
        // the sender and accrue to the reward pool according to the domain's fee
        // policy.
        let mut simulated_balances = dag.balances.clone();
        let mut stateful_hashes = Vec::with_capacity(order.len());
        let mut active_accounts = std::collections::HashSet::new();
        let mut total_fees = 0u128;
        // Pre-compute fees while we hold the DAG so the fee-deduction phase does
        // not need to re-acquire the lock or re-run fee policy logic.
        let mut stateful_fees: std::collections::HashMap<TxHash, (AccountId, u128)> =
            std::collections::HashMap::new();

        // Batch-verify signatures for all candidate shifts up-front.  This is
        // significantly faster than verifying one-by-one because ed25519 batch
        // verification amortizes the scalar multiplication cost.
        let candidate_shifts: Vec<StatefulShift> = order
            .iter()
            .filter_map(|hash| {
                let node = dag.nodes.get(hash).expect("hash in DAG");
                if matches!(node.status, ShiftStatus::Rejected(_)) || node.applied {
                    None
                } else {
                    Some(node.shift.clone())
                }
            })
            .collect();
        let verification_results = Self::batch_verify_stateful(&candidate_shifts, key_registry);
        let valid_shift_set: std::collections::HashSet<TxHash> = candidate_shifts
            .iter()
            .zip(verification_results.iter())
            .filter(|(_, valid)| **valid)
            .map(|(shift, _)| shift.hash())
            .collect();

        for hash in order {
            let node = dag.nodes.get(&hash).expect("hash in DAG");

            // Skip shifts already rejected by double-spend detection or applied.
            if matches!(node.status, ShiftStatus::Rejected(_)) || node.applied {
                continue;
            }

            let shift = &node.shift;

            if !valid_shift_set.contains(&hash) {
                let err = DagError::InvalidSignature(hash);
                self.sign_rejection_proof(&mut dag, hash, RejectionReason::InvalidSignature, tick);
                result.stateful_rejected.push(err);
                continue;
            }

            let policy = match self.domain_registry.read().unwrap().get(&shift.domain) {
                Some(p) => p.clone(),
                None => {
                    // Domain was validated at ingest time; missing here is an
                    // inconsistent state. Skip rather than panic.
                    tracing::warn!(
                        "strict check: unknown domain {} for shift {}",
                        hex::encode(shift.domain),
                        hex::encode(hash)
                    );
                    continue;
                }
            };

            // Strict domains require an operator quorum certificate for the tick
            // in which the shift was inserted before it can be applied.
            if policy.ordering == StatefulOrdering::Strict {
                if self.check_quorum(node.inserted_at_tick).is_none() {
                    // Skip for now; will retry on a later tick once quorum exists.
                    continue;
                }
            }

            let (fee, net_amount) = match self.compute_signal_fee(shift) {
                Ok(v) => v,
                Err(_) => {
                    let err = DagError::InsufficientBalance(hash);
                    self.sign_rejection_proof(&mut dag, hash, RejectionReason::InsufficientBalance, tick);
                    result.stateful_rejected.push(err);
                    continue;
                }
            };

            let balance = simulated_balances.get(&shift.from).copied().unwrap_or(0);
            if balance < shift.amount {
                let err = DagError::InsufficientBalance(hash);
                self.sign_rejection_proof(&mut dag, hash, RejectionReason::InsufficientBalance, tick);
                result.stateful_rejected.push(err);
                continue;
            }

            *simulated_balances.get_mut(&shift.from).unwrap() -= shift.amount;
            *simulated_balances.entry(shift.to).or_insert(0) += net_amount;
            total_fees = total_fees.saturating_add(fee);
            result.stateful_applied += 1;
            stateful_hashes.push(hash);
            stateful_fees.insert(hash, (shift.from, fee));
            // Record both parties as active so they receive metabolic-decay
            // grace starting next tick.  Self-transfers do not count as real
            // economic activity, otherwise a whale could bypass decay for free by
            // scripting transfers to themselves.
            if shift.from != shift.to {
                active_accounts.insert(shift.from);
                active_accounts.insert(shift.to);
            }
        }
        drop(dag);

        // 3b. Apply verified EVM transactions in nonce order.
        let evm_hashes = {
            let mut evm_pool = self.evm_pool.lock().unwrap();
            let (evm_applied, evm_latency_ms, hashes) = evm_pool.synthesize(
                &mut simulated_balances, finalized_at, tick);
            result.evm_applied = evm_applied;
            finalized_count += evm_applied;
            finalized_latency_ms += evm_latency_ms;
            hashes
        };

        // Deduct accumulated signal fees from the wave-field sender balances and
        // add them to the reward pool.  Fees were pre-computed during the DAG pass
        // above, so we do not need to re-acquire the DAG lock here.
        if total_fees > 0 {
            let mut fee_debt: std::collections::HashMap<AccountId, u128> = std::collections::HashMap::new();
            for (_, (from, fee)) in &stateful_fees {
                *fee_debt.entry(*from).or_insert(0) += fee;
            }
            {
                let field = self.wave_field.lock().unwrap();
                for (account, fee) in fee_debt {
                    field.debit_account(account, fee.min(field.account_balance(account).units));
                }
            }
            {
                let reward_pool = self.reward_pool.read().unwrap();
                reward_pool.distribute_fees(total_fees, &self.stake_table);
            }
        }

        // Commit applied stateful shifts and the cumulative balance set back to
        // the DAG so future ticks do not replay already-settled shifts.
        {
            let mut dag = self.dag.lock().unwrap();
            for hash in &stateful_hashes {
                if let Some(node) = dag.nodes.get_mut(hash) {
                    node.applied = true;
                }
            }
            dag.balances = simulated_balances.clone();
        }

        // Sync wave-field account balances with DAG result.  Because fees were
        // already debited above, only transfer the net simulated balances here
        // so we do not double-charge the sender.
        {
            let field = self.wave_field.lock().unwrap();
            for (account, balance) in &simulated_balances {
                field.ensure_account(*account);
                if let Some(dex) = field.domains.get(&DEFAULT_DEX_DOMAIN) {
                    if let Some(mut state) = dex.accounts.get_mut(account) {
                        state.balance.units = *balance;
                        // Accounts touched by an applied stateful shift this tick start
                        // their activity grace window.
                        if active_accounts.contains(account) {
                            state.balance.last_active_tick = tick;
                        }
                    }
                }
            }
            result.final_balances = simulated_balances.clone();
        }

        // 3c. Match and settle intents atomically using the latest balances.
        self.process_intents(tick, &mut result);

        // 4. Optionally sign a synthesis certificate if the operator is staked.
        if let Some(ref op_kp) = self.operator_keypair {
            if !self.stake_table.is_staked(&op_kp.account_id()) {
                return result;
            }
            let state_root = stateful_root(tick, &stateful_hashes);
            let bal_root = balances_root(&simulated_balances);
            let stake_root = self.stake_table.root();
            let reward_root = self.reward_pool.read().unwrap().root();
            let evm_r = evm_root(tick, &evm_hashes);
            let timestamp_ns = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0);
            let cert = SynthesisCertificate::sign(
                op_kp,
                tick,
                result.commutative_applied,
                result.stateful_applied,
                result.evm_applied,
                comm_root,
                state_root,
                bal_root,
                stake_root,
                reward_root,
                evm_r,
                result.metabolic_burned,
                timestamp_ns,
            );
            // Count our own certificate toward local quorum detection.
            let _ = self.ingest_certificate(cert.clone(), key_registry);
            self.certificates.write().unwrap().insert(tick, cert);
        }

        // Compute real performance metrics.
        result.elapsed_ms = start.elapsed().as_nanos() as f64 / 1_000_000.0;
        let total_processed = result.commutative_applied
            + result.stateful_applied
            + result.evm_applied;
        if result.elapsed_ms > 0.0 {
            result.throughput_per_sec = (total_processed as f64) / (result.elapsed_ms / 1000.0);
        }
        if finalized_count > 0 {
            result.avg_latency_ms = finalized_latency_ms / finalized_count as f64;
        }

        result
    }

    pub fn tick_vector_clock(&self) {
        let mut vc = self.vector_clock.lock().unwrap();
        vc.tick(self.id);
    }

    pub fn current_vector_clock(&self) -> VectorClock {
        self.vector_clock.lock().unwrap().clone()
    }

    pub fn stateful_count(&self) -> usize {
        self.dag.lock().unwrap().nodes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::DEFAULT_DEX_DOMAIN;
    use crate::crypto::keys::KeyPair;

    #[test]
    fn oscillator_rejects_replayed_nonce() {
        let osc = Oscillator::new([1u8; 32], 64);
        let alice = KeyPair::generate();
        let bob = KeyPair::generate();
        osc.seed_account(alice.account_id(), 10_000_000_000_000);

        let mut vc = VectorClock::new();
        vc.tick(osc.id);
        let st1 = StatefulShift::new(
            &alice,
            DEFAULT_DEX_DOMAIN,
            bob.account_id(),
            100,
            vc.clone(),
            vec![],
            1,
            0,
        );
        let mut registry = HashMap::new();
        registry.insert(alice.account_id(), alice.public_key());
        osc.ingest(Signal::Stateful(st1), &registry).unwrap();

        // Same nonce should be rejected.
        let st2 = StatefulShift::new(
            &alice,
            DEFAULT_DEX_DOMAIN,
            bob.account_id(),
            200,
            vc.clone(),
            vec![],
            1,
            0,
        );
        assert!(osc.ingest(Signal::Stateful(st2), &registry).is_err());

        // Higher nonce is accepted.
        let st3 = StatefulShift::new(
            &alice,
            DEFAULT_DEX_DOMAIN,
            bob.account_id(),
            200,
            vc,
            vec![],
            2,
            0,
        );
        osc.ingest(Signal::Stateful(st3), &registry).unwrap();
    }

    #[test]
    fn oscillator_registers_domain_and_deducts_fee() {
        let osc = Oscillator::new([1u8; 32], 64);
        let alice = KeyPair::generate();
        let fee = crate::consensus::domain::domain_reservation_fee_units();
        osc.seed_account(alice.account_id(), fee + 1_000_000_000_000);

        let domain_id = [7u8; 32];
        let policy = crate::consensus::domain::DomainPolicy::new(
            domain_id,
            true,
            true,
            crate::consensus::domain::StatefulOrdering::Causal,
            3,
            20,
            crate::consensus::domain::FeePolicy::MetabolicOnly,
        )
        .unwrap();

        let balance_before = osc
            .wave_field
            .lock()
            .unwrap()
            .account_balance(alice.account_id())
            .units;
        osc.register_domain(policy, alice.account_id()).unwrap();
        let balance_after = osc
            .wave_field
            .lock()
            .unwrap()
            .account_balance(alice.account_id())
            .units;

        assert_eq!(balance_before - balance_after, fee);
        assert!(osc.domain_registry.read().unwrap().contains(&domain_id));
    }

    #[test]
    fn oscillator_strict_domain_waits_for_quorum() {
        let mut osc = Oscillator::new([1u8; 32], 64);
        let operator = KeyPair::generate();
        let alice = KeyPair::generate();
        let bob = KeyPair::generate();

        // Seed accounts and stake the operator so quorum certificates can be
        // produced. The default min_stake is 1_000_000_000_000_000_000.
        osc.seed_account(operator.account_id(), 2_000_000_000_000_000_000);
        osc.seed_account(alice.account_id(), 1_000_000_000_000);
        osc.set_operator_keypair(operator.clone());

        // Register a strict domain.
        let strict_domain = [42u8; 32];
        let policy = crate::consensus::domain::DomainPolicy::new(
            strict_domain,
            false,
            true,
            crate::consensus::domain::StatefulOrdering::Strict,
            3,
            20,
            crate::consensus::domain::FeePolicy::MetabolicOnly,
        )
        .unwrap();
        osc.register_domain(policy, operator.account_id()).unwrap();

        // Stake the operator before synthesis so a quorum certificate is produced
        // for the tick in which the strict shift is inserted.
        osc.stake_table.stake(operator.account_id(), 1_000_000_000_000_000_000);

        // Create a stateful shift in the strict domain.
        let mut registry = HashMap::new();
        registry.insert(alice.account_id(), alice.public_key());
        registry.insert(operator.account_id(), operator.public_key());

        let mut vc = VectorClock::new();
        vc.tick(osc.id);
        let st = StatefulShift::new(
            &alice,
            strict_domain,
            bob.account_id(),
            500,
            vc,
            vec![],
            1,
            0,
        );
        osc.ingest(Signal::Stateful(st), &registry).unwrap();

        // First synthesis: strict shift is inserted but skipped because quorum for
        // its insertion tick does not yet exist (certificate is produced at end).
        let result1 = osc.synthesize(&registry);
        assert_eq!(result1.stateful_applied, 0);

        // Second synthesis: the certificate from tick 0 now provides quorum, so
        // the strict shift applies.
        let result2 = osc.synthesize(&registry);
        assert_eq!(result2.stateful_applied, 1);
    }

    #[test]
    fn oscillator_emits_signed_rejection_proof_for_insufficient_balance() {
        let mut osc = Oscillator::new([1u8; 32], 64);
        let operator = KeyPair::generate();
        let alice = KeyPair::generate();
        let bob = KeyPair::generate();
        osc.set_operator_keypair(operator.clone());
        // Seed alice with less than she tries to spend.
        osc.seed_account(alice.account_id(), 100);

        let mut registry = HashMap::new();
        registry.insert(alice.account_id(), alice.public_key());

        let mut vc = VectorClock::new();
        vc.tick(osc.id);
        let st = StatefulShift::new(
            &alice,
            DEFAULT_DEX_DOMAIN,
            bob.account_id(),
            500,
            vc,
            vec![],
            1,
            0,
        );
        let hash = st.hash();
        osc.ingest(Signal::Stateful(st), &registry).unwrap();

        let result = osc.synthesize(&registry);
        assert_eq!(result.stateful_applied, 0);
        assert!(result.stateful_rejected.iter().any(|e| matches!(e, DagError::InsufficientBalance(h) if *h == hash)));

        let proof = osc
            .dag
            .lock()
            .unwrap()
            .rejection_proofs
            .get(&hash)
            .cloned()
            .expect("rejection proof should be stored");
        assert_eq!(proof.shift_hash, hash);
        assert!(matches!(proof.reason, RejectionReason::InsufficientBalance));
        assert!(proof.verify(&operator.public_key()));
    }
}
