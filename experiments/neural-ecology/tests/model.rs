use ne0::model::*;

#[test]
fn accelerated_sha256_matches_portable_sha256() {
    use sha2::{Digest, Sha256};
    for size in [0, 1, 55, 56, 64, 65, 1024, 200_000] {
        let bytes: Vec<u8> = (0..size).map(|i| (i % 251) as u8).collect();
        assert_eq!(ne0::digest(&bytes), format!("{:x}", Sha256::digest(&bytes)));
    }
}

#[test]
fn persistent_serialization_cannot_contain_activity_history() {
    let full = Checkpoint {
        seed: 1001,
        topology: Topology::new(1001),
        networks: vec![Network::new(1001); 3],
    };
    let captured = PersistentCheckpoint::capture(&full);
    let value = serde_json::to_value(&captured).unwrap();
    assert!(value.get("nodes").is_none());
    assert!(value.get("networks").is_none());
    let reconstructed = captured.reconstruct();
    assert_eq!(full.networks, reconstructed.networks);
    assert!(
        serde_json::from_value::<PersistentCheckpoint>(serde_json::to_value(full).unwrap())
            .is_err()
    );
}

#[test]
fn addressed_rng_and_prefix_are_stable() {
    // Independently computed with Python hashlib, not copied from this RNG.
    assert_eq!(draw(1001, "grammar/A", 0, 4), 3);
    assert_eq!(draw(1001, "grammar/A", 1, 4), 1);
    assert_eq!(draw(1001, "weight/0", 0, 16), 8);
    assert_eq!(grammar(1001, "A", 50), grammar(1001, "A", 80)[..50]);
    assert_ne!(grammar(1001, "A", 50), grammar(1001, "B", 50));
}
#[test]
fn reset_and_checkpoint_remove_only_transients() {
    let mut network = Network::new(1001);
    network.weights[0] = 32;
    network.nodes[0] = Node {
        v: 32,
        r: 1,
        s: 1,
        q: 255,
    };
    let weights = network.persistent_bytes();
    network.reset();
    assert_eq!(weights, network.persistent_bytes());
    assert!(network.nodes.iter().all(|&n| n == Node::default()));
    let once = network.clone();
    network.reset();
    assert_eq!(once, network);
    let json = serde_json::to_vec(&network).unwrap();
    assert_eq!(network, serde_json::from_slice::<Network>(&json).unwrap());
}
#[test]
fn state_encoding_is_canonical_and_abi_has_explicit_padding() {
    let n = Node {
        v: 255,
        r: 1,
        s: 1,
        q: 128,
    };
    assert_eq!(n.bytes(), [255, 0, 1, 1, 128, 0]);
    assert_eq!(n.abi_bytes(), [255, 0, 1, 0, 1, 0, 128, 0]);
    assert_eq!(n, Node::from_bytes(&n.bytes()));
    assert_eq!(n, Node::from_abi(&n.abi_bytes()));
}
#[test]
fn timing_rule_truncates_negative_fractions_toward_zero() {
    let pre = Node {
        s: 1,
        ..Default::default()
    };
    for (q, expected) in [(1, 0), (63, 0), (64, -1), (255, -1)] {
        assert_eq!(
            reference_delta(
                pre,
                Node {
                    q,
                    ..Default::default()
                },
                Node::default()
            ),
            expected
        );
    }
}
#[test]
fn topology_and_yoke_constraints_and_clipping_audit() {
    let top = Topology::new(1001);
    for target in 0..N {
        let row = &top.sources[target * DEGREE..(target + 1) * DEGREE];
        assert!(!row.contains(&target));
        assert!(row.windows(2).all(|w| w[0] < w[1]));
    }
    let mut permutation = top.permutation.clone();
    permutation.sort_unstable();
    assert_eq!(permutation, (0..E).collect::<Vec<_>>());
    for (e, &p) in top.permutation.iter().enumerate() {
        assert_ne!(e, p);
        assert_eq!(
            Topology::sign(top.sources[e]),
            Topology::sign(top.sources[p])
        );
    }
    let mut network = Network::new(1001);
    network.weights.fill(0);
    let stats = apply(&mut network, &top, &vec![-1; E]);
    assert!(network.weights.iter().all(|&w| w == 0));
    assert_eq!(stats.iter().map(|s| s.clipped).sum::<u64>(), E as u64);
    network.weights.fill(32);
    let stats = apply(&mut network, &top, &vec![1; E]);
    assert_eq!(stats.iter().map(|s| s.realised).sum::<u64>(), 0);
}
#[test]
fn old_snapshot_propagates_spikes_and_keeps_current_bounds() {
    let top = Topology::new(1001);
    let mut net = Network::new(1001);
    net.weights.fill(32);
    net.nodes.fill(Node {
        s: 1,
        ..Default::default()
    });
    let currents = net.currents(&top, 0);
    for (i, &current) in currents.iter().enumerate() {
        let excitatory = top.sources[i * DEGREE..(i + 1) * DEGREE]
            .iter()
            .filter(|&&j| j < 768)
            .count() as i64;
        assert_eq!(
            current,
            32 * (2 * excitatory - 16) + if i % 4 == 0 { 48 } else { 0 }
        );
    }
}
