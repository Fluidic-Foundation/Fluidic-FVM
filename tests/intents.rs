use fluidic::consensus::Oscillator;
use fluidic::crypto::keys::KeyPair;
use fluidic::crypto::{
    AccountId, AgentRegistrationShift, IntentConstraint, IntentFillShift, IntentShift, Signal,
    DEFAULT_DEX_DOMAIN,
};
use std::collections::HashMap;

fn registry(keys: &[KeyPair]) -> HashMap<AccountId, ed25519_dalek::VerifyingKey> {
    keys.iter().map(|kp| (kp.account_id(), kp.public_key())).collect()
}

#[test]
fn agent_registration_sets_account_type() {
    let osc = Oscillator::new([1u8; 32], 16);
    let owner = KeyPair::generate();
    let agent = KeyPair::generate();

    osc.seed_account(owner.account_id(), 1_000_000_000_000_000);

    let reg = AgentRegistrationShift::new(
        &owner,
        &agent.public_key(),
        100, // expiry_tick
        1,
        0,
    );

    let keys = registry(&[owner.clone(), agent.clone()]);
    assert!(osc.apply_agent_registration(&reg, &keys));

    let field = osc.wave_field.lock().unwrap();
    let account_type = field.account_type(agent.account_id());
    assert!(
        matches!(
            account_type,
            fluidic::field::wave_field::AccountType::Agent {
                owner: o,
                expiry_tick: 100,
            } if o == owner.account_id()
        ),
        "agent account type should be set"
    );
}

#[test]
fn intent_transfer_is_matched_and_settled() {
    let osc = Oscillator::new([1u8; 32], 16);
    let owner = KeyPair::generate();
    let beneficiary = KeyPair::generate();
    let solver = KeyPair::generate();

    osc.seed_account(owner.account_id(), 1_000_000_000_000_000);
    osc.seed_account(solver.account_id(), 1_000_000_000_000_000);

    let keys = registry(&[owner.clone(), beneficiary.clone(), solver.clone()]);

    let intent = IntentShift::new(
        &owner,
        DEFAULT_DEX_DOMAIN,
        10, // deadline_tick
        IntentConstraint::Transfer {
            to: beneficiary.account_id(),
            min_amount: 100_000_000_000, // 100 WAVE
        },
        1_000_000_000, // 0.001 WAVE solver reward
        1,
        0,
    );

    osc.ingest(Signal::Intent(intent.clone()), &keys).unwrap();

    let fill = IntentFillShift::new(
        &solver,
        intent.intent_id,
        100_000_000_000,
        1,
        0,
    );

    osc.ingest(Signal::IntentFill(fill), &keys).unwrap();

    // Synthesize at tick 5, before the deadline.
    osc.synthesis_tick.store(5, std::sync::atomic::Ordering::SeqCst);
    let result = osc.synthesize(&keys);

    assert_eq!(result.intents_matched, 1);

    let field = osc.wave_field.lock().unwrap();
    let beneficiary_balance = field.account_balance(beneficiary.account_id()).units;
    assert_eq!(beneficiary_balance, 100_000_000_000);
}

#[test]
fn intent_fill_below_min_amount_is_rejected() {
    let osc = Oscillator::new([1u8; 32], 16);
    let owner = KeyPair::generate();
    let beneficiary = KeyPair::generate();
    let solver = KeyPair::generate();

    osc.seed_account(owner.account_id(), 1_000_000_000_000_000);

    let keys = registry(&[owner.clone(), beneficiary.clone(), solver.clone()]);

    let intent = IntentShift::new(
        &owner,
        DEFAULT_DEX_DOMAIN,
        10,
        IntentConstraint::Transfer {
            to: beneficiary.account_id(),
            min_amount: 100_000_000_000,
        },
        1_000_000_000,
        1,
        0,
    );

    osc.ingest(Signal::Intent(intent.clone()), &keys).unwrap();

    let fill = IntentFillShift::new(
        &solver,
        intent.intent_id,
        50_000_000_000, // below min_amount
        1,
        0,
    );

    osc.ingest(Signal::IntentFill(fill), &keys).unwrap();

    osc.synthesis_tick.store(5, std::sync::atomic::Ordering::SeqCst);
    let result = osc.synthesize(&keys);

    assert_eq!(result.intents_matched, 0);
}

#[test]
fn expired_intent_is_dropped() {
    let osc = Oscillator::new([1u8; 32], 16);
    let owner = KeyPair::generate();
    let beneficiary = KeyPair::generate();

    osc.seed_account(owner.account_id(), 1_000_000_000_000_000);

    let keys = registry(&[owner.clone(), beneficiary.clone()]);

    let intent = IntentShift::new(
        &owner,
        DEFAULT_DEX_DOMAIN,
        5, // deadline_tick
        IntentConstraint::Transfer {
            to: beneficiary.account_id(),
            min_amount: 100_000_000_000,
        },
        1_000_000_000,
        1,
        0,
    );

    osc.ingest(Signal::Intent(intent), &keys).unwrap();

    osc.synthesis_tick.store(10, std::sync::atomic::Ordering::SeqCst);
    let result = osc.synthesize(&keys);

    assert_eq!(result.intents_matched, 0);
    assert!(osc.pending_intents.lock().unwrap().is_empty());
}
