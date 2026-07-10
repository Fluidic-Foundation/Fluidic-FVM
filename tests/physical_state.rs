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

    let keys = registry(&[owner.clone(), depin_node.clone()]);

    let attestation = PhysicalAttestation::new(
        &depin_node,
        PhysicalResourceType::Compute,
        "eu-west/amsterdam/node-7".to_string(),
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
    let result = osc.synthesize(&keys);

    assert_eq!(
        result.physical_intents_matched, 1,
        "physical intent should be matched"
    );
    assert_eq!(
        result.physical_attestations_ingested, 1,
        "attestation ingestion should be reported"
    );

    // Only 4 of the 8 cores were consumed, so the attestation remains open.
    let remaining = osc.pending_physical_attestations.lock().unwrap();
    assert_eq!(remaining.len(), 1, "attestation should still be open");
    assert_eq!(
        remaining[0].remaining_capacity,
        4,
        "remaining capacity should be 4"
    );
    drop(remaining);

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
fn physical_state_full_capacity_consumption_removes_attestation() {
    let osc = Oscillator::new([1u8; 32], 16);
    let owner = KeyPair::generate();
    let depin_node = KeyPair::generate();

    osc.seed_account(owner.account_id(), 1_000_000_000_000_000);
    let keys = registry(&[owner.clone(), depin_node.clone()]);

    let attestation = PhysicalAttestation::new(
        &depin_node,
        PhysicalResourceType::Storage,
        "us-east/ashburn/node-3".to_string(),
        10,
        100,
        100,
        1,
        0,
    );

    osc.ingest(Signal::PhysicalAttestation(attestation), &keys).unwrap();

    let intent = IntentShift::new(
        &owner,
        DEFAULT_DEX_DOMAIN,
        50,
        IntentConstraint::PhysicalResource {
            resource_type: PhysicalResourceType::Storage,
            location_prefix: "us-east/ashburn".to_string(),
            min_capacity: 10,
            max_price_per_unit: 100,
            duration_ticks: 5,
        },
        1_000_000_000,
        1,
        0,
    );

    osc.ingest(Signal::Intent(intent), &keys).unwrap();
    osc.synthesis_tick.store(5, std::sync::atomic::Ordering::SeqCst);
    let result = osc.synthesize(&keys);

    assert_eq!(result.physical_intents_matched, 1);
    assert!(
        osc.pending_physical_attestations.lock().unwrap().is_empty(),
        "attestation should be removed when capacity is exhausted"
    );

    let field = osc.wave_field.lock().unwrap();
    let publisher_balance = field.account_balance(depin_node.account_id()).units;
    let resource_payment = 10u128 * 100 * 5;
    assert_eq!(publisher_balance, resource_payment + 1_000_000_000);
}

#[test]
fn physical_state_partial_capacity_fills_two_intents() {
    let osc = Oscillator::new([1u8; 32], 16);
    let owner = KeyPair::generate();
    let depin_node = KeyPair::generate();

    osc.seed_account(owner.account_id(), 1_000_000_000_000_000);
    let keys = registry(&[owner.clone(), depin_node.clone()]);

    let attestation = PhysicalAttestation::new(
        &depin_node,
        PhysicalResourceType::Bandwidth,
        "asia/tokyo/node-1".to_string(),
        100,
        50,
        100,
        1,
        0,
    );

    osc.ingest(Signal::PhysicalAttestation(attestation), &keys).unwrap();

    let intent1 = IntentShift::new(
        &owner,
        DEFAULT_DEX_DOMAIN,
        50,
        IntentConstraint::PhysicalResource {
            resource_type: PhysicalResourceType::Bandwidth,
            location_prefix: "asia/tokyo".to_string(),
            min_capacity: 40,
            max_price_per_unit: 50,
            duration_ticks: 2,
        },
        100,
        1,
        0,
    );

    let intent2 = IntentShift::new(
        &owner,
        DEFAULT_DEX_DOMAIN,
        50,
        IntentConstraint::PhysicalResource {
            resource_type: PhysicalResourceType::Bandwidth,
            location_prefix: "asia".to_string(),
            min_capacity: 50,
            max_price_per_unit: 50,
            duration_ticks: 3,
        },
        200,
        2,
        0,
    );

    osc.ingest(Signal::Intent(intent1), &keys).unwrap();
    osc.ingest(Signal::Intent(intent2), &keys).unwrap();

    osc.synthesis_tick.store(5, std::sync::atomic::Ordering::SeqCst);
    let result = osc.synthesize(&keys);

    assert_eq!(result.physical_intents_matched, 2);

    let remaining = osc.pending_physical_attestations.lock().unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].remaining_capacity, 10);
    drop(remaining);

    let field = osc.wave_field.lock().unwrap();
    let publisher_balance = field.account_balance(depin_node.account_id()).units;
    let resource_payment = 40u128 * 50 * 2 + 50u128 * 50 * 3;
    assert_eq!(publisher_balance, resource_payment + 100 + 200);
}

#[test]
fn physical_attestation_rejected_when_expired() {
    let osc = Oscillator::new([1u8; 32], 16);
    let depin_node = KeyPair::generate();
    let keys = registry(&[depin_node.clone()]);

    osc.synthesis_tick.store(50, std::sync::atomic::Ordering::SeqCst);

    let attestation = PhysicalAttestation::new(
        &depin_node,
        PhysicalResourceType::Storage,
        "nyc/new-york/node-1".to_string(),
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
fn physical_attestation_pruned_after_expiry() {
    let osc = Oscillator::new([1u8; 32], 16);
    let owner = KeyPair::generate();
    let depin_node = KeyPair::generate();

    osc.seed_account(owner.account_id(), 1_000_000_000_000_000);
    let keys = registry(&[owner.clone(), depin_node.clone()]);

    let attestation = PhysicalAttestation::new(
        &depin_node,
        PhysicalResourceType::Compute,
        "eu-west/amsterdam/node-7".to_string(),
        8,
        1_000_000_000,
        10, // expires at tick 10
        1,
        0,
    );

    osc.ingest(Signal::PhysicalAttestation(attestation), &keys).unwrap();

    let intent = IntentShift::new(
        &owner,
        DEFAULT_DEX_DOMAIN,
        50,
        IntentConstraint::PhysicalResource {
            resource_type: PhysicalResourceType::Compute,
            location_prefix: "eu-west".to_string(),
            min_capacity: 4,
            max_price_per_unit: 1_000_000_000,
            duration_ticks: 1,
        },
        1_000_000_000,
        1,
        0,
    );

    osc.ingest(Signal::Intent(intent), &keys).unwrap();

    // Synthesize at tick 15, after the attestation has expired.
    osc.synthesis_tick.store(15, std::sync::atomic::Ordering::SeqCst);
    let result = osc.synthesize(&keys);

    assert_eq!(result.physical_intents_matched, 0);
    assert!(
        osc.pending_physical_attestations.lock().unwrap().is_empty(),
        "expired attestation should be pruned"
    );
    assert_eq!(
        osc.pending_intents.lock().unwrap().len(),
        1,
        "unmatched intent should be re-queued"
    );
}

#[test]
fn physical_intent_price_too_high_is_not_matched() {
    let osc = Oscillator::new([1u8; 32], 16);
    let owner = KeyPair::generate();
    let depin_node = KeyPair::generate();

    osc.seed_account(owner.account_id(), 1_000_000_000_000_000);
    let keys = registry(&[owner.clone(), depin_node.clone()]);

    let attestation = PhysicalAttestation::new(
        &depin_node,
        PhysicalResourceType::Compute,
        "eu-west/amsterdam/node-7".to_string(),
        8,
        1_000_000_000,
        100,
        1,
        0,
    );

    osc.ingest(Signal::PhysicalAttestation(attestation), &keys).unwrap();

    let intent = IntentShift::new(
        &owner,
        DEFAULT_DEX_DOMAIN,
        50,
        IntentConstraint::PhysicalResource {
            resource_type: PhysicalResourceType::Compute,
            location_prefix: "eu-west".to_string(),
            min_capacity: 4,
            max_price_per_unit: 500_000_000, // below attestation price
            duration_ticks: 10,
        },
        1_000_000_000,
        1,
        0,
    );

    osc.ingest(Signal::Intent(intent.clone()), &keys).unwrap();

    osc.synthesis_tick.store(5, std::sync::atomic::Ordering::SeqCst);
    let result = osc.synthesize(&keys);

    assert_eq!(result.physical_intents_matched, 0);
    assert_eq!(
        osc.pending_intents.lock().unwrap().len(),
        1,
        "intent should remain open when price is too high"
    );
}

#[test]
fn physical_intent_location_prefix_mismatch_is_not_matched() {
    let osc = Oscillator::new([1u8; 32], 16);
    let owner = KeyPair::generate();
    let depin_node = KeyPair::generate();

    osc.seed_account(owner.account_id(), 1_000_000_000_000_000);
    let keys = registry(&[owner.clone(), depin_node.clone()]);

    let attestation = PhysicalAttestation::new(
        &depin_node,
        PhysicalResourceType::Compute,
        "eu-west/amsterdam/node-7".to_string(),
        8,
        1_000_000_000,
        100,
        1,
        0,
    );

    osc.ingest(Signal::PhysicalAttestation(attestation), &keys).unwrap();

    let intent = IntentShift::new(
        &owner,
        DEFAULT_DEX_DOMAIN,
        50,
        IntentConstraint::PhysicalResource {
            resource_type: PhysicalResourceType::Compute,
            location_prefix: "eu".to_string(), // partial segment; should not match
            min_capacity: 4,
            max_price_per_unit: 1_000_000_000,
            duration_ticks: 10,
        },
        1_000_000_000,
        1,
        0,
    );

    osc.ingest(Signal::Intent(intent.clone()), &keys).unwrap();

    osc.synthesis_tick.store(5, std::sync::atomic::Ordering::SeqCst);
    let result = osc.synthesize(&keys);

    assert_eq!(result.physical_intents_matched, 0);
    assert_eq!(
        osc.pending_intents.lock().unwrap().len(),
        1,
        "intent should remain open when location prefix does not match"
    );
}

#[test]
fn physical_intent_insufficient_owner_balance_fails() {
    let osc = Oscillator::new([1u8; 32], 16);
    let owner = KeyPair::generate();
    let depin_node = KeyPair::generate();

    // Seed the owner with just enough to pay the submission fee and a tiny
    // solver reward, but not enough to cover the resource payment.
    osc.seed_account(owner.account_id(), 2_000_000_000);
    let keys = registry(&[owner.clone(), depin_node.clone()]);

    let attestation = PhysicalAttestation::new(
        &depin_node,
        PhysicalResourceType::Compute,
        "eu-west/amsterdam/node-7".to_string(),
        8,
        1_000_000_000,
        100,
        1,
        0,
    );

    osc.ingest(Signal::PhysicalAttestation(attestation), &keys).unwrap();

    let intent = IntentShift::new(
        &owner,
        DEFAULT_DEX_DOMAIN,
        50,
        IntentConstraint::PhysicalResource {
            resource_type: PhysicalResourceType::Compute,
            location_prefix: "eu-west".to_string(),
            min_capacity: 4,
            max_price_per_unit: 1_000_000_000,
            duration_ticks: 10,
        },
        1_000_000_000,
        1,
        0,
    );

    osc.ingest(Signal::Intent(intent.clone()), &keys).unwrap();

    osc.synthesis_tick.store(5, std::sync::atomic::Ordering::SeqCst);
    let result = osc.synthesize(&keys);

    assert_eq!(result.physical_intents_matched, 0);
    assert_eq!(
        osc.pending_intents.lock().unwrap().len(),
        1,
        "intent should be re-queued when owner has insufficient balance"
    );
}

#[test]
fn physical_intent_multiple_resource_types_do_not_cross_match() {
    let osc = Oscillator::new([1u8; 32], 16);
    let owner = KeyPair::generate();
    let depin_node = KeyPair::generate();

    osc.seed_account(owner.account_id(), 1_000_000_000_000_000);
    let keys = registry(&[owner.clone(), depin_node.clone()]);

    let attestation = PhysicalAttestation::new(
        &depin_node,
        PhysicalResourceType::Storage,
        "eu-west/amsterdam/node-7".to_string(),
        1000,
        100,
        100,
        1,
        0,
    );

    osc.ingest(Signal::PhysicalAttestation(attestation), &keys).unwrap();

    let intent = IntentShift::new(
        &owner,
        DEFAULT_DEX_DOMAIN,
        50,
        IntentConstraint::PhysicalResource {
            resource_type: PhysicalResourceType::Compute,
            location_prefix: "eu-west".to_string(),
            min_capacity: 4,
            max_price_per_unit: 100,
            duration_ticks: 10,
        },
        1_000_000_000,
        1,
        0,
    );

    osc.ingest(Signal::Intent(intent.clone()), &keys).unwrap();

    osc.synthesis_tick.store(5, std::sync::atomic::Ordering::SeqCst);
    let result = osc.synthesize(&keys);

    assert_eq!(result.physical_intents_matched, 0);
    assert_eq!(
        osc.pending_intents.lock().unwrap().len(),
        1,
        "intent should remain open when resource type differs"
    );
}

#[test]
fn physical_publisher_reputation_increases_on_match() {
    let osc = Oscillator::new([1u8; 32], 16);
    let owner = KeyPair::generate();
    let depin_node = KeyPair::generate();

    osc.seed_account(owner.account_id(), 1_000_000_000_000_000);
    let keys = registry(&[owner.clone(), depin_node.clone()]);

    let attestation = PhysicalAttestation::new(
        &depin_node,
        PhysicalResourceType::Energy,
        "eu-west/amsterdam/node-7".to_string(),
        100,
        100,
        100,
        1,
        0,
    );

    osc.ingest(Signal::PhysicalAttestation(attestation), &keys).unwrap();

    let intent = IntentShift::new(
        &owner,
        DEFAULT_DEX_DOMAIN,
        50,
        IntentConstraint::PhysicalResource {
            resource_type: PhysicalResourceType::Energy,
            location_prefix: "eu-west/amsterdam".to_string(),
            min_capacity: 10,
            max_price_per_unit: 100,
            duration_ticks: 1,
        },
        1_000_000_000,
        1,
        0,
    );

    osc.ingest(Signal::Intent(intent), &keys).unwrap();

    osc.synthesis_tick.store(5, std::sync::atomic::Ordering::SeqCst);
    osc.synthesize(&keys);

    let field = osc.wave_field.lock().unwrap();
    let rep = field
        .domains
        .get(&DEFAULT_DEX_DOMAIN)
        .unwrap()
        .accounts
        .get(&depin_node.account_id())
        .unwrap()
        .reputation;
    assert_eq!(rep, 1, "publisher reputation should increase by 1");
}

#[test]
fn physical_attestation_invalid_location_is_rejected() {
    let osc = Oscillator::new([1u8; 32], 16);
    let depin_node = KeyPair::generate();
    let keys = registry(&[depin_node.clone()]);

    let attestation = PhysicalAttestation::new(
        &depin_node,
        PhysicalResourceType::Storage,
        "/eu-west/amsterdam".to_string(), // leading slash is invalid
        1000,
        100,
        100,
        1,
        0,
    );

    assert!(
        osc.ingest(Signal::PhysicalAttestation(attestation), &keys)
            .is_err(),
        "attestation with invalid location should be rejected"
    );
}

#[test]
fn physical_intent_invalid_location_prefix_is_rejected() {
    let osc = Oscillator::new([1u8; 32], 16);
    let owner = KeyPair::generate();

    osc.seed_account(owner.account_id(), 1_000_000_000_000_000);
    let keys = registry(&[owner.clone()]);

    let intent = IntentShift::new(
        &owner,
        DEFAULT_DEX_DOMAIN,
        50,
        IntentConstraint::PhysicalResource {
            resource_type: PhysicalResourceType::Compute,
            location_prefix: "eu-west/".to_string(), // trailing slash is invalid
            min_capacity: 4,
            max_price_per_unit: 1_000_000_000,
            duration_ticks: 10,
        },
        1_000_000_000,
        1,
        0,
    );

    assert!(
        osc.ingest(Signal::Intent(intent), &keys).is_err(),
        "intent with invalid location_prefix should be rejected"
    );
}

#[test]
fn physical_intent_requeued_when_no_matching_attestation() {
    let osc = Oscillator::new([1u8; 32], 16);
    let owner = KeyPair::generate();
    osc.seed_account(owner.account_id(), 1_000_000_000_000_000);

    let keys = registry(&[owner.clone()]);

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
    let result = osc.synthesize(&keys);

    assert_eq!(result.physical_intents_matched, 0);
    assert_eq!(
        osc.pending_intents.lock().unwrap().len(),
        1,
        "unmatched physical intent should be re-queued"
    );
}
