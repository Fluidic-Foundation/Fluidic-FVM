use fluidic::consensus::certificate::{CertificateTracker, QuorumView, SynthesisCertificate};
use fluidic::crypto::keys::KeyPair;
use fluidic::crypto::AccountId;
use std::collections::HashMap;

#[test]
fn quorum_reaches_threshold_when_two_thirds_stake_agrees() {
    let a = KeyPair::generate();
    let b = KeyPair::generate();
    let c = KeyPair::generate();

    let mut registry = HashMap::new();
    registry.insert(a.account_id(), a.public_key());
    registry.insert(b.account_id(), b.public_key());
    registry.insert(c.account_id(), c.public_key());

    let stakes: HashMap<AccountId, u128> = [
        (a.account_id(), 100),
        (b.account_id(), 100),
        (c.account_id(), 100),
    ]
    .into_iter()
    .collect();
    let total: u128 = stakes.values().sum();
    let threshold = total / 3 * 2 + 1; // >2/3

    let tracker = CertificateTracker::new();
    let stake_checker = |op: &AccountId| stakes.contains_key(op);
    let stake_amount = |op: &AccountId| *stakes.get(op).unwrap_or(&0);
    let mut slash = |_op| {};

    let cert_a = SynthesisCertificate::sign(
        &a, 1, 5, 2, 0, [1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32], [5u8; 32], [6u8; 32], 0, 0,
    );
    let cert_b = SynthesisCertificate::sign(
        &b, 1, 5, 2, 0, [1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32], [5u8; 32], [6u8; 32], 0, 0,
    );
    let cert_c = SynthesisCertificate::sign(
        &c, 1, 5, 2, 0, [1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32], [5u8; 32], [6u8; 32], 0, 0,
    );

    tracker.apply(cert_a, &registry, &stake_checker, &stake_amount, &mut slash).unwrap();
    assert!(tracker.check_quorum(1, threshold).is_none());

    tracker.apply(cert_b, &registry, &stake_checker, &stake_amount, &mut slash).unwrap();
    assert!(tracker.check_quorum(1, threshold).is_none());

    tracker.apply(cert_c, &registry, &stake_checker, &stake_amount, &mut slash).unwrap();
    let (view, stake) = tracker.check_quorum(1, threshold).unwrap();
    assert_eq!(stake, 300);
    assert_eq!(view, QuorumView {
        commutative_root: [1u8; 32],
        stateful_root: [2u8; 32],
        evm_root: [6u8; 32],
        balances_root: [3u8; 32],
        stake_root: [4u8; 32],
        reward_root: [5u8; 32],
    });
}

#[test]
fn quorum_does_not_form_on_conflicting_roots() {
    let a = KeyPair::generate();
    let b = KeyPair::generate();

    let mut registry = HashMap::new();
    registry.insert(a.account_id(), a.public_key());
    registry.insert(b.account_id(), b.public_key());

    let stakes: HashMap<AccountId, u128> =
        [(a.account_id(), 100), (b.account_id(), 100)].into_iter().collect();
    let threshold = 134u128;

    let tracker = CertificateTracker::new();
    let stake_checker = |op: &AccountId| stakes.contains_key(op);
    let stake_amount = |op: &AccountId| *stakes.get(op).unwrap_or(&0);
    let mut slash = |_op| {};

    let cert_a = SynthesisCertificate::sign(
        &a, 1, 5, 2, 0, [1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32], [5u8; 32], [6u8; 32], 0, 0,
    );
    let cert_b = SynthesisCertificate::sign(
        &b, 1, 5, 2, 0, [7u8; 32], [8u8; 32], [9u8; 32], [10u8; 32], [11u8; 32], [12u8; 32], 0, 0,
    );

    tracker.apply(cert_a, &registry, &stake_checker, &stake_amount, &mut slash).unwrap();
    tracker.apply(cert_b, &registry, &stake_checker, &stake_amount, &mut slash).unwrap();
    assert!(tracker.check_quorum(1, threshold).is_none());
}
