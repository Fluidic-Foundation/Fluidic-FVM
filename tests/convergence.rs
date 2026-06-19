use fluidic::consensus::Oscillator;
use fluidic::crypto::keys::KeyPair;
use fluidic::crypto::{
    CommutativeShift, Signal, StatefulShift, VectorClock, DEFAULT_DEX_DOMAIN,
};
use fluidic::field::coordinates::Coordinate;
use fluidic::operator::{StakeTable, StakingConfig};
use std::collections::HashMap;

/// Build a deterministic, mixed workload of commutative and stateful signals.
fn build_workload(
    keypairs: &[KeyPair],
) -> (Vec<Signal>, HashMap<fluidic::crypto::AccountId, ed25519_dalek::VerifyingKey>) {
    let registry: HashMap<_, _> = keypairs
        .iter()
        .map(|kp| (kp.account_id(), kp.public_key()))
        .collect();

    let mut shifts = Vec::new();
    let pool = [0xAB; 32];

    // Commutative liquidity deltas.
    for i in 0..50 {
        let sender = &keypairs[i % keypairs.len()];
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

    // Stateful transfers chained through the first few accounts.
    let mut vc = VectorClock::new();
    vc.tick([1u8; 32]);
    for i in 0..20 {
        let from = &keypairs[i % 4];
        let to = keypairs[(i + 1) % 4].account_id();
        let mut shift_vc = vc.clone();
        shift_vc.tick([1u8; 32]);
        shifts.push(Signal::Stateful(StatefulShift::new(
            from,
            DEFAULT_DEX_DOMAIN,
            to,
            1_000_000_000,
            shift_vc,
            vec![],
            i as u64,
            0,
        )));
    }

    (shifts, registry)
}

#[test]
fn independent_nodes_converge_to_identical_state() {
    let keypairs: Vec<_> = (0..4).map(|_| KeyPair::generate()).collect();
    let (shifts, registry) = build_workload(&keypairs);

    // Spin up three independent oscillators with distinct identities.
    let nodes: Vec<_> = (0..3)
        .map(|i| Oscillator::new([i as u8; 32], 2048))
        .collect();

    // Seed identical balances.
    for node in &nodes {
        for kp in &keypairs {
            node.seed_account(kp.account_id(), 1_000_000_000_000_000);
        }
    }

    // Feed the same signals in the same order to every node.
    for node in &nodes {
        for shift in &shifts {
            node.ingest(shift.clone()).unwrap();
        }
    }

    // Synthesize enough ticks for finalization.
    let mut results = Vec::new();
    for node in &nodes {
        for _ in 0..=fluidic::consensus::VectorClockDag::FINALIZATION_DEPTH {
            results.push(node.synthesize(&registry));
        }
    }

    // All nodes should produce identical result roots per tick.
    let per_node: Vec<_> = results
        .chunks_exact(fluidic::consensus::VectorClockDag::FINALIZATION_DEPTH as usize + 1)
        .map(|rs| rs.to_vec())
        .collect();

    for i in 1..per_node.len() {
        assert_eq!(
            per_node[0].len(),
            per_node[i].len(),
            "node {} produced a different number of synthesis results",
            i
        );
        for (tick, (a, b)) in per_node[0].iter().zip(per_node[i].iter()).enumerate() {
            assert_eq!(
                a.commutative_applied, b.commutative_applied,
                "tick {tick}: commutative applied diverged between node 0 and node {i}"
            );
            assert_eq!(
                a.stateful_applied, b.stateful_applied,
                "tick {tick}: stateful applied diverged between node 0 and node {i}"
            );
            assert_eq!(
                a.final_balances, b.final_balances,
                "tick {tick}: final balances diverged between node 0 and node {i}"
            );
            assert_eq!(
                a.metabolic_burned, b.metabolic_burned,
                "tick {tick}: metabolic burned diverged between node 0 and node {i}"
            );
        }
    }
}

#[test]
fn synthesis_certificates_are_produced_and_verifiable() {
    let kp = KeyPair::generate();
    let stake = StakeTable::new(StakingConfig { min_stake: 1_000 });
    stake.stake(kp.account_id(), 10_000);
    let osc = Oscillator::new_with_stake([7u8; 32], 512, kp.clone(), stake);

    osc.seed_account(kp.account_id(), 1_000_000_000_000_000);
    assert!(
        osc.stake_table.is_staked(&kp.account_id()),
        "operator should be staked before synthesis"
    );

    let pool = [0xCD; 32];
    osc.ingest(Signal::Commutative(CommutativeShift::new(
        &kp,
        DEFAULT_DEX_DOMAIN,
        Coordinate::from_scalar(1),
        5_000_000,
        pool,
        1,
        0,
    )))
    .unwrap();

    let mut registry = HashMap::new();
    registry.insert(kp.account_id(), kp.public_key());

    let result = osc.synthesize(&registry);
    assert!(result.commutative_applied > 0);

    let certs = osc.certificates.read().unwrap();
    let cert = certs.get(&0).expect("certificate for tick 0");
    assert_eq!(cert.operator, kp.account_id());
    assert!(cert.verify(&kp.public_key()));
}
