use fluidic::consensus::Oscillator;
use fluidic::crypto::keys::KeyPair;
use fluidic::crypto::{AccountId, Signal, StatefulShift, VectorClock, DEFAULT_DEX_DOMAIN};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Build a registry mapping every account id to its verifying key.
fn build_registry(keypairs: &[KeyPair]) -> HashMap<AccountId, ed25519_dalek::VerifyingKey> {
    keypairs
        .iter()
        .map(|kp| (kp.account_id(), kp.public_key()))
        .collect()
}

#[test]
fn spam_invalid_and_underfunded_shifts() {
    let osc = Oscillator::new([1u8; 32], 64);
    let victims: Vec<KeyPair> = (0..10).map(|_| KeyPair::generate()).collect();
    for v in &victims {
        osc.seed_account(v.account_id(), 1_000_000_000_000);
    }

    let attacker = KeyPair::generate();
    let registry = build_registry(&victims);

    let total = 1_000usize;
    let start = Instant::now();
    let mut ingest_failures = 0usize;

    for i in 0..total {
        let mut vc = VectorClock::new();
        vc.tick(osc.id);

        let shift = match i % 4 {
            // Invalid signature: unique random bytes for signature.
            0 => {
                let mut s = StatefulShift::new(&attacker, DEFAULT_DEX_DOMAIN, victims[0].account_id(), 1, vc, vec![], i as u64, 0);
                s.signature = (0..64).map(|b| ((i + b) % 256) as u8).collect();
                s
            }
            // Underfunded: attacker has no balance.
            1 => StatefulShift::new(&attacker, DEFAULT_DEX_DOMAIN, victims[0].account_id(), 1_000_000_000_000, vc, vec![], i as u64, 0),
            // Zero amount.
            2 => StatefulShift::new(&victims[i % victims.len()], DEFAULT_DEX_DOMAIN, victims[1].account_id(), 0, vc, vec![], i as u64, 0),
            // Valid but from a victim with sufficient balance.
            _ => StatefulShift::new(
                &victims[i % victims.len()],
                DEFAULT_DEX_DOMAIN,
                victims[(i + 1) % victims.len()].account_id(),
                1,
                vc,
                vec![],
                i as u64,
                0,
            ),
        };

        // Count ingest-time rejections as well as synthesis-time rejections.
        if osc.ingest(Signal::Stateful(shift), &registry).is_err() {
            ingest_failures += 1;
        }
    }

    let ingest_elapsed = start.elapsed();

    // Synthesize once to process the queue.
    let synth_start = Instant::now();
    let result = osc.synthesize(&registry);
    let synth_elapsed = synth_start.elapsed();

    let rejected = result.stateful_rejected.len() + ingest_failures;
    let accepted = osc.stateful_count();
    let processed = rejected + accepted;
    let median_ingest_us = ingest_elapsed.as_micros() as f64 / total as f64;
    let rejection_rate = if processed > 0 {
        rejected as f64 / processed as f64
    } else {
        0.0
    };

    // 3/4 of the shifts are malicious; the rejection rate among processed
    // shifts should be well above 50% (most attacker shifts are rejected).
    assert!(
        rejection_rate > 0.5,
        "rejection rate {} too low (rejected={} accepted={})",
        rejection_rate,
        rejected,
        accepted
    );
    assert!(
        ingest_failures > 0,
        "zero-amount shifts should be rejected at ingest"
    );
    assert!(
        result.stateful_rejected.len() > 0,
        "invalid/underfunded shifts should be rejected at synthesis"
    );
    assert!(
        median_ingest_us < 5_000.0,
        "median ingest latency {} us too high",
        median_ingest_us
    );
    assert!(
        synth_elapsed.as_millis() < 5_000,
        "synthesis took too long: {:?}",
        synth_elapsed
    );

    println!(
        "spam test: total={} rejected={} accepted={} rate={:.2} median_ingest={:.2}us synth={:?}",
        total, rejected, accepted, rejection_rate, median_ingest_us, synth_elapsed
    );
}

#[test]
fn mev_sandwich_extracts_value() {
    use fluidic::api::state::ApiState;

    let osc = Arc::new(Oscillator::new([1u8; 32], 64));
    let api = ApiState::new(osc.clone());
    let attacker = KeyPair::generate();
    let victim = KeyPair::generate();

    // Register attacker and victim so their token accounts can sign shifts.
    api.register_key(attacker.account_id(), attacker.public_key());
    api.register_key(victim.account_id(), victim.public_key());
    let (att_wave, att_usdc) = api.token_accounts(attacker.account_id());
    let (vic_wave, vic_usdc) = api.token_accounts(victim.account_id());
    api.register_key(att_wave, attacker.public_key());
    api.register_key(att_usdc, attacker.public_key());
    api.register_key(vic_wave, victim.public_key());
    api.register_key(vic_usdc, victim.public_key());

    // Seed attacker and victim WAVE/USDC accounts for the sandwich.
    osc.seed_account(att_wave, 20_000_000_000_000);
    osc.seed_account(att_usdc, 20_000_000_000_000);
    osc.seed_account(vic_wave, 10_000_000_000_000);
    osc.seed_account(vic_usdc, 10_000_000_000_000);

    let registry = api.key_registry();

    let mut vc = VectorClock::new();
    vc.tick(osc.id);

    // 1. Attacker dumps WAVE into the pool, pushing the WAVE price down.
    let attack_amount = 10_000_000_000_000u128;
    let attack = StatefulShift::new(&attacker, DEFAULT_DEX_DOMAIN, api.pool_wave_account, attack_amount, vc.clone(), vec![], 1, 0);
    // Rewrite `from` to the attacker's derived WAVE account and re-sign.
    let mut attack = attack;
    attack.from = att_wave;
    attack.signature = attacker.sign(&attack.signing_bytes()).to_bytes().to_vec();
    osc.ingest(Signal::Stateful(attack), &registry).unwrap();
    osc.synthesize(&registry);
    let mid_price = api.pool_price();

    // 2. Victim sells WAVE at the now-depressed price.
    let mut victim_swap = StatefulShift::new(&victim, DEFAULT_DEX_DOMAIN, api.pool_wave_account, 1_000_000_000_000, vc.clone(), vec![], 2, 0);
    victim_swap.from = vic_wave;
    victim_swap.signature = victim.sign(&victim_swap.signing_bytes()).to_bytes().to_vec();
    osc.ingest(Signal::Stateful(victim_swap), &registry).unwrap();
    osc.synthesize(&registry);

    // 3. Attacker buys back WAVE with USDC, closing most of the position at a profit.
    let mut close = StatefulShift::new(&attacker, DEFAULT_DEX_DOMAIN, api.pool_usdc_account, attack_amount / 2, vc.clone(), vec![], 3, 0);
    close.from = att_usdc;
    close.signature = attacker.sign(&close.signing_bytes()).to_bytes().to_vec();
    osc.ingest(Signal::Stateful(close), &registry).unwrap();
    osc.synthesize(&registry);
    let final_price = api.pool_price();

    // The sandwich should move the pool price: first push it down, then partially
    // restore it. Without the attacker buying back USDC, the price would stay lower.
    assert!(
        mid_price < 1.0,
        "attacker dump should push WAVE price below 1.0: {}",
        mid_price
    );
    assert!(
        final_price > mid_price,
        "attacker buy-back should raise the price: mid={} final={}",
        mid_price,
        final_price
    );

    println!(
        "mev sandwich: mid_price={} final_price={}",
        mid_price, final_price
    );
}

#[test]
fn conflicting_shifts_across_two_oscillators_reject_double_spend() {
    // Two oscillator nodes share the same key registry and start with the same seed state.
    let osc_a = Oscillator::new([1u8; 32], 64);
    let osc_b = Oscillator::new([2u8; 32], 64);
    let alice = KeyPair::generate();
    let bob = KeyPair::generate();
    let charlie = KeyPair::generate();
    let balance = 1_000_000_000_000u128;

    osc_a.seed_account(alice.account_id(), balance);
    osc_b.seed_account(alice.account_id(), balance);

    let registry = build_registry(&[alice.clone(), bob.clone(), charlie.clone()]);

    // Each oscillator receives one conflicting shift from alice.
    let amount = balance / 2 + 1;
    let mut vc_a = VectorClock::new();
    vc_a.tick(osc_a.id);
    let shift_a = StatefulShift::new(&alice, DEFAULT_DEX_DOMAIN, bob.account_id(), amount, vc_a, vec![], 1, 0);

    let mut vc_b = VectorClock::new();
    vc_b.tick(osc_b.id);
    let shift_b = StatefulShift::new(&alice, DEFAULT_DEX_DOMAIN, charlie.account_id(), amount, vc_b, vec![], 2, 0);

    osc_a.ingest(Signal::Stateful(shift_a), &registry).unwrap();
    osc_b.ingest(Signal::Stateful(shift_b), &registry).unwrap();

    // Both synthesize independently (partitioned state).
    let _res_a = osc_a.synthesize(&registry);
    let _res_b = osc_b.synthesize(&registry);

    // When the shifts are later gossiped to a single node, the double-spend is caught.
    // Simulate merge by ingesting osc_b's accepted shift into osc_a and vice versa.
    let dag_b = osc_b.dag.lock().unwrap();
    for (_, node) in dag_b.nodes.iter() {
        if matches!(node.status, fluidic::consensus::dag::ShiftStatus::Accepted | fluidic::consensus::dag::ShiftStatus::Finalized) {
            osc_a.ingest(Signal::Stateful(node.shift.clone()), &registry).unwrap();
        }
    }
    drop(dag_b);

    // Synthesize enough ticks for finalization on the merged node.
    for _ in 0..=fluidic::consensus::dag::VectorClockDag::FINALIZATION_DEPTH {
        osc_a.synthesize(&registry);
    }

    let dag_a = osc_a.dag.lock().unwrap();
    let statuses: Vec<_> = dag_a.nodes.values().map(|n| n.status.clone()).collect();
    let finalized = statuses.iter().filter(|s| matches!(s, fluidic::consensus::dag::ShiftStatus::Finalized)).count();
    let rejected = statuses
        .iter()
        .filter(|s| matches!(s, fluidic::consensus::dag::ShiftStatus::Rejected(fluidic::consensus::dag::DagError::DoubleSpend(_))))
        .count();

    assert_eq!(finalized, 1, "only one conflicting shift should finalize");
    assert_eq!(rejected, 1, "the other should be rejected as double-spend");
}
