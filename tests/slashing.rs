use fluidic::consensus::certificate::{CertificateTracker, SlashingReason, SynthesisCertificate};
use fluidic::crypto::keys::KeyPair;
use std::collections::HashMap;

#[test]
fn conflicting_certificates_slash_operator() {
    let kp = KeyPair::generate();
    let mut registry = HashMap::new();
    registry.insert(kp.account_id(), kp.public_key());

    let tracker = CertificateTracker::new();
    let staked: HashMap<_, _> = [(kp.account_id(), true)].into_iter().collect();
    let mut slashed = false;

    let cert_a = SynthesisCertificate::sign(
        &kp, 0, 1, 0, 0, [1u8; 32], [2u8; 32], [3u8; 32], [7u8; 32], [8u8; 32], [11u8; 32], 0, 0,
    );
    let cert_b = SynthesisCertificate::sign(
        &kp, 0, 2, 0, 0, [4u8; 32], [5u8; 32], [6u8; 32], [9u8; 32], [10u8; 32], [12u8; 32], 0, 0,
    );

    let stake_checker = |op: &fluidic::crypto::AccountId| *staked.get(op).unwrap_or(&false);
    let stake_amount = |_op: &fluidic::crypto::AccountId| 1_000u128;
    let mut slash = |_op| slashed = true;

    assert!(tracker.apply(cert_a, &registry, &stake_checker, &stake_amount, &mut slash).is_ok());
    assert_eq!(
        tracker.apply(cert_b, &registry, &stake_checker, &stake_amount, &mut slash),
        Err(SlashingReason::ConflictingCertificate)
    );
    assert!(slashed, "operator should have been slashed");
}

#[test]
fn identical_certificate_is_idempotent() {
    let kp = KeyPair::generate();
    let mut registry = HashMap::new();
    registry.insert(kp.account_id(), kp.public_key());

    let tracker = CertificateTracker::new();
    let staked: HashMap<_, _> = [(kp.account_id(), true)].into_iter().collect();
    let mut slashed = false;

    let cert = SynthesisCertificate::sign(
        &kp, 0, 1, 0, 0, [1u8; 32], [2u8; 32], [3u8; 32], [7u8; 32], [8u8; 32], [11u8; 32], 0, 0,
    );

    let stake_checker = |op: &fluidic::crypto::AccountId| *staked.get(op).unwrap_or(&false);
    let stake_amount = |_op: &fluidic::crypto::AccountId| 1_000u128;
    let mut slash = |_op| slashed = true;

    assert!(tracker.apply(cert.clone(), &registry, &stake_checker, &stake_amount, &mut slash).is_ok());
    assert!(tracker.apply(cert, &registry, &stake_checker, &stake_amount, &mut slash).is_ok());
    assert!(!slashed);
}
