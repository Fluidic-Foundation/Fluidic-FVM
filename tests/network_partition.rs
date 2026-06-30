use fluidic::consensus::oscillator::Oscillator;
use fluidic::crypto::keys::KeyPair;
use fluidic::crypto::{Signal, StatefulShift, VectorClock};
use fluidic::operator::{StakeTable, StakingConfig};
use std::collections::HashMap;

#[test]
fn partitioned_mesh_reaches_quorum_after_heal() {
    let n = 3usize;
    let op_keypairs: Vec<KeyPair> = (0..n).map(|_| KeyPair::generate()).collect();
    let user = KeyPair::generate();
    let recipient = KeyPair::generate();

    let nodes: Vec<Oscillator> = op_keypairs
        .iter()
        .enumerate()
        .map(|(i, kp)| {
            let stake_table = StakeTable::new(StakingConfig { min_stake: 1 });
            for op in &op_keypairs {
                stake_table.stake(op.account_id(), 1_000);
            }
            Oscillator::new_with_stake([i as u8; 32], 64, kp.clone(), stake_table)
        })
        .collect();

    // Seed the user account on every node.
    for node in &nodes {
        node.seed_account(user.account_id(), 10_000_000_000_000);
    }

    // Build a shared key registry.
    let mut key_registry = HashMap::new();
    key_registry.insert(user.account_id(), user.public_key());
    key_registry.insert(recipient.account_id(), recipient.public_key());
    for op in &op_keypairs {
        key_registry.insert(op.account_id(), op.public_key());
    }

    // Build a valid stateful shift and ingest it into every node before the
    // network partition takes effect.
    let mut vc = VectorClock::new();
    vc.tick(nodes[0].id);
    let shift = StatefulShift::new(
        &user,
        fluidic::crypto::DEFAULT_DEX_DOMAIN,
        recipient.account_id(),
        1_000_000_000,
        vc,
        vec![],
        0,
        0,
    );
    for node in &nodes {
        node.ingest(Signal::Stateful(shift.clone())).unwrap();
    }

    // Partition: nodes 0 and 1 can exchange messages; node 2 is isolated.
    let partition_ticks = 4usize;
    let mut cross_partition_queued: Vec<Vec<fluidic::consensus::certificate::SynthesisCertificate>> =
        vec![vec![]; n];

    for step in 0..partition_ticks {
        for i in 0..n {
            let _result = nodes[i].synthesize(&key_registry);
            let cert = nodes[i]
                .certificates
                .read()
                .unwrap()
                .get(&(step as u64))
                .cloned();

            for j in 0..n {
                if i == j {
                    continue;
                }
                if let Some(ref c) = cert {
                    let same_group = (i < 2) == (j < 2);
                    if same_group {
                        let _ = nodes[j].ingest_certificate(c.clone(), &key_registry);
                    } else {
                        cross_partition_queued[j].push(c.clone());
                    }
                }
            }
        }

        if step == partition_ticks - 1 {
            for node in &nodes {
                assert!(
                    node.check_quorum(step as u64).is_none(),
                    "quorum must not form while the network is partitioned"
                );
            }
        }
    }

    // Heal the partition: deliver all queued cross-partition certificates.
    for j in 0..n {
        for c in cross_partition_queued[j].drain(..) {
            let _ = nodes[j].ingest_certificate(c, &key_registry);
        }
    }

    // Run a few more ticks so straggler certificates can be observed.
    for step in partition_ticks..partition_ticks + 3 {
        for i in 0..n {
            let _result = nodes[i].synthesize(&key_registry);
            let cert = nodes[i]
                .certificates
                .read()
                .unwrap()
                .get(&(step as u64))
                .cloned();
            for j in 0..n {
                if i == j {
                    continue;
                }
                if let Some(ref c) = cert {
                    let _ = nodes[j].ingest_certificate(c.clone(), &key_registry);
                }
            }
        }
    }

    // After healing, all nodes should agree on a quorum for the post-heal tick.
    let quorum_tick = (partition_ticks + 2) as u64;
    let mut views = Vec::new();
    for node in &nodes {
        let view = node
            .check_quorum(quorum_tick)
            .expect("node failed to reach quorum after heal");
        views.push(view.0);
    }

    let first = &views[0];
    for (i, view) in views.iter().enumerate() {
        assert_eq!(
            view, first,
            "node {} has divergent quorum view after heal",
            i
        );
    }

    // The user shift should have finalized exactly once.  With metabolic decay
    // the recipient balance will be slightly less than the original transfer;
    // the important invariant is that all nodes agree on the same value.
    let mut recipient_balances = Vec::new();
    for node in &nodes {
        let balances = node.synthesize(&key_registry).final_balances;
        let b = balances.get(&recipient.account_id()).copied().unwrap_or(0);
        assert!(b > 0, "recipient received nothing");
        recipient_balances.push(b);
    }
    let first = recipient_balances[0];
    for (i, b) in recipient_balances.iter().enumerate() {
        assert_eq!(*b, first, "node {} recipient balance diverged", i);
    }
}
