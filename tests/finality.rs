use fluidic::consensus::dag::{DagError, ShiftStatus, VectorClockDag};
use fluidic::consensus::Oscillator;
use fluidic::crypto::keys::KeyPair;
use fluidic::crypto::{Signal, StatefulShift, VectorClock, DEFAULT_DEX_DOMAIN};
use std::collections::HashMap;

#[test]
fn shift_reaches_finalized_after_confirmation_depth() {
    let osc = Oscillator::new([1u8; 32], 64);
    let alice = KeyPair::generate();
    let bob = KeyPair::generate();
    let balance = 1_000_000_000_000_000u128;
    osc.seed_account(alice.account_id(), balance);

    let mut vc = VectorClock::new();
    vc.tick(osc.id);
    let shift = StatefulShift::new(&alice, DEFAULT_DEX_DOMAIN, bob.account_id(), 1_000_000, vc, vec![], 1, 0);
    let hash = shift.hash();

    let mut registry = HashMap::new();
    registry.insert(alice.account_id(), alice.public_key());

    osc.ingest(Signal::Stateful(shift), &registry).unwrap();

    // First synthesis accepts the shift.
    osc.synthesize(&registry);
    assert_eq!(
        osc.dag.lock().unwrap().shift_status(&hash),
        Some(ShiftStatus::Accepted)
    );

    // Run enough subsequent ticks to finalize it.
    for _ in 0..VectorClockDag::FINALIZATION_DEPTH {
        osc.synthesize(&registry);
    }

    assert_eq!(
        osc.dag.lock().unwrap().shift_status(&hash),
        Some(ShiftStatus::Finalized)
    );
}

#[test]
fn double_spend_is_rejected_and_first_shift_finalizes() {
    let osc = Oscillator::new([1u8; 32], 64);
    let alice = KeyPair::generate();
    let bob = KeyPair::generate();
    let charlie = KeyPair::generate();
    let balance = 1_000_000_000_000u128;
    osc.seed_account(alice.account_id(), balance);

    // Two concurrent shifts that together overdraw alice.
    let amount = balance / 2 + 1;

    let mut vc_a = VectorClock::new();
    vc_a.tick([1u8; 32]);
    let shift_a = StatefulShift::new(&alice, DEFAULT_DEX_DOMAIN, bob.account_id(), amount, vc_a, vec![], 1, 0);
    let hash_a = shift_a.hash();

    let mut vc_b = VectorClock::new();
    vc_b.tick([2u8; 32]);
    let shift_b = StatefulShift::new(&alice, DEFAULT_DEX_DOMAIN, charlie.account_id(), amount, vc_b, vec![], 2, 0);
    let hash_b = shift_b.hash();

    let mut registry = HashMap::new();
    registry.insert(alice.account_id(), alice.public_key());

    osc.ingest(Signal::Stateful(shift_a), &registry).unwrap();
    osc.ingest(Signal::Stateful(shift_b), &registry).unwrap();

    // Run synthesize enough times for finalization.
    for _ in 0..=VectorClockDag::FINALIZATION_DEPTH {
        osc.synthesize(&registry);
    }

    let dag = osc.dag.lock().unwrap();
    let statuses = [
        dag.shift_status(&hash_a).unwrap(),
        dag.shift_status(&hash_b).unwrap(),
    ];
    let finalized_count = statuses.iter().filter(|s| matches!(s, ShiftStatus::Finalized)).count();
    let rejected_count = statuses
        .iter()
        .filter(|s| matches!(s, ShiftStatus::Rejected(DagError::DoubleSpend(_))))
        .count();
    assert_eq!(finalized_count, 1, "exactly one concurrent shift should finalize");
    assert_eq!(rejected_count, 1, "exactly one concurrent shift should be rejected as double-spend");

    // Ensure conservation: alice cannot end up with a negative balance
    // and the total issued value does not exceed the seed.
    let final_alice = osc
        .wave_field
        .lock()
        .unwrap()
        .account_balance(alice.account_id())
        .units;
    assert!(final_alice <= balance);
}
