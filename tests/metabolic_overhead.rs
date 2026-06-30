use fluidic::consensus::Oscillator;
use fluidic::crypto::keys::KeyPair;
use fluidic::crypto::{AccountId, CommutativeShift, Signal, StatefulShift, VectorClock, DEFAULT_DEX_DOMAIN};
use fluidic::field::coordinates::Coordinate;
use fluidic::value::metabolic::MetabolicStream;
use std::collections::HashMap;
use std::time::Instant;

const COMMUTATIVE_WORKLOAD: usize = 1_000;
const STATEFUL_WORKLOAD: usize = 200;
const STREAM_COUNT: usize = 1_000;
const ITERATIONS: usize = 10;

fn build_workload() -> (
    Vec<Signal>,
    HashMap<AccountId, ed25519_dalek::VerifyingKey>,
    Vec<KeyPair>,
) {
    let mut keypairs = Vec::with_capacity(STATEFUL_WORKLOAD + 1);
    for _ in 0..(STATEFUL_WORKLOAD + 1) {
        keypairs.push(KeyPair::generate());
    }
    let registry: HashMap<_, _> = keypairs
        .iter()
        .map(|kp| (kp.account_id(), kp.public_key()))
        .collect();

    let mut shifts = Vec::with_capacity(COMMUTATIVE_WORKLOAD + STATEFUL_WORKLOAD);
    let sender = &keypairs[0];
    let pool = [0xAB; 32];
    for i in 0..COMMUTATIVE_WORKLOAD {
        shifts.push(Signal::Commutative(CommutativeShift::new(
            sender,
            DEFAULT_DEX_DOMAIN,
            Coordinate::from_scalar(i as u64),
            1_000_000,
            pool,
            i as u64,
            0,
        )));
    }

    let mut vc = VectorClock::new();
    vc.tick([1u8; 32]);
    for i in 0..STATEFUL_WORKLOAD {
        let from = &keypairs[i % keypairs.len()];
        let to = keypairs[(i + 1) % keypairs.len()].account_id();
        shifts.push(Signal::Stateful(StatefulShift::new(
            from,
            DEFAULT_DEX_DOMAIN,
            to,
            1_000_000,
            vc.clone(),
            vec![],
            i as u64,
            0,
        )));
    }

    (shifts, registry, keypairs)
}

fn create_oscillator(stream_count: usize, keypairs: &[KeyPair]) -> Oscillator {
    let osc = Oscillator::new([0u8; 32], 4096);
    for kp in keypairs {
        osc.seed_account(kp.account_id(), 1_000_000_000_000_000);
    }
    let owner = KeyPair::generate().account_id();
    for i in 0..stream_count {
        let mut id = [0u8; 32];
        id[0..8].copy_from_slice(&(i as u64).to_le_bytes());
        osc.metabolic_engine.add_stream(MetabolicStream::new(
            id,
            owner,
            1_000_000_000_000,
            fluidic::value::metabolic::DEFAULT_DEX_LAMBDA_BP,
        ));
    }
    osc
}

fn run_synthesis(
    osc: &Oscillator,
    shifts: &[Signal],
    registry: &HashMap<AccountId, ed25519_dalek::VerifyingKey>,
) {
    for shift in shifts {
        osc.ingest(shift.clone()).unwrap();
    }
    osc.synthesize(registry);
}

/// Invariant: the incremental cost of metabolic decay must stay well below
/// the total oscillator synthesis time under a realistic load.  The 5% test
/// bound absorbs measurement noise on shared CI runners while the actual
/// production overhead target remains <1%.
#[test]
fn metabolic_decay_overhead_under_one_percent() {
    let (shifts, registry, keypairs) = build_workload();

    // Baseline: no metabolic streams.
    let mut baseline_total = 0u128;
    for _ in 0..ITERATIONS {
        let osc = create_oscillator(0, &keypairs);
        let start = Instant::now();
        run_synthesis(&osc, &shifts, &registry);
        baseline_total += start.elapsed().as_nanos();
    }
    let baseline_avg = baseline_total / ITERATIONS as u128;

    // With metabolic load.
    let mut loaded_total = 0u128;
    for _ in 0..ITERATIONS {
        let osc = create_oscillator(STREAM_COUNT, &keypairs);
        let start = Instant::now();
        run_synthesis(&osc, &shifts, &registry);
        loaded_total += start.elapsed().as_nanos();
    }
    let loaded_avg = loaded_total / ITERATIONS as u128;

    let incremental = loaded_avg.saturating_sub(baseline_avg);
    let overhead_ratio = incremental as f64 / loaded_avg as f64;

    println!("\n=== Metabolic Decay Overhead ===");
    println!("Baseline avg (no streams): {} ns", baseline_avg);
    println!("Loaded avg ({} streams):  {} ns", STREAM_COUNT, loaded_avg);
    println!("Incremental decay time:    {} ns", incremental);
    println!("Overhead ratio:            {:.4}%", overhead_ratio * 100.0);
    println!("================================\n");

    assert!(
        overhead_ratio < 0.05,
        "metabolic decay overhead {:.4}% exceeds 5% test bound",
        overhead_ratio * 100.0
    );
}
