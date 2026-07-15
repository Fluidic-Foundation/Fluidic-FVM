use crate::api::state::{ApiState, RecentShift, build_pool_payout_shift};
use crate::consensus::domain::{DomainPolicy, FeePolicy, StatefulOrdering};
use crate::consensus::dag::{DagError, RejectionReason, ShiftStatus, VectorClockDag};
use crate::crypto::{
    decrypt_signal, encrypt_signal, AccountId, AgentRegistrationShift, CommutativeShift,
    EncryptedSignal, EntanglementAttestShift, EntanglementBreakShift, EntanglementCreateShift,
    IntentConstraint, IntentFillShift, IntentShift, KeyPair, PhysicalAttestation,
    PhysicalResourceType, Signal, StakeShift, StatefulShift,
};
use crate::evm::{block_hash_for, evm_address_to_fluidic};
use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub fn api_router() -> Router<Arc<ApiState>> {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/state", get(get_state))
        .route("/api/account/:id/balance", get(get_balance))
        .route("/api/account/:id/shifts", get(get_account_shifts))
        .route("/api/account/:id", get(get_account_overview))
        .route("/api/account/register", post(register_account))
        .route("/api/agent/register", post(register_agent))
        .route("/api/intent/submit", post(submit_intent))
        .route("/api/intent/fill", post(submit_intent_fill))
        .route("/api/intents/open", get(get_open_intents))
        .route("/api/physical/attest", post(submit_physical_attestation))
        .route("/api/physical/attestations/open", get(get_open_physical_attestations))
        .route("/api/physical/attestations/:id", get(get_physical_attestation))
        .route("/api/entanglement/create", post(create_entanglement))
        .route("/api/entanglement/attest", post(attest_entanglement))
        .route("/api/entanglement/break", post(break_entanglement))
        .route("/api/entanglement/:id", get(get_entanglement))
        .route("/api/entanglements/:account", get(get_account_entanglements))
        .route("/api/shift/encrypt", post(encrypt_shift))
        .route("/api/shift/submit-encrypted", post(submit_encrypted_shift))
        .route("/api/shift/commutative", post(submit_commutative))
        .route("/api/shift/stateful", post(submit_stateful))
        .route("/api/shifts/recent", get(get_recent_shifts))
        .route("/api/shift/:hash/status", get(shift_status))
        .route("/api/shift/:hash/proof", get(get_rejection_proof))
        .route("/api/certificate/:tick", get(get_certificate))
        .route("/api/quorum/:tick", get(get_quorum_status))
        .route("/api/ticks/recent", get(get_recent_ticks))
        .route("/api/ticks/:tick", get(get_tick))
        .route("/api/operator/info", get(get_operator_info))
        .route("/api/operator/stake", post(submit_operator_stake))
        .route("/api/operators", get(get_staked_operators))
        .route("/api/operator/:id/rewards", get(get_operator_rewards))
        .route("/api/rewards/claim", post(claim_operator_rewards))
        .route("/api/rewards/lp/:pool_id/claim", post(claim_lp_rewards))
        .route("/api/supply", get(get_supply))
        .route("/api/debug/burn", get(debug_burn))
        .route("/api/domains", get(get_domains).post(register_domain))
        .route("/api/domain/:id", get(get_domain))
        .route("/api/evm/faucet", post(evm_faucet))
        .route("/api/sync/state", get(get_sync_state))
        .route("/api/sync/shifts", get(get_sync_shifts))
        .route("/api/ws", get(ws_handler))
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

#[derive(Serialize)]
struct StateResponse {
    wave_reserve: String,
    usdc_reserve: String,
    price: f64,
    throughput: f64,
    latency_ms: f64,
    metabolic_burned: String,
    commutative_applied: usize,
    stateful_applied: usize,
    evm_applied: usize,
    intents_matched: usize,
    /// Count of open physical attestations.
    physical_attestations: usize,
    /// Physical-state intents matched since node start.
    physical_intents_matched: usize,
    pool_wave_account: String,
    pool_usdc_account: String,
}

#[derive(Deserialize)]
struct StateQuery {
    #[serde(default)]
    min_tick: Option<u64>,
}

#[derive(Deserialize)]
struct CommutativeShiftRequest {
    #[serde(default)]
    domain: Option<String>,
    from: String,
    coordinate: CoordinateRequest,
    delta: String,
    pool_id: String,
    nonce: u64,
    timestamp_ns: u64,
    signature: String,
}

#[derive(Deserialize)]
struct CoordinateRequest {
    components: Vec<u64>,
}

/// Wait until the local oscillator has synthesized at least `min_tick`, with a
/// timeout to avoid blocking forever on isolated nodes.
async fn wait_for_min_tick(state: &ApiState, min_tick: Option<u64>) {
    let Some(min_tick) = min_tick else {
        return;
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while state.oscillator.synthesis_tick.load(std::sync::atomic::Ordering::SeqCst) < min_tick {
        if std::time::Instant::now() > deadline {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

async fn get_state(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<StateQuery>,
) -> impl IntoResponse {
    wait_for_min_tick(&state, query.min_tick).await;

    let snap = state.snapshot();
    let open_attestations = state
        .oscillator
        .pending_physical_attestations
        .lock()
        .unwrap()
        .len();
    Json(StateResponse {
        wave_reserve: snap.wave_reserve.to_string(),
        usdc_reserve: snap.usdc_reserve.to_string(),
        price: snap.price,
        throughput: snap.throughput,
        latency_ms: snap.latency_ms,
        metabolic_burned: snap.metabolic_burned.to_string(),
        commutative_applied: snap.commutative_applied,
        stateful_applied: snap.stateful_applied,
        evm_applied: snap.evm_applied,
        intents_matched: snap.intents_matched,
        physical_attestations: open_attestations,
        physical_intents_matched: snap.physical_intents_matched,
        pool_wave_account: hex::encode(state.pool_wave_account.0),
        pool_usdc_account: hex::encode(state.pool_usdc_account.0),
    })
}

#[derive(Serialize)]
struct BalanceResponse {
    wave: String,
    usdc: String,
}

async fn get_balance(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Query(query): Query<StateQuery>,
) -> Result<Json<BalanceResponse>, StatusCode> {
    wait_for_min_tick(&state, query.min_tick).await;

    let bytes = hex::decode(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    if bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    let user = AccountId(arr);
    let (wave_acc, usdc_acc) = state.token_accounts(user);

    let field = state.oscillator.wave_field.lock().unwrap();
    let wave = field.account_balance(wave_acc).units;
    let usdc = field.account_balance(usdc_acc).units;
    drop(field);

    Ok(Json(BalanceResponse {
        wave: wave.to_string(),
        usdc: usdc.to_string(),
    }))
}

/// Return the recent shifts that involve a given account (matching either the
/// main account id or its derived WAVE / USDC token accounts as sender or
/// recipient).  Backed by the in-memory recent-shift ring buffer.
async fn get_account_shifts(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Query(query): Query<StateQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    wait_for_min_tick(&state, query.min_tick).await;
    let user = parse_account(&id)?;
    let (wave_acc, usdc_acc) = state.token_accounts(user);
    let keys: [String; 3] = [
        user.to_string(),
        wave_acc.to_string(),
        usdc_acc.to_string(),
    ];
    let matches: Vec<RecentShift> = state
        .recent_shifts
        .lock()
        .unwrap()
        .iter()
        .filter(|s| {
            let from = s.from.as_deref().unwrap_or("");
            let to = s.to.as_deref().unwrap_or("");
            keys.iter().any(|k| k == from || k == to)
        })
        .cloned()
        .collect();
    Ok(Json(serde_json::json!({
        "account": id,
        "wave_account": wave_acc.to_string(),
        "usdc_account": usdc_acc.to_string(),
        "shifts": matches,
    })))
}

/// Aggregate wallet view for the explorer: balances, stake, accrued operator
/// rewards, and the recent shifts touching this account.
async fn get_account_overview(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Query(query): Query<StateQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    wait_for_min_tick(&state, query.min_tick).await;
    let user = parse_account(&id)?;
    let (wave_acc, usdc_acc) = state.token_accounts(user);

    let (wave, usdc) = {
        let field = state.oscillator.wave_field.lock().unwrap();
        (
            field.account_balance(wave_acc).units,
            field.account_balance(usdc_acc).units,
        )
    };

    let stake = state.oscillator.stake_table.get_stake(&user);
    let is_staked = state.oscillator.stake_table.is_staked(&user);
    let rewards = state.oscillator.reward_pool.read().unwrap().balance(&user);
    let registered = state.registry.read().unwrap().contains_key(&user);

    let keys: [String; 3] = [
        user.to_string(),
        wave_acc.to_string(),
        usdc_acc.to_string(),
    ];
    let shifts: Vec<RecentShift> = state
        .recent_shifts
        .lock()
        .unwrap()
        .iter()
        .filter(|s| {
            let from = s.from.as_deref().unwrap_or("");
            let to = s.to.as_deref().unwrap_or("");
            keys.iter().any(|k| k == from || k == to)
        })
        .cloned()
        .collect();

    Ok(Json(serde_json::json!({
        "account": id,
        "registered": registered,
        "wave_account": wave_acc.to_string(),
        "usdc_account": usdc_acc.to_string(),
        "wave": wave.to_string(),
        "usdc": usdc.to_string(),
        "stake": stake.to_string(),
        "is_staked": is_staked,
        "rewards": rewards.to_string(),
        "shift_count": shifts.len(),
        "shifts": shifts,
    })))
}

#[derive(Deserialize)]
struct RegisterRequest {
    public_key_hex: String,
}

async fn register_account(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let pk_bytes = hex::decode(&req.public_key_hex).map_err(|_| StatusCode::BAD_REQUEST)?;
    let vk = VerifyingKey::from_bytes(&pk_bytes.try_into().map_err(|_| StatusCode::BAD_REQUEST)?)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let account = AccountId::from_public_key(&vk);
    state.register_key(account, vk);

    // Faucet: seed both token accounts for the demo.
    let (wave_acc, usdc_acc) = state.token_accounts(account);
    state.oscillator.seed_account(wave_acc, 1_000_000_000_000_000); // 1,000 WAVE
    state.oscillator.seed_account(usdc_acc, 1_000_000_000_000_000); // 1,000 USDC
    // USDC is foreign value and is exempt from metabolic decay.
    state.oscillator.mark_non_decaying(usdc_acc);

    // Register derived token accounts so they can sign stateful shifts.
    state.register_key(wave_acc, vk);
    state.register_key(usdc_acc, vk);

    // Map derived token accounts back to the owner main account for payouts.
    state.register_derived(wave_acc, account);
    state.register_derived(usdc_acc, account);

    // Gossip the registration so every mesh node learns this account.
    state.broadcast_registration(crate::crypto::RegistrationShift {
        account,
        public_key: vk.to_bytes(),
        wave_account: wave_acc,
        usdc_account: usdc_acc,
        nonce: 0,
        timestamp_ns: 0,
    });

    Ok(Json(serde_json::json!({
        "account_id": account.to_string(),
        "wave_account": hex::encode(wave_acc.0),
        "usdc_account": hex::encode(usdc_acc.0),
    })))
}

#[derive(Deserialize)]
struct RegisterAgentRequest {
    owner: String,
    agent_public_key_hex: String,
    expiry_tick: u64,
    nonce: u64,
    timestamp_ns: u64,
    signature: String,
}

async fn register_agent(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<RegisterAgentRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let owner = parse_account(&req.owner).map_err(|e| (e, "invalid owner account".to_string()))?;
    let pk_bytes = hex::decode(&req.agent_public_key_hex)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid agent public key hex".to_string()))?;
    let vk = VerifyingKey::from_bytes(
        &pk_bytes.try_into().map_err(|_| {
            (StatusCode::BAD_REQUEST, "invalid agent public key length".to_string())
        })?,
    )
    .map_err(|_| (StatusCode::BAD_REQUEST, "invalid agent public key".to_string()))?;
    let agent = AccountId::from_public_key(&vk);
    let signature = hex::decode(&req.signature)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid signature hex".to_string()))?;

    let reg = AgentRegistrationShift {
        agent,
        owner,
        public_key: vk.to_bytes(),
        expiry_tick: req.expiry_tick,
        nonce: req.nonce,
        timestamp_ns: req.timestamp_ns,
        signature,
    };

    let registry = state.key_registry();
    if !reg.verify(
        registry
            .get(&owner)
            .ok_or_else(|| (StatusCode::UNAUTHORIZED, "unknown owner".to_string()))?,
    ) {
        return Err((StatusCode::UNAUTHORIZED, "invalid owner signature".to_string()));
    }

    // Apply the registration (and debit the anti-spam fee) before gossiping it.
    state
        .oscillator
        .ingest(Signal::AgentRegistration(reg.clone()), &registry)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("agent registration ingest failed: {}", e)))?;

    state.register_key(agent, vk);
    state.broadcast_agent_registration(reg);

    Ok(Json(serde_json::json!({
        "agent_id": agent.to_string(),
        "owner": owner.to_string(),
        "expiry_tick": req.expiry_tick,
    })))
}

#[derive(Deserialize)]
struct IntentRequest {
    owner: String,
    #[serde(default)]
    domain: Option<String>,
    deadline_tick: u64,
    constraint: IntentConstraintRequest,
    solver_reward: String,
    nonce: u64,
    timestamp_ns: u64,
    signature: String,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum IntentConstraintRequest {
    Transfer { to: String, min_amount: String },
    Swap {
        from_token: String,
        to_token: String,
        min_out: String,
        max_slippage_bp: u64,
    },
    PhysicalResource {
        resource_type: PhysicalResourceTypeRequest,
        location_prefix: String,
        min_capacity: String,
        max_price_per_unit: String,
        duration_ticks: u64,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum PhysicalResourceTypeRequest {
    Storage,
    Bandwidth,
    Compute,
    Energy,
    Sensor,
}

impl From<PhysicalResourceTypeRequest> for PhysicalResourceType {
    fn from(req: PhysicalResourceTypeRequest) -> Self {
        match req {
            PhysicalResourceTypeRequest::Storage => PhysicalResourceType::Storage,
            PhysicalResourceTypeRequest::Bandwidth => PhysicalResourceType::Bandwidth,
            PhysicalResourceTypeRequest::Compute => PhysicalResourceType::Compute,
            PhysicalResourceTypeRequest::Energy => PhysicalResourceType::Energy,
            PhysicalResourceTypeRequest::Sensor => PhysicalResourceType::Sensor,
        }
    }
}

impl IntentConstraintRequest {
    fn into_constraint(self) -> Result<IntentConstraint, (StatusCode, String)> {
        match self {
            IntentConstraintRequest::Transfer { to, min_amount } => Ok(IntentConstraint::Transfer {
                to: parse_account(&to).map_err(|e| (e, "invalid transfer to account".to_string()))?,
                min_amount: parse_u128(&min_amount)
                    .map_err(|e| (e, "invalid min_amount".to_string()))?,
            }),
            IntentConstraintRequest::Swap {
                from_token,
                to_token,
                min_out,
                max_slippage_bp,
            } => Ok(IntentConstraint::Swap {
                from_token: parse_account(&from_token)
                    .map_err(|e| (e, "invalid from_token".to_string()))?,
                to_token: parse_account(&to_token)
                    .map_err(|e| (e, "invalid to_token".to_string()))?,
                min_out: parse_u128(&min_out).map_err(|e| (e, "invalid min_out".to_string()))?,
                max_slippage_bp,
            }),
            IntentConstraintRequest::PhysicalResource {
                resource_type,
                location_prefix,
                min_capacity,
                max_price_per_unit,
                duration_ticks,
            } => Ok(IntentConstraint::PhysicalResource {
                resource_type: resource_type.into(),
                location_prefix,
                min_capacity: parse_u128(&min_capacity).map_err(|e| (e, "invalid min_capacity".to_string()))?,
                max_price_per_unit: parse_u128(&max_price_per_unit).map_err(|e| (e, "invalid max_price_per_unit".to_string()))?,
                duration_ticks,
            }),
        }
    }
}

async fn submit_intent(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<IntentRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let owner = parse_account(&req.owner).map_err(|e| (e, "invalid owner account".to_string()))?;
    let domain = match req.domain {
        Some(hex) => parse_domain(&hex).map_err(|e| (e, "invalid domain".to_string()))?,
        None => crate::crypto::DEFAULT_DEX_DOMAIN,
    };
    let constraint = req.constraint.into_constraint()?;
    let solver_reward = parse_u128(&req.solver_reward)
        .map_err(|e| (e, "invalid solver_reward".to_string()))?;
    let signature = hex::decode(&req.signature)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid signature hex".to_string()))?;

    let mut intent = IntentShift {
        owner,
        intent_id: [0u8; 32],
        domain,
        deadline_tick: req.deadline_tick,
        constraint,
        solver_reward,
        nonce: req.nonce,
        timestamp_ns: req.timestamp_ns,
        signature,
    };
    intent.intent_id = intent.hash();

    let registry = state.key_registry();
    if !intent.verify(
        registry
            .get(&owner)
            .ok_or_else(|| (StatusCode::UNAUTHORIZED, "unknown owner".to_string()))?,
    ) {
        return Err((StatusCode::UNAUTHORIZED, "invalid owner signature".to_string()));
    }

    let hash = intent.hash();
    state.broadcast_intent(intent.clone());
    state
        .oscillator
        .ingest(Signal::Intent(intent), &registry)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("intent ingest failed: {}", e)))?;

    Ok(Json(serde_json::json!({
        "status": "queued",
        "intent_id": hex::encode(hash),
    })))
}

#[derive(Deserialize)]
struct IntentFillRequest {
    intent_id: String,
    solver: String,
    fill_amount: String,
    nonce: u64,
    timestamp_ns: u64,
    signature: String,
}

async fn submit_intent_fill(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<IntentFillRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let intent_id = parse_hash(&req.intent_id)
        .map_err(|e| (e, "invalid intent_id".to_string()))?;
    let solver = parse_account(&req.solver).map_err(|e| (e, "invalid solver account".to_string()))?;
    let fill_amount = parse_u128(&req.fill_amount)
        .map_err(|e| (e, "invalid fill_amount".to_string()))?;
    let signature = hex::decode(&req.signature)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid signature hex".to_string()))?;

    let fill = IntentFillShift {
        intent_id,
        solver,
        fill_amount,
        nonce: req.nonce,
        timestamp_ns: req.timestamp_ns,
        signature,
    };

    let registry = state.key_registry();
    if !fill.verify(
        registry
            .get(&solver)
            .ok_or_else(|| (StatusCode::UNAUTHORIZED, "unknown solver".to_string()))?,
    ) {
        return Err((StatusCode::UNAUTHORIZED, "invalid solver signature".to_string()));
    }

    let hash = fill.hash();
    state.broadcast_intent_fill(fill.clone());
    state
        .oscillator
        .ingest(Signal::IntentFill(fill), &registry)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("intent fill ingest failed: {}", e)))?;

    Ok(Json(serde_json::json!({
        "status": "queued",
        "fill_hash": hex::encode(hash),
    })))
}

async fn get_open_intents(State(state): State<Arc<ApiState>>) -> Json<serde_json::Value> {
    let intents: Vec<_> = state
        .oscillator
        .pending_intents
        .lock()
        .unwrap()
        .iter()
        .map(|i| {
            serde_json::json!({
                "intent_id": hex::encode(i.intent_id),
                "owner": i.owner.to_string(),
                "domain": hex::encode(i.domain),
                "deadline_tick": i.deadline_tick,
                "solver_reward": i.solver_reward.to_string(),
                "nonce": i.nonce,
                "timestamp_ns": i.timestamp_ns,
            })
        })
        .collect();
    Json(serde_json::json!({ "intents": intents }))
}

#[derive(Deserialize)]
struct PhysicalAttestationRequest {
    publisher: String,
    resource_type: PhysicalResourceTypeRequest,
    location: String,
    capacity: String,
    price_per_unit: String,
    available_until_tick: u64,
    nonce: u64,
    timestamp_ns: u64,
    signature: String,
}

async fn submit_physical_attestation(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<PhysicalAttestationRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let publisher = parse_account(&req.publisher)
        .map_err(|e| (e, "invalid publisher account".to_string()))?;
    let capacity = parse_u128(&req.capacity)
        .map_err(|e| (e, "invalid capacity".to_string()))?;
    let price_per_unit = parse_u128(&req.price_per_unit)
        .map_err(|e| (e, "invalid price_per_unit".to_string()))?;
    let signature = hex::decode(&req.signature)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid signature hex".to_string()))?;

    let attestation = PhysicalAttestation {
        publisher,
        attestation_id: [0u8; 32],
        resource_type: req.resource_type.into(),
        location: req.location,
        capacity,
        price_per_unit,
        available_until_tick: req.available_until_tick,
        nonce: req.nonce,
        timestamp_ns: req.timestamp_ns,
        signature,
    };

    let registry = state.key_registry();
    if !attestation.verify(
        registry
            .get(&publisher)
            .ok_or_else(|| (StatusCode::UNAUTHORIZED, "unknown publisher".to_string()))?,
    ) {
        return Err((StatusCode::UNAUTHORIZED, "invalid publisher signature".to_string()));
    }

    let hash = attestation.hash();
    state.broadcast_physical_attestation(attestation.clone());
    state
        .oscillator
        .ingest(Signal::PhysicalAttestation(attestation), &registry)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("physical attestation ingest failed: {}", e)))?;

    Ok(Json(serde_json::json!({
        "status": "queued",
        "attestation_id": hex::encode(hash),
    })))
}

async fn get_open_physical_attestations(State(state): State<Arc<ApiState>>) -> Json<serde_json::Value> {
    let attestations: Vec<_> = state
        .oscillator
        .pending_physical_attestations
        .lock()
        .unwrap()
        .iter()
        .map(|pending| {
            let a = &pending.attestation;
            serde_json::json!({
                "attestation_id": hex::encode(a.attestation_id),
                "publisher": a.publisher.to_string(),
                "resource_type": serde_json::to_value(&a.resource_type).unwrap_or(serde_json::Value::Null),
                "location": &a.location,
                "capacity": a.capacity.to_string(),
                "remaining_capacity": pending.remaining_capacity.to_string(),
                "price_per_unit": a.price_per_unit.to_string(),
                "available_until_tick": a.available_until_tick,
                "nonce": a.nonce,
                "timestamp_ns": a.timestamp_ns,
            })
        })
        .collect();
    Json(serde_json::json!({ "attestations": attestations }))
}

async fn get_physical_attestation(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let bytes = hex::decode(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    if bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&bytes);

    let guard = state.oscillator.pending_physical_attestations.lock().unwrap();
    let pending = guard
        .iter()
        .find(|p| p.attestation.attestation_id == hash)
        .ok_or(StatusCode::NOT_FOUND)?;
    let a = &pending.attestation;
    Ok(Json(serde_json::json!({
        "attestation_id": hex::encode(a.attestation_id),
        "publisher": a.publisher.to_string(),
        "resource_type": serde_json::to_value(&a.resource_type).unwrap_or(serde_json::Value::Null),
        "location": &a.location,
        "capacity": a.capacity.to_string(),
        "remaining_capacity": pending.remaining_capacity.to_string(),
        "price_per_unit": a.price_per_unit.to_string(),
        "available_until_tick": a.available_until_tick,
        "nonce": a.nonce,
        "timestamp_ns": a.timestamp_ns,
    })))
}

#[derive(Deserialize)]
struct EntanglementCreateRequest {
    id: String,
    creator: String,
    subject: String,
    witnesses: Vec<String>,
    threshold: usize,
    expiry_tick: u64,
    nonce: u64,
    timestamp_ns: u64,
    signature: String,
}

async fn create_entanglement(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<EntanglementCreateRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let id = parse_hash(&req.id).map_err(|e| (e, "invalid entanglement id".to_string()))?;
    let creator = parse_account(&req.creator).map_err(|e| (e, "invalid creator account".to_string()))?;
    let subject = parse_account(&req.subject).map_err(|e| (e, "invalid subject account".to_string()))?;
    let witnesses: Vec<AccountId> = req
        .witnesses
        .iter()
        .map(|h| parse_account(h).map_err(|e| (e, format!("invalid witness account: {}", h))))
        .collect::<Result<_, _>>()?;
    let signature = hex::decode(&req.signature)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid signature hex".to_string()))?;

    let shift = EntanglementCreateShift {
        id,
        creator,
        subject,
        witnesses,
        threshold: req.threshold,
        expiry_tick: req.expiry_tick,
        nonce: req.nonce,
        timestamp_ns: req.timestamp_ns,
        signature,
    };

    let registry = state.key_registry();
    if !shift.verify(
        registry
            .get(&creator)
            .ok_or_else(|| (StatusCode::UNAUTHORIZED, "unknown creator".to_string()))?,
    ) {
        return Err((StatusCode::UNAUTHORIZED, "invalid creator signature".to_string()));
    }

    let recomputed = EntanglementCreateShift::recompute_id(
        creator,
        subject,
        &shift.witnesses,
        shift.threshold,
        shift.expiry_tick,
        shift.nonce,
        shift.timestamp_ns,
    );
    if recomputed != id {
        return Err((StatusCode::BAD_REQUEST, "entanglement id does not match recomputed id".to_string()));
    }

    state.broadcast_entanglement_create(shift.clone());
    state
        .oscillator
        .ingest(Signal::EntanglementCreate(shift), &registry)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("entanglement create ingest failed: {}", e)))?;

    Ok(Json(serde_json::json!({
        "status": "created",
        "entanglement_id": hex::encode(id),
    })))
}

#[derive(Deserialize)]
struct EntanglementAttestRequest {
    entanglement_id: String,
    witness: String,
    nonce: u64,
    timestamp_ns: u64,
    signature: String,
}

async fn attest_entanglement(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<EntanglementAttestRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let entanglement_id = parse_hash(&req.entanglement_id)
        .map_err(|e| (e, "invalid entanglement_id".to_string()))?;
    let witness = parse_account(&req.witness).map_err(|e| (e, "invalid witness account".to_string()))?;
    let signature = hex::decode(&req.signature)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid signature hex".to_string()))?;

    let shift = EntanglementAttestShift {
        entanglement_id,
        witness,
        nonce: req.nonce,
        timestamp_ns: req.timestamp_ns,
        signature,
    };

    let registry = state.key_registry();
    if !shift.verify(
        registry
            .get(&witness)
            .ok_or_else(|| (StatusCode::UNAUTHORIZED, "unknown witness".to_string()))?,
    ) {
        return Err((StatusCode::UNAUTHORIZED, "invalid witness signature".to_string()));
    }

    state.broadcast_entanglement_attest(shift.clone());
    state
        .oscillator
        .ingest(Signal::EntanglementAttest(shift), &registry)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("entanglement attest ingest failed: {}", e)))?;

    Ok(Json(serde_json::json!({
        "status": "attested",
        "entanglement_id": hex::encode(entanglement_id),
        "witness": witness.to_string(),
    })))
}

#[derive(Deserialize)]
struct EntanglementBreakRequest {
    entanglement_id: String,
    breaker: String,
    nonce: u64,
    timestamp_ns: u64,
    signature: String,
}

async fn break_entanglement(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<EntanglementBreakRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let entanglement_id = parse_hash(&req.entanglement_id)
        .map_err(|e| (e, "invalid entanglement_id".to_string()))?;
    let breaker = parse_account(&req.breaker).map_err(|e| (e, "invalid breaker account".to_string()))?;
    let signature = hex::decode(&req.signature)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid signature hex".to_string()))?;

    let shift = EntanglementBreakShift {
        entanglement_id,
        breaker,
        nonce: req.nonce,
        timestamp_ns: req.timestamp_ns,
        signature,
    };

    let registry = state.key_registry();
    if !shift.verify(
        registry
            .get(&breaker)
            .ok_or_else(|| (StatusCode::UNAUTHORIZED, "unknown breaker".to_string()))?,
    ) {
        return Err((StatusCode::UNAUTHORIZED, "invalid breaker signature".to_string()));
    }

    state.broadcast_entanglement_break(shift.clone());
    state
        .oscillator
        .ingest(Signal::EntanglementBreak(shift), &registry)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("entanglement break ingest failed: {}", e)))?;

    Ok(Json(serde_json::json!({
        "status": "broken",
        "entanglement_id": hex::encode(entanglement_id),
    })))
}

async fn get_entanglement(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let bytes = hex::decode(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    if bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut eid = [0u8; 32];
    eid.copy_from_slice(&bytes);

    let entanglements = state.oscillator.entanglements.read().unwrap();
    let contract = entanglements.get(&eid).ok_or(StatusCode::NOT_FOUND)?;
    let attestation_count = state
        .oscillator
        .entanglement_attestations
        .read()
        .unwrap()
        .get(&eid)
        .map(|s| s.len())
        .unwrap_or(0);
    let current_tick = state.oscillator.synthesis_tick.load(std::sync::atomic::Ordering::SeqCst);

    Ok(Json(serde_json::json!({
        "entanglement_id": hex::encode(contract.id),
        "creator": contract.creator.to_string(),
        "subject": contract.subject.to_string(),
        "witnesses": contract.witnesses.iter().map(|a| a.to_string()).collect::<Vec<_>>(),
        "threshold": contract.threshold,
        "expiry_tick": contract.expiry_tick,
        "created_tick": contract.created_tick,
        "active": current_tick <= contract.expiry_tick,
        "attestations_this_tick": attestation_count,
    })))
}

async fn get_account_entanglements(
    State(state): State<Arc<ApiState>>,
    Path(account_hex): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let account = parse_account(&account_hex)?;
    let current_tick = state.oscillator.synthesis_tick.load(std::sync::atomic::Ordering::SeqCst);

    let items: Vec<serde_json::Value> = state
        .oscillator
        .entanglements
        .read()
        .unwrap()
        .values()
        .filter(|c| {
            c.subject == account || c.creator == account || c.witnesses.contains(&account)
        })
        .map(|c| {
            let attestation_count = state
                .oscillator
                .entanglement_attestations
                .read()
                .unwrap()
                .get(&c.id)
                .map(|s| s.len())
                .unwrap_or(0);
            serde_json::json!({
                "entanglement_id": hex::encode(c.id),
                "role": if c.subject == account { "subject" }
                    else if c.creator == account { "creator" }
                    else { "witness" },
                "creator": c.creator.to_string(),
                "subject": c.subject.to_string(),
                "witnesses": c.witnesses.iter().map(|a| a.to_string()).collect::<Vec<_>>(),
                "threshold": c.threshold,
                "expiry_tick": c.expiry_tick,
                "created_tick": c.created_tick,
                "active": current_tick <= c.expiry_tick,
                "attestations_this_tick": attestation_count,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "entanglements": items })))
}

#[derive(Deserialize)]
struct EncryptShiftRequest {
    /// Hex-encoded network PSK used to encrypt the payload.
    psk_hex: String,
    /// JSON-serialized inner signal to encrypt.
    signal: serde_json::Value,
}

async fn encrypt_shift(
    Json(req): Json<EncryptShiftRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let psk = hex::decode(&req.psk_hex)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid psk hex".to_string()))?;
    let signal: Signal = serde_json::from_value(req.signal)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid signal json: {}", e)))?;
    let enc = encrypt_signal(&psk, &signal)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("encryption failed: {}", e)))?;
    Ok(Json(serde_json::json!({
        "nonce": hex::encode(enc.nonce),
        "ciphertext": hex::encode(&enc.ciphertext),
        "tag": hex::encode(enc.tag),
    })))
}

#[derive(Deserialize)]
struct SubmitEncryptedShiftRequest {
    /// Hex-encoded network PSK used to decrypt the payload.
    psk_hex: String,
    nonce: String,
    ciphertext: String,
    tag: String,
}

async fn submit_encrypted_shift(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<SubmitEncryptedShiftRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let psk = hex::decode(&req.psk_hex)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid psk hex".to_string()))?;
    let nonce_bytes = hex::decode(&req.nonce)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid nonce hex".to_string()))?;
    let mut nonce = [0u8; 12];
    if nonce_bytes.len() != 12 {
        return Err((StatusCode::BAD_REQUEST, "nonce must be 12 bytes".to_string()));
    }
    nonce.copy_from_slice(&nonce_bytes);
    let ciphertext = hex::decode(&req.ciphertext)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid ciphertext hex".to_string()))?;
    let tag_bytes = hex::decode(&req.tag)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid tag hex".to_string()))?;
    let mut tag = [0u8; 16];
    if tag_bytes.len() != 16 {
        return Err((StatusCode::BAD_REQUEST, "tag must be 16 bytes".to_string()));
    }
    tag.copy_from_slice(&tag_bytes);

    let enc = EncryptedSignal {
        nonce,
        ciphertext,
        tag,
    };
    let signal = decrypt_signal(&psk, &enc)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("decryption failed: {}", e)))?;

    let registry = state.key_registry();
    let hash = signal.hash();
    state
        .oscillator
        .ingest(signal, &registry)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("ingest failed: {}", e)))?;

    Ok(Json(serde_json::json!({
        "status": "queued",
        "hash": hex::encode(hash),
    })))
}

#[derive(Deserialize)]
struct EvmFaucetRequest {
    address: String,
}

async fn evm_faucet(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<EvmFaucetRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let addr_bytes = hex::decode(req.address.trim_start_matches("0x"))
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    if addr_bytes.len() != 20 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let addr = ethers_core::types::Address::from_slice(&addr_bytes);
    let fluidic_account = evm_address_to_fluidic(&addr);
    state.oscillator.seed_account(fluidic_account, 1_000_000_000_000_000); // 1,000 WAVE
    Ok(Json(serde_json::json!({
        "address": req.address,
        "fluidic_account": fluidic_account.to_string(),
        "dripped_wave": "1000",
    })))
}

#[derive(Deserialize)]
struct VectorClockInput {
    entries: std::collections::HashMap<String, u64>,
}

#[derive(Deserialize)]
struct StatefulShiftRequest {
    from: String,
    to: String,
    amount: String,
    #[serde(default)]
    domain: Option<crate::crypto::DomainId>,
    #[serde(default)]
    vector_clock: Option<VectorClockInput>,
    predecessors: Vec<String>,
    nonce: u64,
    timestamp_ns: u64,
    signature: String,
}

fn parse_account(hex: &str) -> Result<AccountId, StatusCode> {
    let bytes = hex::decode(hex).map_err(|_| StatusCode::BAD_REQUEST)?;
    if bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(AccountId(arr))
}

fn parse_domain(hex: &str) -> Result<[u8; 32], StatusCode> {
    parse_hash(hex)
}

fn parse_hash(hex: &str) -> Result<[u8; 32], StatusCode> {
    let bytes = hex::decode(hex).map_err(|_| StatusCode::BAD_REQUEST)?;
    if bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

fn parse_u128(s: &str) -> Result<u128, StatusCode> {
    s.parse().map_err(|_| StatusCode::BAD_REQUEST)
}

fn parse_stateful_shift(req: StatefulShiftRequest) -> Result<StatefulShift, (StatusCode, String)> {
    let from = parse_account(&req.from).map_err(|e| (e, "invalid from account".to_string()))?;
    let to = parse_account(&req.to).map_err(|e| (e, "invalid to account".to_string()))?;
    let amount = parse_u128(&req.amount).map_err(|e| (e, "invalid amount".to_string()))?;
    let signature = hex::decode(&req.signature)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid signature hex".to_string()))?;

    let clock_map = req.vector_clock
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "missing vector_clock".to_string()))?
        .entries;
    let mut vector_clock = crate::crypto::VectorClock::new();
    for (node_hex, time) in clock_map {
        let node = parse_hash(&node_hex).map_err(|e| (e, "invalid vector_clock node".to_string()))?;
        vector_clock.0.insert(node, time);
    }

    let predecessors = req
        .predecessors
        .into_iter()
        .map(|h| parse_hash(&h).map_err(|e| (e, "invalid predecessor".to_string())))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(StatefulShift {
        domain: req.domain.unwrap_or(crate::crypto::DEFAULT_DEX_DOMAIN),
        from,
        to,
        amount,
        vector_clock,
        predecessors,
        nonce: req.nonce,
        timestamp_ns: req.timestamp_ns,
        first_seen_at_ns: 0,
        signature,
    })
}

async fn submit_stateful(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<StatefulShiftRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let shift = parse_stateful_shift(req)?;

    let registry = state.key_registry();
    let pk = match registry.get(&shift.from) {
        Some(pk) => pk,
        None => {
            tracing::warn!("stateful shift rejected: unknown sender {}", shift.from);
            return Err((StatusCode::UNAUTHORIZED, "unknown sender".to_string()));
        }
    };
    let sig = Signature::from_slice(&shift.signature)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid signature bytes".to_string()))?;
    if !KeyPair::verify(pk, &shift.signing_bytes(), &sig) {
        tracing::warn!(
            "stateful shift rejected: invalid signature from {}",
            shift.from
        );
        return Err((StatusCode::UNAUTHORIZED, "invalid signature".to_string()));
    }

    // Validate vector clock against locally observed causal history.
    {
        let dag = state.oscillator.dag.lock().unwrap();
        if let Err(e) = dag.validate_vector_clock(shift.from, &shift.vector_clock) {
            tracing::warn!(
                "stateful shift rejected: invalid vector clock from {}: {}",
                shift.from,
                e
            );
            return Err((StatusCode::BAD_REQUEST, e));
        }
    }

    // If the shift targets a pool, create a matching payout.
    let is_wave_to_pool = shift.to == state.pool_wave_account;
    let is_usdc_to_pool = shift.to == state.pool_usdc_account;
    let _is_pool_payout = shift.from == state.pool_wave_account || shift.from == state.pool_usdc_account;

    let token = if is_wave_to_pool || shift.from == state.pool_wave_account {
        "WAVE"
    } else if is_usdc_to_pool || shift.from == state.pool_usdc_account {
        "USDC"
    } else {
        "units"
    };

    if is_wave_to_pool || is_usdc_to_pool {
        let main_account = state.main_account(shift.from)
            .ok_or_else(|| (StatusCode::BAD_REQUEST, "derived account not registered".to_string()))?;
        let (wave_user, usdc_user) = state.token_accounts(main_account);

        let (payout_from, payout_to, payout_amount) = if is_wave_to_pool {
            let out = state.wave_to_usdc_out(shift.amount);
            if out == 0 {
                return Err((StatusCode::SERVICE_UNAVAILABLE, "swap output is zero".to_string()));
            }
            (state.pool_usdc_account, usdc_user, out)
        } else {
            let out = state.usdc_to_wave_out(shift.amount);
            if out == 0 {
                return Err((StatusCode::SERVICE_UNAVAILABLE, "swap output is zero".to_string()));
            }
            (state.pool_wave_account, wave_user, out)
        };

        let payout = {
            let dag = state.oscillator.dag.lock().unwrap();
            let tip = dag.account_tip(&payout_from);
            build_pool_payout_shift(&state.pool_keypair, payout_from, payout_to, payout_amount, shift.nonce, &tip)
        };
        state
            .oscillator
            .ingest(Signal::Stateful(payout.clone()), &registry)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("payout ingest failed: {}", e)))?;
        state.broadcast_stateful_shift(payout);
    }

    let user_hash = shift.hash();
    state.record_shift(RecentShift {
        hash: hex::encode(user_hash),
        kind: "stateful".to_string(),
        status: "accepted".to_string(),
        domain: Some(hex::encode(shift.domain)),
        from: Some(shift.from.to_string()),
        to: Some(shift.to.to_string()),
        amount: Some(shift.amount.to_string()),
        token: Some(token.to_string()),
        timestamp_ns: shift.timestamp_ns,
    });
    state
        .oscillator
        .ingest(Signal::Stateful(shift.clone()), &registry)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("shift ingest failed: {}", e)))?;
    state.broadcast_stateful_shift(shift);

    Ok(Json(serde_json::json!({
        "status": "queued",
        "hash": hex::encode(user_hash)
    })))
}

async fn submit_commutative(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<CommutativeShiftRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let domain = match req.domain {
        Some(hex) => parse_domain(&hex)?,
        None => crate::crypto::DEFAULT_DEX_DOMAIN,
    };
    let components: [u64; 4] = req
        .coordinate
        .components
        .try_into()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let delta = req
        .delta
        .parse::<i128>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let pool_id = parse_hash(&req.pool_id)?;
    let signature = hex::decode(&req.signature).map_err(|_| StatusCode::BAD_REQUEST)?;
    let from = parse_account(&req.from).map_err(|_| StatusCode::BAD_REQUEST)?;

    let shift = CommutativeShift {
        domain,
        from,
        coordinate: crate::field::coordinates::Coordinate::new(components),
        delta,
        pool_id,
        nonce: req.nonce,
        timestamp_ns: req.timestamp_ns,
        first_seen_at_ns: 0,
        signature,
    };

    let registry = state.key_registry();
    let hash = shift.hash();
    state.record_shift(RecentShift {
        hash: hex::encode(hash),
        kind: "commutative".to_string(),
        status: "accepted".to_string(),
        domain: Some(hex::encode(shift.domain)),
        from: Some(shift.from.to_string()),
        to: None,
        amount: Some(shift.delta.to_string()),
        token: Some("units".to_string()),
        timestamp_ns: shift.timestamp_ns,
    });
    state
        .oscillator
        .ingest(Signal::Commutative(shift.clone()), &registry)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    state.broadcast_commutative_shift(shift);
    Ok(Json(serde_json::json!({
        "hash": hex::encode(hash),
        "status": "queued"
    })))
}

async fn get_operator_info(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<StateQuery>,
) -> impl IntoResponse {
    wait_for_min_tick(&state, query.min_tick).await;
    let guard = state.operator_keypair.lock().unwrap();
    match guard.as_ref() {
        Some(kp) => {
            let account = kp.account_id();
            let stake = state.oscillator.stake_table.get_stake(&account);
            let min_stake = state.oscillator.stake_table.min_stake();
            Json(serde_json::json!({
                "account": account.to_string(),
                "public_key": hex::encode(kp.public_key().to_bytes()),
                "stake": stake.to_string(),
                "min_stake": min_stake.to_string(),
                "is_staked": state.oscillator.stake_table.is_staked(&account),
            }))
            .into_response()
        }
        None => (StatusCode::SERVICE_UNAVAILABLE, "operator keypair not configured").into_response(),
    }
}

#[derive(Deserialize)]
struct StakeRequest {
    amount: String,
}

async fn submit_operator_stake(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<StakeRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let amount = req
        .amount
        .parse::<u128>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let kp = state
        .operator_keypair
        .lock()
        .unwrap()
        .clone()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let timestamp_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let stake = StakeShift::sign(&kp, amount, 0, timestamp_ns);

    if !state.oscillator.apply_stake(&stake) {
        return Err(StatusCode::BAD_REQUEST);
    }
    state.broadcast_stake(stake.clone());

    Ok(Json(serde_json::json!({
        "status": "staked",
        "operator": kp.account_id().to_string(),
        "amount": amount.to_string(),
        "is_staked": state.oscillator.stake_table.is_staked(&kp.account_id()),
    })))
}

async fn get_staked_operators(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<StateQuery>,
) -> impl IntoResponse {
    wait_for_min_tick(&state, query.min_tick).await;
    let operators: Vec<_> = state
        .oscillator
        .stake_table
        .staked_operators()
        .into_iter()
        .map(|(account, stake)| {
            serde_json::json!({
                "account": account.to_string(),
                "stake": stake.to_string(),
            })
        })
        .collect();
    Json(serde_json::json!({ "operators": operators }))
}

async fn get_quorum_status(
    State(state): State<Arc<ApiState>>,
    Path(tick): Path<u64>,
    Query(query): Query<StateQuery>,
) -> impl IntoResponse {
    wait_for_min_tick(&state, query.min_tick).await;
    let threshold = state.oscillator.stake_table.quorum_threshold();
    let total = state.oscillator.stake_table.total_stake();
    match state.oscillator.check_quorum(tick) {
        Some((view, stake)) => Json(serde_json::json!({
            "tick": tick,
            "finalized": true,
            "stake": stake.to_string(),
            "threshold": threshold.to_string(),
            "total_stake": total.to_string(),
            "roots": {
                "commutative": hex::encode(view.commutative_root),
                "stateful": hex::encode(view.stateful_root),
                "evm": hex::encode(view.evm_root),
                "balances": hex::encode(view.balances_root),
                "stake": hex::encode(view.stake_root),
                "reward": hex::encode(view.reward_root),
            }
        })),
        None => Json(serde_json::json!({
            "tick": tick,
            "finalized": false,
            "threshold": threshold.to_string(),
            "total_stake": total.to_string(),
        })),
    }
}

async fn get_certificate(
    State(state): State<Arc<ApiState>>,
    Path(tick): Path<u64>,
    Query(query): Query<StateQuery>,
) -> Result<Json<crate::consensus::certificate::SynthesisCertificate>, StatusCode> {
    wait_for_min_tick(&state, query.min_tick).await;
    let certs = state.oscillator.certificates.read().unwrap();
    certs
        .get(&tick)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)
        .map(Json)
}

async fn get_operator_rewards(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Query(query): Query<StateQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    wait_for_min_tick(&state, query.min_tick).await;
    let bytes = hex::decode(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    if bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    let account = AccountId(arr);
    let balance = state.oscillator.reward_pool.read().unwrap().balance(&account);
    Ok(Json(serde_json::json!({
        "account": id,
        "rewards": balance.to_string(),
    })))
}

async fn claim_operator_rewards(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<StateQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    wait_for_min_tick(&state, query.min_tick).await;
    let kp = state
        .operator_keypair
        .lock()
        .unwrap()
        .clone()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let account = kp.account_id();
    let claimed = state.oscillator.reward_pool.read().unwrap().claim(&account);
    if claimed > 0 {
        state.oscillator.seed_account(account, claimed);
    }
    Ok(Json(serde_json::json!({
        "account": account.to_string(),
        "claimed": claimed.to_string(),
    })))
}

#[derive(Deserialize)]
struct LpClaimRequest {
    pool_id: String,
}

async fn claim_lp_rewards(
    State(state): State<Arc<ApiState>>,
    Path(pool_id): Path<String>,
    Query(query): Query<StateQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    wait_for_min_tick(&state, query.min_tick).await;
    let bytes = hex::decode(&pool_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    if bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    let pool = arr;

    let claimed = state.oscillator.reward_pool.read().unwrap().claim_lp_reward(pool);
    if claimed > 0 {
        // LP rewards accrue to the pool reserves as protocol-owned liquidity.
        // This rewards all LPs implicitly by deepening the pool they share.
        let half = claimed / 2;
        state.oscillator.seed_account(state.pool_wave_account, half);
        state.oscillator.seed_account(state.pool_usdc_account, claimed - half);
    }
    Ok(Json(serde_json::json!({
        "pool_id": pool_id,
        "claimed": claimed.to_string(),
    })))
}

async fn get_supply(
    State(state): State<Arc<ApiState>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "total": crate::value::supply::TOTAL_WAVE_SUPPLY.to_string(),
        "circulating": state.oscillator.supply_tracker.circulating().to_string(),
        "burned": state.oscillator.supply_tracker.burned().to_string(),
        "remaining": state.oscillator.supply_tracker.remaining().to_string(),
    }))
}

/// Debug endpoint: break down where metabolic burn is coming from.
async fn debug_burn(State(state): State<Arc<ApiState>>) -> Json<serde_json::Value> {
    let immune_accounts: std::collections::HashSet<crate::crypto::AccountId> = state
        .oscillator
        .stake_table
        .staked_operators()
        .into_iter()
        .map(|(operator, _)| operator)
        .collect();

    let mut total_accounts = 0u64;
    let mut total_wave_units = 0u128;
    let mut immune_wave_units = 0u128;
    let mut non_decaying_wave_units = 0u128;
    let mut decayable_wave_units = 0u128;
    let mut decayable_accounts = 0u64;
    let mut account_summaries: Vec<serde_json::Value> = Vec::new();

    let field = state.oscillator.wave_field.lock().unwrap();
    for domain_entry in field.domains.iter() {
        for entry in domain_entry.value().accounts.iter() {
            total_accounts += 1;
            let id = *entry.key();
            let balance = &entry.value().balance;
            if !balance.decays {
                non_decaying_wave_units += balance.units;
                continue;
            }
            total_wave_units += balance.units;
            if immune_accounts.contains(&id) {
                immune_wave_units += balance.units;
                continue;
            }
            decayable_wave_units += balance.units;
            decayable_accounts += 1;
            account_summaries.push(serde_json::json!({
                "account": hex::encode(id.0),
                "domain": hex::encode(*domain_entry.key()),
                "wave_units": balance.units.to_string(),
                "last_active_tick": balance.last_active_tick,
                "last_decay_tick": balance.last_decay_tick,
            }));
        }
    }

    // Sort decayable accounts by balance descending and take top 50.
    account_summaries.sort_by(|a, b| {
        let a_units = a["wave_units"].as_str().unwrap_or("0").parse::<u128>().unwrap_or(0);
        let b_units = b["wave_units"].as_str().unwrap_or("0").parse::<u128>().unwrap_or(0);
        b_units.cmp(&a_units)
    });
    account_summaries.truncate(50);

    Json(serde_json::json!({
        "total_accounts": total_accounts,
        "total_wave_units": total_wave_units.to_string(),
        "immune_wave_units": immune_wave_units.to_string(),
        "non_decaying_wave_units": non_decaying_wave_units.to_string(),
        "decayable_wave_units": decayable_wave_units.to_string(),
        "decayable_accounts": decayable_accounts,
        "immune_account_count": immune_accounts.len(),
        "metabolic_burned": state.oscillator.metabolic_engine.total_burned().to_string(),
        "supply_burned": state.oscillator.supply_tracker.burned().to_string(),
        "top_decayable_accounts": account_summaries,
    }))
}

/// Best-effort decode of a 32-byte domain id as an ASCII name (trailing zero
/// padding stripped).  Falls back to the hex id when it is not printable.
fn domain_display_name(domain: &[u8; 32]) -> String {
    let trimmed: Vec<u8> = domain.iter().copied().take_while(|b| *b != 0).collect();
    if !trimmed.is_empty() && trimmed.iter().all(|b| b.is_ascii_graphic() || *b == b' ') {
        String::from_utf8_lossy(&trimmed).to_string()
    } else {
        hex::encode(domain)
    }
}

fn fee_policy_json(policy: &crate::consensus::domain::FeePolicy) -> serde_json::Value {
    use crate::consensus::domain::FeePolicy;
    match policy {
        FeePolicy::Flat(fee) => serde_json::json!({
            "type": "flat",
            "label": "Flat fee",
            "fee": fee.to_string(),
        }),
        FeePolicy::Percentage(bp) => serde_json::json!({
            "type": "percentage",
            "label": "Percentage fee",
            "basis_points": bp,
            "percent": (*bp as f64) / 100.0,
        }),
        FeePolicy::MetabolicOnly => serde_json::json!({
            "type": "metabolic_only",
            "label": "Metabolic only",
        }),
    }
}

fn domain_policy_json(
    policy: &crate::consensus::domain::DomainPolicy,
    shift_count: usize,
) -> serde_json::Value {
    use crate::consensus::domain::StatefulOrdering;
    serde_json::json!({
        "id": hex::encode(policy.domain),
        "name": domain_display_name(&policy.domain),
        "commutative": policy.commutative,
        "stateful": policy.stateful,
        "ordering": match policy.ordering {
            StatefulOrdering::Causal => "causal",
            StatefulOrdering::Strict => "strict",
        },
        "finalization_depth": policy.finalization_depth,
        "metabolic_lambda_ppm": policy.metabolic_lambda_ppm,
        "fee_policy": fee_policy_json(&policy.fee_policy),
        "shift_count": shift_count,
    })
}

/// Count recent shifts tagged with a given domain id (hex).
fn domain_shift_count(state: &ApiState, domain_hex: &str) -> usize {
    state
        .recent_shifts
        .lock()
        .unwrap()
        .iter()
        .filter(|s| s.domain.as_deref() == Some(domain_hex))
        .count()
}

async fn get_domains(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<StateQuery>,
) -> impl IntoResponse {
    wait_for_min_tick(&state, query.min_tick).await;
    let policies = state.oscillator.domain_registry.read().unwrap().all();
    let domains: Vec<serde_json::Value> = policies
        .iter()
        .map(|p| {
            let count = domain_shift_count(&state, &hex::encode(p.domain));
            domain_policy_json(p, count)
        })
        .collect();
    Json(serde_json::json!({ "domains": domains }))
}

async fn get_domain(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Query(query): Query<StateQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    wait_for_min_tick(&state, query.min_tick).await;
    let domain = parse_domain(&id)?;
    let policy = state
        .oscillator
        .domain_registry
        .read()
        .unwrap()
        .get(&domain)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)?;
    let domain_hex = hex::encode(policy.domain);
    let count = domain_shift_count(&state, &domain_hex);

    // Include the most recent shifts in this domain for the detail page.
    let recent: Vec<RecentShift> = state
        .recent_shifts
        .lock()
        .unwrap()
        .iter()
        .filter(|s| s.domain.as_deref() == Some(domain_hex.as_str()))
        .take(25)
        .cloned()
        .collect();

    let mut body = domain_policy_json(&policy, count);
    if let serde_json::Value::Object(ref mut map) = body {
        map.insert("recent_shifts".to_string(), serde_json::json!(recent));
    }
    Ok(Json(body))
}

#[derive(Deserialize)]
struct RegisterDomainRequest {
    domain: String,
    commutative: bool,
    stateful: bool,
    ordering: String,
    finalization_depth: u64,
    metabolic_lambda_ppm: u64,
    fee_policy: String,
    fee_amount: Option<String>,
    registrant: String,
    signature: String,
}

async fn register_domain(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<RegisterDomainRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let domain = parse_domain(&req.domain).map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid domain: {:?}", e)))?;
    let registrant = parse_account(&req.registrant)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid registrant: {}", e)))?;
    let signature = hex::decode(&req.signature)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid signature hex".to_string()))?;

    let ordering = match req.ordering.as_str() {
        "causal" => StatefulOrdering::Causal,
        "strict" => StatefulOrdering::Strict,
        other => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("unknown ordering mode: {}", other),
            ));
        }
    };

    let fee_policy = match req.fee_policy.as_str() {
        "metabolic_only" => FeePolicy::MetabolicOnly,
        "flat" => {
            let amount = req
                .fee_amount
                .as_ref()
                .ok_or((StatusCode::BAD_REQUEST, "flat fee requires fee_amount".to_string()))?
                .parse::<u128>()
                .map_err(|_| (StatusCode::BAD_REQUEST, "invalid fee_amount".to_string()))?;
            FeePolicy::Flat(amount)
        }
        "percentage" => {
            let bp = req
                .fee_amount
                .as_ref()
                .ok_or((StatusCode::BAD_REQUEST, "percentage fee requires fee_amount basis points".to_string()))?
                .parse::<u64>()
                .map_err(|_| (StatusCode::BAD_REQUEST, "invalid fee_amount".to_string()))?;
            FeePolicy::Percentage(bp)
        }
        other => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("unknown fee_policy: {}", other),
            ));
        }
    };

    let policy = DomainPolicy::new(
        domain,
        req.commutative,
        req.stateful,
        ordering,
        req.finalization_depth,
        req.metabolic_lambda_ppm,
        fee_policy,
    )
    .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    // Verify the registrant signed the canonical domain policy bytes.
    let signing_bytes = domain_registration_signing_bytes(&policy,
        crate::consensus::domain::domain_reservation_fee_units(),
    );
    let registry = state.registry.read().unwrap();
    let pk = registry
        .get(&registrant)
        .ok_or((StatusCode::UNAUTHORIZED, "unknown registrant".to_string()))?;
    let sig = Signature::from_slice(&signature)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid signature bytes".to_string()))?;
    if !KeyPair::verify(pk, &signing_bytes, &sig) {
        return Err((StatusCode::UNAUTHORIZED, "invalid signature".to_string()));
    }
    drop(registry);

    state
        .oscillator
        .register_domain(policy, registrant)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    Ok(Json(serde_json::json!({
        "status": "registered",
        "domain": req.domain,
        "reservation_fee": crate::consensus::domain::DOMAIN_RESERVATION_FEE_WAVE.to_string(),
    })))
}

fn domain_registration_signing_bytes(policy: &DomainPolicy, fee: u128) -> Vec<u8> {
    let mut buf = Vec::with_capacity(256);
    buf.extend_from_slice(b"FLUIDIC:REGISTER_DOMAIN:v1");
    buf.extend_from_slice(&policy.domain);
    buf.push(policy.commutative as u8);
    buf.push(policy.stateful as u8);
    buf.push(match policy.ordering {
        StatefulOrdering::Causal => 0,
        StatefulOrdering::Strict => 1,
    });
    buf.extend_from_slice(&policy.finalization_depth.to_le_bytes());
    buf.extend_from_slice(&policy.metabolic_lambda_ppm.to_le_bytes());
    buf.extend_from_slice(&fee.to_le_bytes());
    buf
}

async fn get_recent_shifts(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<StateQuery>,
) -> impl IntoResponse {
    wait_for_min_tick(&state, query.min_tick).await;
    let shifts = state.recent_shifts.lock().unwrap().clone();
    Json(serde_json::json!({ "shifts": shifts }))
}

async fn shift_status(
    State(state): State<Arc<ApiState>>,
    Path(hash_hex): Path<String>,
    Query(query): Query<StateQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    wait_for_min_tick(&state, query.min_tick).await;
    let bytes = hex::decode(&hash_hex).map_err(|_| StatusCode::BAD_REQUEST)?;
    if bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&bytes);

    let current_tick = state.oscillator.synthesis_tick.load(std::sync::atomic::Ordering::SeqCst);

    let status = match state.shift_status(&hash) {
        Some(ShiftStatus::Accepted) => {
            let inserted = state.oscillator.dag.lock().unwrap()
                .nodes.get(&hash).map(|n| n.inserted_at_tick);
            let confirmations = inserted.map(|t| current_tick.saturating_sub(t)).unwrap_or(0);
            serde_json::json!({
                "hash": hash_hex,
                "status": "accepted",
                "error": null,
                "synthesis_tick": current_tick,
                "confirmations": confirmations,
            })
        }
        Some(ShiftStatus::Finalized) => serde_json::json!({
            "hash": hash_hex,
            "status": "finalized",
            "error": null,
            "synthesis_tick": current_tick,
            "confirmations": VectorClockDag::FINALIZATION_DEPTH,
        }),
        Some(ShiftStatus::Rejected(err)) => serde_json::json!({
            "hash": hash_hex,
            "status": "rejected",
            "error": dag_error_name(&err),
            "synthesis_tick": current_tick,
            "confirmations": 0,
        }),
        None => serde_json::json!({
            "hash": hash_hex,
            "status": "unknown",
            "error": null,
            "synthesis_tick": current_tick,
            "confirmations": 0,
        }),
    };

    Ok(Json(status))
}

fn dag_error_name(err: &DagError) -> &'static str {
    match err {
        DagError::MissingPredecessor(_) => "missing_predecessor",
        DagError::InvalidSignature(_) => "invalid_signature",
        DagError::InsufficientBalance(_) => "insufficient_balance",
        DagError::DoubleSpend(_) => "double_spend",
        DagError::CausalCycle(_) => "causal_cycle",
        DagError::EntanglementThresholdNotMet(_) => "entanglement_threshold_not_met",
    }
}

fn rejection_reason_name(reason: &RejectionReason) -> String {
    match reason {
        RejectionReason::InvalidSignature => "invalid_signature".to_string(),
        RejectionReason::MissingPredecessor(h) => format!("missing_predecessor:{}", hex::encode(h)),
        RejectionReason::InsufficientBalance => "insufficient_balance".to_string(),
        RejectionReason::DoubleSpend => "double_spend".to_string(),
        RejectionReason::CausalCycle => "causal_cycle".to_string(),
        RejectionReason::EntanglementThresholdNotMet => "entanglement_threshold_not_met".to_string(),
    }
}

async fn get_rejection_proof(
    State(state): State<Arc<ApiState>>,
    Path(hash_hex): Path<String>,
    Query(query): Query<StateQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    wait_for_min_tick(&state, query.min_tick).await;
    let bytes = hex::decode(&hash_hex).map_err(|_| (StatusCode::BAD_REQUEST, "invalid hash hex".to_string()))?;
    if bytes.len() != 32 {
        return Err((StatusCode::BAD_REQUEST, "hash must be 32 bytes".to_string()));
    }
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&bytes);

    let proof = state
        .oscillator
        .dag
        .lock()
        .unwrap()
        .rejection_proofs
        .get(&hash)
        .cloned();

    match proof {
        Some(p) => Ok(Json(serde_json::json!({
            "hash": hash_hex,
            "found": true,
            "reason": rejection_reason_name(&p.reason),
            "rejected_at_tick": p.rejected_at_tick,
            "operator_id": p.operator_id.to_string(),
            "signature": hex::encode(&p.signature),
            "signing_bytes": hex::encode(p.signing_bytes()),
        }))),
        None => Ok(Json(serde_json::json!({
            "hash": hash_hex,
            "found": false,
            "reason": null,
            "rejected_at_tick": null,
            "operator_id": null,
            "signature": null,
            "signing_bytes": null,
        }))),
    }
}

#[derive(Deserialize)]
struct RecentTicksQuery {
    #[serde(default)]
    min_tick: Option<u64>,
    #[serde(default)]
    limit: Option<usize>,
}

async fn get_recent_ticks(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<RecentTicksQuery>,
) -> impl IntoResponse {
    wait_for_min_tick(&state, query.min_tick).await;
    let limit = query.limit.unwrap_or(20).min(100);
    let certs = state.oscillator.certificates.read().unwrap();
    let mut ticks: Vec<_> = certs
        .iter()
        .map(|(tick, cert)| {
            let finalized = state.oscillator.check_quorum(*tick).is_some();
            serde_json::json!({
                "tick": cert.tick,
                "hash": hex::encode(cert.hash()),
                "operator": cert.operator.to_string(),
                "commutative_applied": cert.commutative_applied,
                "stateful_applied": cert.stateful_applied,
                "evm_applied": cert.evm_applied,
                "roots": {
                    "commutative": hex::encode(cert.commutative_root),
                    "stateful": hex::encode(cert.stateful_root),
                    "balances": hex::encode(cert.balances_root),
                    "stake": hex::encode(cert.stake_root),
                    "reward": hex::encode(cert.reward_root),
                },
                "finalized": finalized,
            })
        })
        .collect();
    // Sort descending by tick.
    ticks.sort_by(|a, b| {
        let at = a.get("tick").and_then(|v| v.as_u64()).unwrap_or(0);
        let bt = b.get("tick").and_then(|v| v.as_u64()).unwrap_or(0);
        bt.cmp(&at)
    });
    ticks.truncate(limit);
    Json(serde_json::json!({ "ticks": ticks }))
}

async fn get_tick(
    State(state): State<Arc<ApiState>>,
    Path(tick): Path<u64>,
    Query(query): Query<StateQuery>,
) -> impl IntoResponse {
    wait_for_min_tick(&state, query.min_tick).await;
    let certs = state.oscillator.certificates.read().unwrap();
    match certs.get(&tick) {
        Some(cert) => {
            let finalized = state.oscillator.check_quorum(tick).is_some();
            Json(serde_json::json!({
                "tick": cert.tick,
                "hash": hex::encode(cert.hash()),
                "operator": cert.operator.to_string(),
                "commutative_applied": cert.commutative_applied,
                "stateful_applied": cert.stateful_applied,
                "evm_applied": cert.evm_applied,
                "roots": {
                    "commutative": hex::encode(cert.commutative_root),
                    "stateful": hex::encode(cert.stateful_root),
                    "balances": hex::encode(cert.balances_root),
                    "stake": hex::encode(cert.stake_root),
                    "reward": hex::encode(cert.reward_root),
                },
                "finalized": finalized,
            }))
            .into_response()
        }
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "tick not found" }))).into_response(),
    }
}

#[derive(Deserialize)]
struct SyncShiftsQuery {
    #[serde(default)]
    from_tick: u64,
    #[serde(default)]
    limit: Option<usize>,
}

async fn get_sync_state(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    let current_tick = state
        .oscillator
        .synthesis_tick
        .load(std::sync::atomic::Ordering::SeqCst);

    // Sync snapshots can grow very large over time.  Keep the response small
    // enough to serialize and transfer reliably: drop zero-balance accounts and
    // only include certificates from the most recent ticks (enough to bridge a
    // joining node past finalization depth without sending the whole chain).
    const SYNC_CERTIFICATE_LOOKBACK: u64 = 1_000;

    let balances: std::collections::HashMap<String, serde_json::Value> = {
        let field = state.oscillator.wave_field.lock().unwrap();
        field
            .domains
            .get(&crate::crypto::DEFAULT_DEX_DOMAIN)
            .map(|dex| {
                dex.accounts
                    .iter()
                    .filter(|entry| entry.value().balance.units > 0)
                    .map(|entry| {
                        let acc = *entry.key();
                        let bal = &entry.value().balance;
                        (
                            hex::encode(acc.0),
                            serde_json::json!({
                                "units": bal.units.to_string(),
                                "last_decay_tick": bal.last_decay_tick,
                                "decays": bal.decays,
                                "last_active_tick": bal.last_active_tick,
                            }),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    };

    let pools: std::collections::HashMap<String, String> = {
        let field = state.oscillator.wave_field.lock().unwrap();
        field
            .domains
            .get(&crate::crypto::DEFAULT_DEX_DOMAIN)
            .map(|dex| {
                dex.pools
                    .iter()
                    .filter(|entry| entry.value().units > 0)
                    .map(|entry| (hex::encode(entry.key()), entry.value().units.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    };

    let registry: std::collections::HashMap<String, String> = {
        let reg = state.registry.read().unwrap();
        reg.iter()
            .map(|(acc, pk)| (hex::encode(acc.0), hex::encode(pk.to_bytes())))
            .collect()
    };

    let stake_table = state.oscillator.stake_table.to_snapshot();

    let certificates: Vec<serde_json::Value> = {
        let certs = state.oscillator.certificates.read().unwrap();
        let min_tick = current_tick.saturating_sub(SYNC_CERTIFICATE_LOOKBACK);
        certs
            .iter()
            .filter(|(tick, _cert)| **tick >= min_tick)
            .map(|(_tick, cert)| serde_json::to_value(cert).unwrap_or(serde_json::Value::Null))
            .filter(|v| !v.is_null())
            .collect()
    };

    let total_burned = *state
        .oscillator
        .metabolic_engine
        .total_burned
        .lock()
        .unwrap();

    Json(serde_json::json!({
        "synthesis_tick": current_tick,
        "block_hash": hex::encode(block_hash_for(current_tick).as_bytes()),
        "balances": balances,
        "pools": pools,
        "registry": registry,
        "stake_table": stake_table,
        "certificates": certificates,
        "total_burned": total_burned.to_string(),
    }))
}

async fn get_sync_shifts(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<SyncShiftsQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(1000).min(10_000);

    let shifts: Vec<serde_json::Value> = {
        let dag = state.oscillator.dag.lock().unwrap();
        dag.finalized_shifts_since(query.from_tick)
            .into_iter()
            .take(limit)
            .map(|node| {
                serde_json::json!({
                    "hash": hex::encode(node.hash),
                    "domain": hex::encode(node.shift.domain),
                    "from": hex::encode(node.shift.from.0),
                    "to": hex::encode(node.shift.to.0),
                    "amount": node.shift.amount.to_string(),
                    "nonce": node.shift.nonce,
                    "inserted_at_tick": node.inserted_at_tick,
                    "finalized_at_tick": node.finalized_at_tick,
                    "timestamp_ns": node.shift.timestamp_ns,
                    "predecessors": node.shift.predecessors.iter().map(|h| hex::encode(h)).collect::<Vec<_>>(),
                    "signature": hex::encode(&node.shift.signature),
                })
            })
            .collect()
    };

    let receipts: Vec<serde_json::Value> = {
        let pool = state.oscillator.evm_pool.lock().unwrap();
        pool.receipts
            .values()
            .filter(|r| r.block_number >= query.from_tick)
            .take(limit)
            .map(|r| {
                serde_json::json!({
                    "transactionHash": hex::encode(r.transaction_hash.as_bytes()),
                    "transactionIndex": r.transaction_index,
                    "blockNumber": r.block_number,
                    "blockHash": hex::encode(r.block_hash.as_bytes()),
                    "from": hex::encode(r.from.as_bytes()),
                    "to": r.to.map(|a| hex::encode(a.as_bytes())),
                    "contractAddress": r.contract_address.map(|a| hex::encode(a.as_bytes())),
                    "gasUsed": r.gas_used,
                    "cumulativeGasUsed": r.cumulative_gas_used,
                    "status": r.status,
                })
            })
            .collect()
    };

    Json(serde_json::json!({
        "from_tick": query.from_tick,
        "shifts": shifts,
        "receipts": receipts,
    }))
}

fn gossip_psk() -> Option<[u8; 32]> {
    std::env::var("FLUIDIC_PSK")
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
        })
}

async fn ws_handler(
    headers: HeaderMap,
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiState>>,
) -> impl IntoResponse {
    ws.protocols([&"fluidic-gossip" as &str,
    ])
        .on_upgrade(move |socket| async move {
            let is_gossip = headers
                .get("sec-websocket-protocol")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.contains("fluidic-gossip"))
                .unwrap_or(false);
            if is_gossip {
                crate::network::handle_gossip_socket(socket, state, gossip_psk()).await
            } else {
                crate::api::websocket::handle_socket(socket, state).await
            }
        })
}
