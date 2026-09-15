use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const N: usize = 1024;
pub const DEGREE: usize = 16;
pub const E: usize = N * DEGREE;
pub const ARMS: [&str; 3] = ["Frozen", "Plastic", "Yoked"];
pub const ADAPT: usize = 50_000;
pub const WARM: usize = 256;
pub const SAMPLES: usize = 10_000;

pub fn draw(seed: u64, namespace: &str, counter: usize, m: usize) -> usize {
    assert!(m > 0);
    let limit = (1u128 << 64) / m as u128 * m as u128;
    for attempt in 0u64.. {
        let key = format!("NE0-v1/{seed}/{namespace}/{counter}/{attempt}");
        let hash = Sha256::digest(key.as_bytes());
        let word = u64::from_le_bytes(hash[..8].try_into().unwrap());
        if (word as u128) < limit {
            return (word % m as u64) as usize;
        }
    }
    unreachable!()
}
pub fn shuffle<T>(items: &mut [T], seed: u64, namespace: &str) {
    for (counter, i) in (1..items.len()).rev().enumerate() {
        let j = draw(seed, namespace, counter, i + 1);
        items.swap(i, j);
    }
}
pub fn grammar(seed: u64, phase: &str, count: usize) -> Vec<u8> {
    let namespace = format!("grammar/{phase}");
    let mut out = vec![
        draw(seed, &namespace, 0, 4) as u8,
        draw(seed, &namespace, 1, 4) as u8,
    ];
    for t in 2..count {
        let r = draw(seed, &namespace, t, 10);
        let z = if r < 7 { 0 } else { r - 6 } as u8;
        out.push((out[t - 2] + out[t - 1] + z) % 4);
    }
    out.truncate(count);
    out
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub v: u16,
    pub r: u8,
    pub s: u8,
    pub q: u16,
}
impl Node {
    pub fn bytes(self) -> [u8; 6] {
        let v = self.v.to_le_bytes();
        let q = self.q.to_le_bytes();
        [v[0], v[1], self.r, self.s, q[0], q[1]]
    }
    pub fn from_bytes(b: &[u8]) -> Self {
        Self {
            v: u16::from_le_bytes([b[0], b[1]]),
            r: b[2],
            s: b[3],
            q: u16::from_le_bytes([b[4], b[5]]),
        }
    }
    pub fn valid(self) -> bool {
        self.v <= 255 && self.r <= 1 && self.s <= 1 && self.q <= 255
    }
    pub fn abi_bytes(self) -> [u8; 8] {
        let v = self.v.to_le_bytes();
        let q = self.q.to_le_bytes();
        [v[0], v[1], self.r, 0, self.s, 0, q[0], q[1]]
    }
    pub fn from_abi(b: &[u8]) -> Self {
        assert_eq!(b.len(), 8);
        assert_eq!(b[3], 0);
        assert_eq!(b[5], 0);
        Self {
            v: u16::from_le_bytes([b[0], b[1]]),
            r: b[2],
            s: b[4],
            q: u16::from_le_bytes([b[6], b[7]]),
        }
    }
}

// Independent mathematical oracle: checked i64 throughout, no calls to cell source.
pub fn reference_node(old: Node, current: i64) -> Node {
    assert!(old.valid() && (-512..=560).contains(&current));
    let (v, s) = if old.r != 0 {
        (0, 0)
    } else {
        let leaked = i64::from(old.v).checked_mul(7).unwrap() / 8;
        let u = leaked.checked_add(current).unwrap().clamp(0, 255);
        if u >= 64 {
            (0, 1)
        } else {
            (u as u16, 0)
        }
    };
    let q = (i64::from(old.q).checked_mul(15).unwrap() / 16)
        .checked_add(i64::from(s).checked_mul(64).unwrap())
        .unwrap()
        .min(255) as u16;
    Node { v, r: s, s, q }
}
pub fn reference_delta(pre: Node, post: Node, new_post: Node) -> i16 {
    let positive = i64::from(pre.q).checked_mul(i64::from(new_post.s)).unwrap();
    let negative = i64::from(post.q).checked_mul(i64::from(pre.s)).unwrap();
    (positive.checked_sub(negative).unwrap() / 64).clamp(-1, 1) as i16
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Topology {
    pub sources: Vec<usize>,
    pub permutation: Vec<usize>,
}
impl Topology {
    pub fn new(seed: u64) -> Self {
        let mut sources = Vec::with_capacity(E);
        for target in 0..N {
            let mut ids: Vec<_> = (0..N).filter(|&i| i != target).collect();
            shuffle(&mut ids, seed, &format!("topology/{target}"));
            ids.truncate(DEGREE);
            ids.sort_unstable();
            sources.extend(ids);
        }
        let mut permutation = vec![0; E];
        for sign in [1, -1] {
            let ids: Vec<_> = (0..E).filter(|&e| Self::sign(sources[e]) == sign).collect();
            assert!(ids.len() > 1);
            for candidate in 0usize.. {
                let mut shuffled = ids.clone();
                shuffle(&mut shuffled, seed, &format!("yoke/{sign}/{candidate}"));
                if ids.iter().zip(&shuffled).all(|(a, b)| a != b) {
                    for (&a, &b) in ids.iter().zip(&shuffled) {
                        permutation[a] = b;
                    }
                    break;
                }
            }
        }
        Self {
            sources,
            permutation,
        }
    }
    pub fn sign(source: usize) -> i64 {
        if source < 768 {
            1
        } else {
            -1
        }
    }
    pub fn bytes(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(E * 8);
        for (&s, &p) in self.sources.iter().zip(&self.permutation) {
            b.extend_from_slice(&(s as u32).to_le_bytes());
            b.extend_from_slice(&(p as u32).to_le_bytes());
        }
        b
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Network {
    pub weights: Vec<u16>,
    pub nodes: Vec<Node>,
}
impl Network {
    pub fn new(seed: u64) -> Self {
        let weights = (0..E)
            .map(|e| (1 + draw(seed, &format!("weight/{e}"), 0, 16)) as u16)
            .collect();
        Self {
            weights,
            nodes: vec![Node::default(); N],
        }
    }
    pub fn reset(&mut self) {
        self.nodes.fill(Node::default());
    }
    pub fn persistent_bytes(&self) -> Vec<u8> {
        self.weights.iter().flat_map(|w| w.to_le_bytes()).collect()
    }
    pub fn currents(&self, topology: &Topology, symbol: u8) -> Vec<i64> {
        assert!(symbol < 4);
        let mut values = vec![0i64; N];
        for (i, v) in values.iter_mut().enumerate() {
            for e in i * DEGREE..(i + 1) * DEGREE {
                let j = topology.sources[e];
                assert!(self.weights[e] <= 32 && self.nodes[j].valid());
                let current = Topology::sign(j)
                    .checked_mul(i64::from(self.weights[e]))
                    .unwrap()
                    .checked_mul(i64::from(self.nodes[j].s))
                    .unwrap();
                *v = v.checked_add(current).unwrap();
            }
            if i % 4 == usize::from(symbol) {
                *v = v.checked_add(48).unwrap();
            }
            assert!((-512..=560).contains(v));
        }
        values
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Movement {
    pub requested: u64,
    pub realised: u64,
    pub clipped: u64,
    pub requested_nonzero: u64,
    pub realised_nonzero: u64,
    pub net: i64,
    pub realised_hist: [u64; 3],
    pub requested_hist: [u64; 3],
}
impl Movement {
    pub fn add(&mut self, other: &Self) {
        self.requested += other.requested;
        self.realised += other.realised;
        self.clipped += other.clipped;
        self.requested_nonzero += other.requested_nonzero;
        self.realised_nonzero += other.realised_nonzero;
        self.net += other.net;
        for i in 0..3 {
            self.realised_hist[i] += other.realised_hist[i];
            self.requested_hist[i] += other.requested_hist[i];
        }
    }
}
pub fn apply(network: &mut Network, topology: &Topology, deltas: &[i16]) -> [Movement; 2] {
    assert_eq!(deltas.len(), E);
    let mut stats: [Movement; 2] = Default::default();
    for (e, &d) in deltas.iter().enumerate() {
        assert!((-1..=1).contains(&d));
        let old = i64::from(network.weights[e]);
        let new = old.checked_add(i64::from(d)).unwrap().clamp(0, 32);
        let realised = new - old;
        network.weights[e] = new as u16;
        let s = &mut stats[usize::from(topology.sources[e] >= 768)];
        s.requested += d.unsigned_abs() as u64;
        s.realised += realised.unsigned_abs();
        s.clipped += (i64::from(d) - realised).unsigned_abs();
        s.net += realised;
        s.requested_nonzero += u64::from(d != 0);
        s.realised_nonzero += u64::from(realised != 0);
        s.realised_hist[(realised + 1) as usize] += 1;
        s.requested_hist[(d + 1) as usize] += 1;
    }
    for s in &stats {
        assert_eq!(s.requested, s.realised + s.clipped);
    }
    stats
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub seed: u64,
    pub topology: Topology,
    pub networks: Vec<Network>,
}

/// This serialization cannot carry membrane/spike/trace history.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistentCheckpoint {
    pub seed: u64,
    pub topology: Topology,
    pub weights: Vec<Vec<u16>>,
}
impl PersistentCheckpoint {
    pub fn capture(c: &Checkpoint) -> Self {
        Self {
            seed: c.seed,
            topology: c.topology.clone(),
            weights: c.networks.iter().map(|n| n.weights.clone()).collect(),
        }
    }
    pub fn reconstruct(self) -> Checkpoint {
        assert_eq!(self.weights.len(), 3);
        let networks = self
            .weights
            .into_iter()
            .map(|weights| {
                assert_eq!(weights.len(), E);
                assert!(weights.iter().all(|&w| w <= 32));
                Network {
                    weights,
                    nodes: vec![Node::default(); N],
                }
            })
            .collect();
        Checkpoint {
            seed: self.seed,
            topology: self.topology,
            networks,
        }
    }
}
