use fluidic::consensus::Oscillator;
use fluidic::crypto::keys::KeyPair;
use fluidic::crypto::{
    AccountId, IntentConstraint, IntentShift, PhysicalAttestation, PhysicalResourceType, Signal,
    DEFAULT_DEX_DOMAIN,
};
use std::collections::HashMap;

fn registry(keys: &[KeyPair]) -> HashMap<AccountId, ed25519_dalek::VerifyingKey> {
    keys.iter().map(|kp| (kp.account_id(), kp.public_key())).collect()
}

#[test]
fn physical_state_intent_is_matched_and_settled() {
    let osc = Oscillator::new([1u8; 32], 16);
    let owner = KeyPair::generate();
    let depin_node = KeyPair::generate();

    // Seed the owner with enough WAVE to pay the solver reward, resource
    // payment, and the anti-spam intent submission fee.
    osc.seed_account(owner.account_id(), 1_000_000_000_000_000);

    let keys = registry(&[owner.clone(), depin_node.clone()],
    );

    let attestation = PhysicalAttestation::new(
        &depin_node,
        PhysicalResourceType::Compute,
        "eu-west-1a".to_string(),
        8,                           // 8 cores
        1_000_000_000,               // 0.001 WAVE per core per tick
        100,                         // available until tick 100
        1,
        0,
    );

    osc.ingest(Signal::PhysicalAttestation(attestation.clone()), &keys)
        .unwrap();

    assert_eq!(
        osc.pending_physical_attestations.lock().unwrap().len(),
        1,
        "attestation should be queued"
    );

    let intent = IntentShift::new(
        &owner,
        DEFAULT_DEX_DOMAIN,
        50, // deadline_tick
        IntentConstraint::PhysicalResource {
            resource_type: PhysicalResourceType::Compute,
            location_prefix: "eu-west".to_string(),
            min_capacity: 4,
            max_price_per_unit: 1_000_000_000,
            duration_ticks: 10,
        },
        1_000_000_000, // 0.001 WAVE solver reward
        1,
        0,
    );

    osc.ingest(Signal::Intent(intent.clone()), &keys).unwrap();

    // Synthesize at tick 5, well before the deadline and attestation expiry.
    osc.synthesis_tick.store(5, std::sync::atomic::Ordering::SeqCst);
    let result = osc.synthesize(&keys,
    );

    assert_eq!(
        result.physical_intents_matched, 1,
        "physical intent should be matched"
    );
    assert_eq!(
        result.physical_attestations_ingested, 1,
        "attestation ingestion should be reported"
    );
    assert!(
        osc.pending_physical_attestations.lock().unwrap().is_empty(),
        "matched attestation should be removed from the pool"
    );
    assert!(
        osc.pending_intents.lock().unwrap().is_empty(),
        "matched intent should be removed from the pool"
    );

    let field = osc.wave_field.lock().unwrap();
    let publisher_balance = field.account_balance(depin_node.account_id()).units;
    let owner_balance = field.account_balance(owner.account_id()).units;

    let resource_payment = 4u128 * 1_000_000_000 * 10;
    let solver_reward = 1_000_000_000u128;
    let intent_fee = fluidic::consensus::domain::intent_submission_fee_units();
    let expected_publisher_payment = resource_payment + solver_reward;

    assert_eq!(
        publisher_balance, expected_publisher_payment,
        "publisher should receive solver reward plus resource payment"
    );

    let expected_owner_after_all = 1_000_000_000_000_000u128
        .saturating_sub(intent_fee)
        .saturating_sub(expected_publisher_payment);
    // Metabolic decay may slightly reduce the owner balance during synthesis;
    // assert it is close to the expected post-payment balance.
    let delta = if owner_balance > expected_owner_after_all {
        owner_balance - expected_owner_after_all
    } else {
        expected_owner_after_all - owner_balance
    };
    assert!(
        delta <= expected_owner_after_all / 100,
        "owner balance {} should be within 1% of expected {}",
        owner_balance,
        expected_owner_after_all
    );
}

#[test]
fn physical_attestation_rejected_when_expired() {
    let osc = Oscillator::new([1u8; 32], 16);
    let depin_node = KeyPair::generate();
    let keys = registry(&[depin_node.clone()],
    );

    osc.synthesis_tick.store(50, std::sync::atomic::Ordering::SeqCst);

    let attestation = PhysicalAttestation::new(
        &depin_node,
        PhysicalResourceType::Storage,
        "nyc".to_string(),
        1000,
        100,
        50, // available_until_tick == current tick
        1,
        0,
    );

    assert!(
        osc.ingest(Signal::PhysicalAttestation(attestation), &keys)
            .is_err(),
        "expired attestation should be rejected"
    );
}

#[test]
fn physical_intent_requeued_when_no_matching_attestation() {
    let osc = Oscillator::new([1u8; 32], 16);
    let owner = KeyPair::generate();
    osc.seed_account(owner.account_id(), 1_000_000_000_000_000);

    let keys = registry(&[owner.clone()],
    );

    let intent = IntentShift::new(
        &owner,
        DEFAULT_DEX_DOMAIN,
        50,
        IntentConstraint::PhysicalResource {
            resource_type: PhysicalResourceType::Bandwidth,
            location_prefix: "asia".to_string(),
            min_capacity: 100,
            max_price_per_unit: 100,
            duration_ticks: 1,
        },
        1_000_000_000,
        1,
        0,
    );

    osc.ingest(Signal::Intent(intent.clone()), &keys).unwrap();

    osc.synthesis_tick.store(5, std::sync::atomic::Ordering::SeqCst);
    let result = osc.synthesize(&keys,
    );

    assert_eq!(result.physical_intents_matched, 0);
    assert_eq!(
        osc.pending_intents.lock().unwrap().len(),
        1,
        "unmatched physical intent should be re-queued"
    );
}
