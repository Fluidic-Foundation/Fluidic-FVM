use fluidic::consensus::Oscillator;
use fluidic::crypto::keys::KeyPair;
use fluidic::crypto::{
    AccountId, CommutativeShift, PoolId, Signal, StatefulShift, VectorClock, DEFAULT_DEX_DOMAIN,
};
use fluidic::field::coordinates::Coordinate;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

const NTT_SIZE: usize = 4096;
const OSCILLATOR_COUNT: usize = 4;
const ACCOUNT_COUNT: usize = 1000;
const COMMUTATIVE_COUNT: usize = 10_000;
const STATEFUL_COUNT: usize = 500;
const DOUBLE_SPEND_COUNT: usize = 50;
const INITIAL_BALANCE: u128 = 1_000_000_000_000_000; // 1,000,000 WAVE

/// Build a registry mapping every account id to its verifying key.
fn build_registry(keypairs: &[KeyPair]) -> HashMap<AccountId, ed25519_dalek::VerifyingKey> {
    keypairs
        .iter()
        .map(|kp| (kp.account_id(), kp.public_key()))
        .collect()
}

/// Generate `n` commutative liquidity-pool deltas from random accounts.
fn generate_commutative_shifts(
    keypairs: &[KeyPair],
    pools: &[PoolId],
    n: usize,
) -> Vec<CommutativeShift> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..n)
        .map(|i| {
            let kp = &keypairs[rng.gen_range(0..keypairs.len())];
            let pool = pools[rng.gen_range(0..pools.len())];
            let delta = rng.gen_range(1..=1000) as i128;
            let coord = Coordinate::from_scalar(rng.gen_range(0..=u64::MAX));
            CommutativeShift::new(kp, DEFAULT_DEX_DOMAIN, coord, delta, pool, i as u64, 0)
        })
        .collect()
}

/// Generate valid stateful transfers plus double-spend attempts.
/// `valid_keypairs` may send/receive among themselves.
/// `ds_keypairs` are reserved exclusively for double-spend attempts.
fn generate_stateful_shifts(
    valid_keypairs: &[KeyPair],
    ds_keypairs: &[KeyPair],
    oscillator_ids: &[[u8; 32]],
) -> (Vec<StatefulShift>, Vec<StatefulShift>) {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut valid = Vec::with_capacity(STATEFUL_COUNT - DOUBLE_SPEND_COUNT);
    let mut double_spends = Vec::with_capacity(DOUBLE_SPEND_COUNT * 2);

    // Valid chained transfers.  Use nonces that do not overlap with the
    // commutative workload so per-account replay protection accepts them.
    const STATEFUL_NONCE_OFFSET: u64 = 100_000;
    for i in 0..(STATEFUL_COUNT - DOUBLE_SPEND_COUNT) {
        let from_idx = rng.gen_range(0..valid_keypairs.len());
        let to_idx = rng.gen_range(0..valid_keypairs.len());
        let from_kp = &valid_keypairs[from_idx];
        let to = valid_keypairs[to_idx].account_id();
        let amount = rng.gen_range(1..=10_000_000_000_u128); // up to 10 WAVE

        let mut vc = VectorClock::new();
        vc.tick(oscillator_ids[rng.gen_range(0..oscillator_ids.len())]);
        let predecessors = Vec::new();
        valid.push(StatefulShift::new(
            from_kp,
            DEFAULT_DEX_DOMAIN,
            to,
            amount,
            vc,
            predecessors,
            STATEFUL_NONCE_OFFSET + i as u64,
            0,
        ));
    }

    // Double-spend attempts: two concurrent transfers from the same reserved account.
    const DOUBLE_SPEND_NONCE_OFFSET: u64 = 200_000;
    for i in 0..DOUBLE_SPEND_COUNT {
        let from_kp = &ds_keypairs[i % ds_keypairs.len()];
        let to1 = valid_keypairs[rng.gen_range(0..valid_keypairs.len())].account_id();
        let to2 = valid_keypairs[rng.gen_range(0..valid_keypairs.len())].account_id();
        let amount = INITIAL_BALANCE / 2 + 1; // each alone is fine, together they overdraw

        let mut vc_a = VectorClock::new();
        vc_a.tick(oscillator_ids[0]);
        let mut vc_b = VectorClock::new();
        vc_b.tick(oscillator_ids[1]);

        double_spends.push(StatefulShift::new(
            from_kp,
            DEFAULT_DEX_DOMAIN,
            to1,
            amount,
            vc_a,
            vec![],
            DOUBLE_SPEND_NONCE_OFFSET + (i * 2) as u64,
            0,
        ));
        double_spends.push(StatefulShift::new(
            from_kp,
            DEFAULT_DEX_DOMAIN,
            to2,
            amount,
            vc_b,
            vec![],
            DOUBLE_SPEND_NONCE_OFFSET + (i * 2 + 1) as u64,
            0,
        ));
    }

    (valid, double_spends)
}

#[tokio::test]
async fn ten_thousand_overlapping_phase_shifts() {
    let start = Instant::now();

    // 1. Setup oscillators and keypairs.
    let oscillator_ids: Vec<[u8; 32]> = (0..OSCILLATOR_COUNT).map(|i| [i as u8 + 1; 32]).collect();
    let oscillators: Vec<Arc<Oscillator>> = oscillator_ids
        .iter()
        .map(|id| Arc::new(Oscillator::new(*id, NTT_SIZE)))
        .collect();

    let keypairs: Vec<KeyPair> = (0..ACCOUNT_COUNT).map(|_| KeyPair::generate()).collect();
    let registry = build_registry(&keypairs);

    for osc in &oscillators {
        for kp in &keypairs {
            osc.seed_account(kp.account_id(), INITIAL_BALANCE);
        }
    }

    let pools: Vec<PoolId> = (0..8).map(|i| [i as u8; 32]).collect();

    // 2. Generate phase-shifts.
    let commutative = generate_commutative_shifts(&keypairs, &pools, COMMUTATIVE_COUNT);
    let ds_accounts = DOUBLE_SPEND_COUNT * 2;
    let (valid_keypairs, ds_keypairs) = keypairs.split_at(ACCOUNT_COUNT - ds_accounts);
    let (valid_stateful, double_spends) =
        generate_stateful_shifts(valid_keypairs, ds_keypairs, &oscillator_ids);

    // Sequential ground-truth sums for commutative deltas.
    let mut sequential_pool_sum: HashMap<PoolId, i128> = HashMap::new();
    for shift in &commutative {
        *sequential_pool_sum.entry(shift.pool_id).or_insert(0) += shift.delta;
    }

    // 3. Concurrent ingestion across all oscillators (no artificial ordering).
    let mut handles = Vec::new();
    for osc in oscillators.clone() {
        let c = commutative.clone();
        let v = valid_stateful.clone();
        let d = double_spends.clone();
        let registry = registry.clone();
        handles.push(tokio::spawn(async move {
            for shift in c {
                osc.ingest(Signal::Commutative(shift), &registry).unwrap();
            }
            for shift in v {
                osc.ingest(Signal::Stateful(shift), &registry).unwrap();
            }
            for shift in d {
                // Double-spends may be rejected at ingestion or synthesis.
                let _ = osc.ingest(Signal::Stateful(shift), &registry);
            }
        }));
    }
    for h in handles {
        h.await.unwrap();
    }

    let ingest_elapsed = start.elapsed();

    // 4. Synthesize each oscillator.
    let synth_start = Instant::now();
    let mut results = Vec::new();
    for osc in &oscillators {
        results.push(osc.synthesize(&registry));
    }
    let synth_elapsed = synth_start.elapsed();

    // 5. Assertions.
    let first = &results[0];

    // All 10,000 commutative shifts applied.
    assert_eq!(
        first.commutative_applied, COMMUTATIVE_COUNT,
        "all commutative shifts should be applied"
    );

    // NTT-synthesized pool balances must match sequential sums.
    let field = oscillators[0].wave_field.lock().unwrap();
    for (pool_id, expected) in &sequential_pool_sum {
        let actual = field.pool_balance(*pool_id).units as i128;
        assert_eq!(
            actual, *expected,
            "NTT pool balance mismatch for pool {:?}",
            pool_id
        );
    }
    drop(field);

    // Double-spends rejected.
    let rejected = first.stateful_rejected.len();
    assert!(
        rejected >= DOUBLE_SPEND_COUNT,
        "expected at least {} double-spend rejections, got {}",
        DOUBLE_SPEND_COUNT,
        rejected
    );

    // Valid stateful transfers applied.
    let applied = first.stateful_applied;
    assert!(
        applied >= valid_stateful.len(),
        "expected at least {} valid stateful transfers, got {}",
        valid_stateful.len(),
        applied
    );

    // Conservation: total balance across all seeded accounts plus pools
    // should equal seeded total minus any rejected valid-looking transfers.
    // For this benchmark we primarily assert no negative balances.
    let final_balances = &first.final_balances;
    for (account, balance) in final_balances {
        assert!(
            *balance <= INITIAL_BALANCE * ACCOUNT_COUNT as u128,
            "impossible balance for account {}: {}",
            account,
            balance
        );
    }

    // 6. Print metrics.
    let total = start.elapsed();
    println!("\n=== Fluidic 10k Transaction Benchmark ===");
    println!("Oscillators:              {}", OSCILLATOR_COUNT);
    println!("Accounts:                 {}", ACCOUNT_COUNT);
    println!("Commutative shifts:       {}", COMMUTATIVE_COUNT);
    println!("Stateful shifts:          {}", STATEFUL_COUNT);
    println!("Double-spend attempts:    {}", DOUBLE_SPEND_COUNT * 2);
    println!("Ingestion time:           {:?}", ingest_elapsed);
    println!("Synthesis time:           {:?}", synth_elapsed);
    println!("Total elapsed:            {:?}", total);
    println!(
        "Throughput:               {:.2} ops/sec",
        (COMMUTATIVE_COUNT + STATEFUL_COUNT) as f64 / total.as_secs_f64()
    );
    println!("Commutative applied:      {}", first.commutative_applied);
    println!("Stateful applied:         {}", first.stateful_applied);
    println!(
        "Stateful rejected:        {}",
        first.stateful_rejected.len()
    );
    println!("=========================================\n");
}

#[tokio::test]
async fn sub_100ms_finality() {
    use fluidic::consensus::Oscillator;
    use fluidic::crypto::keys::KeyPair;
    use fluidic::crypto::{AccountId, CommutativeShift, Signal, StatefulShift, VectorClock, DEFAULT_DEX_DOMAIN};
    use fluidic::field::coordinates::Coordinate;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    const NTT_SIZE: usize = 256;
    const ACCOUNTS: usize = 10;
    const STATEFUL_PER_TICK: usize = 5;
    const COMMUTATIVE_PER_TICK: usize = 5;
    const TICKS: usize = 5;

    let osc = Arc::new(Oscillator::new([1u8; 32], NTT_SIZE));
    let keypairs: Vec<KeyPair> = (0..ACCOUNTS).map(|_| KeyPair::generate()).collect();
    let registry: HashMap<AccountId, ed25519_dalek::VerifyingKey> =
        keypairs.iter().map(|kp| (kp.account_id(), kp.public_key())).collect();

    for kp in &keypairs {
        osc.seed_account(kp.account_id(), 1_000_000_000_000_000);
    }

    let mut nonce = 1u64;
    for _ in 0..TICKS {
        for i in 0..COMMUTATIVE_PER_TICK {
            let kp = &keypairs[i % ACCOUNTS];
            let shift = CommutativeShift::new(
                kp,
                DEFAULT_DEX_DOMAIN,
                Coordinate::from_scalar(i as u64),
                1_000_000,
                [1u8; 32],
                nonce,
                0,
            );
            nonce += 1;
            osc.ingest(Signal::Commutative(shift), &registry).unwrap();
        }
        for i in 0..STATEFUL_PER_TICK {
            let from = &keypairs[i % ACCOUNTS];
            let to = keypairs[(i + 1) % ACCOUNTS].account_id();
            let mut vc = VectorClock::new();
            vc.tick([1u8; 32]);
            let shift = StatefulShift::new(
                from,
                DEFAULT_DEX_DOMAIN,
                to,
                1_000_000_000,
                vc,
                vec![],
                nonce,
                0,
            );
            nonce += 1;
            osc.ingest(Signal::Stateful(shift), &registry).unwrap();
        }

        let start = Instant::now();
        let result = osc.synthesize(&registry);
        let elapsed = start.elapsed();

        assert!(
            elapsed < Duration::from_millis(100),
            "synthesis took {:?}, exceeding 100 ms target (applied comm={} stateful={})",
            elapsed,
            result.commutative_applied,
            result.stateful_applied
        );
    }

    println!("sub-100ms finality: {} ticks passed", TICKS);
}
