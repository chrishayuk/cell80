//! Composition over cells — the un-foldability gate **and** the learned-prior search ladder,
//! in the documented b3bits regime (see chuk-soma `docs/roadmap.md`: B3′).
//!
//! Run: `cargo run --release --example composition_eval -p cell80`
//!
//! Cells are real (the VM is the transition function). The action set is **lossy bit ops**
//! (AND/OR/XOR/rotate/swap/reverse with fixed constants): AND/OR destroy information, so the
//! graph is non-reversible and **Hamming distance is deceptive** (an AND that lowers Hamming
//! can clear a bit you still need). Task: from `start`, reach a target produced by a random
//! `L`-step chain — deliberately **far**, so a search must work for it.
//!
//! The lesson from the SOMA results (and where my first run went wrong): a learned organ only
//! earns its keep **where blind search is intractable**. On a tiny state space with a loose
//! budget, blind BFS is complete and cheap and wins by itself. So we sweep a **node budget**
//! into the regime where blind BFS breaks, and compare at EQUAL compute:
//!   * blind BFS            — uninformed, FIFO
//!   * Hamming best-first   — the deceptive heuristic (order by popcount(state^target))
//!   * MLP value best-first — learned value net `v≈dist` orders the frontier (the prior's pick)
//!   * MLP+MCTS            — PUCT, value net at leaves (expected to lose on a deterministic graph)
//!
//! Honest expectation from the docs: value best-first beats blind BFS and Hamming exactly where
//! blind BFS floors; MCTS loses (deterministic shortest-path favors best-first, and the node
//! budget starves MCTS of simulations — a flagged confound). Reported straight.
#![allow(clippy::needless_range_loop, clippy::map_entry)]
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

use cell80::{Cartridge, CartridgeOpts, CellConfig, Runner, DEFAULT_CYCLES};

const FDIM: usize = 48;
const DNORM: f32 = 12.0;

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn u16(&mut self) -> u16 {
        (self.next() & 0xFFFF) as u16
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    fn signed(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32 * 2.0 - 1.0
    }
}

struct Act {
    cart: Cartridge,
    arg: u16,
}

fn act(src: &str, id: &str, arg: u16) -> Act {
    let cart = Cartridge::compile(
        src,
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some(id.into()),
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| panic!("compile {id}: {e}"));
    Act { cart, arg }
}

fn actions() -> Vec<Act> {
    vec![
        act(
            include_str!("../cells/bit-mask/mask_intersection.rs"),
            "mask_intersection",
            0xFF00,
        ), // AND (lossy)
        act(
            include_str!("../cells/bit-mask/mask_intersection.rs"),
            "mask_intersection",
            0x0FF0,
        ), // AND (lossy)
        act(
            include_str!("../cells/bit-mask/mask_union.rs"),
            "mask_union",
            0x00FF,
        ), // OR (lossy)
        act(
            include_str!("../cells/bit-mask/mask_union.rs"),
            "mask_union",
            0x0F0F,
        ), // OR (lossy)
        act(
            include_str!("../cells/bit-mask/mask_xor.rs"),
            "mask_xor",
            0x0F0F,
        ),
        act(
            include_str!("../cells/bit-mask/mask_xor.rs"),
            "mask_xor",
            0xF0F0,
        ),
        act(
            include_str!("../cells/bit-mask/toggle_bit.rs"),
            "toggle_bit",
            0,
        ),
        act(
            include_str!("../cells/bit-mask/toggle_bit.rs"),
            "toggle_bit",
            7,
        ),
        act(
            include_str!("../cells/bit-mask/toggle_bit.rs"),
            "toggle_bit",
            15,
        ),
        act(include_str!("../cells/bit-encoding/rotl16.rs"), "rotl16", 4),
        act(
            include_str!("../cells/bit-encoding/swap_bytes.rs"),
            "swap_bytes",
            0,
        ),
        act(
            include_str!("../cells/bit-encoding/reverse_bits.rs"),
            "reverse_bits",
            0,
        ),
    ]
}

fn tables(acts: &[Act]) -> Vec<Vec<u16>> {
    acts.iter()
        .map(|a| {
            let mut r = Runner::new(&a.cart.program);
            let entry = a.cart.manifest.entry.clone();
            (0..=u16::MAX)
                .map(|v| {
                    r.run_fast(Some(&entry), &[v, a.arg], DEFAULT_CYCLES)
                        .map(|f| f.result)
                        .unwrap_or(v)
                })
                .collect()
        })
        .collect()
}

fn reverse_adj(t: &[Vec<u16>]) -> Vec<Vec<u16>> {
    let mut preds: Vec<Vec<u16>> = vec![Vec::new(); 1 << 16];
    for tab in t {
        for v in 0..=u16::MAX {
            preds[tab[v as usize] as usize].push(v);
        }
    }
    preds
}

fn dist_to(target: u16, preds: &[Vec<u16>]) -> Vec<u32> {
    let mut dist = vec![u32::MAX; 1 << 16];
    dist[target as usize] = 0;
    let mut q = VecDeque::new();
    q.push_back(target);
    while let Some(v) = q.pop_front() {
        let d = dist[v as usize];
        for &p in &preds[v as usize] {
            if dist[p as usize] == u32::MAX {
                dist[p as usize] = d + 1;
                q.push_back(p);
            }
        }
    }
    dist
}

/// A far, reachable target: random `L`-step chain, then require true dist(start→target) ≥ `mind`.
fn gen_far(rng: &mut Rng, t: &[Vec<u16>], preds: &[Vec<u16>], l: usize, mind: u32) -> (u16, u16) {
    loop {
        let start = rng.u16();
        let mut v = start;
        for _ in 0..l {
            v = t[rng.below(t.len())][v as usize];
        }
        if v == start {
            continue;
        }
        let d = dist_to(v, preds)[start as usize];
        if d != u32::MAX && d >= mind {
            return (start, v);
        }
    }
}

// ───────────────────────────── learned value (+ policy) ─────────────────────────────
fn feats(state: u16, target: u16) -> Vec<f32> {
    let x = state ^ target;
    let mut f = vec![0.0f32; FDIM];
    for b in 0..16 {
        f[b] = ((state >> b) & 1) as f32;
        f[16 + b] = ((target >> b) & 1) as f32;
        f[32 + b] = ((x >> b) & 1) as f32;
    }
    f
}

struct Net {
    a: usize,
    w1: Vec<Vec<f32>>,
    b1: Vec<f32>,
    w2: Vec<Vec<f32>>,
    b2: Vec<f32>,
}

impl Net {
    fn new(a: usize, hid: usize, rng: &mut Rng) -> Self {
        let s1 = (1.0 / FDIM as f32).sqrt();
        let s2 = (1.0 / hid as f32).sqrt();
        Net {
            a,
            w1: (0..hid)
                .map(|_| (0..FDIM).map(|_| rng.signed() * s1).collect())
                .collect(),
            b1: vec![0.0; hid],
            w2: (0..a + 1)
                .map(|_| (0..hid).map(|_| rng.signed() * s2).collect())
                .collect(),
            b2: vec![0.0; a + 1],
        }
    }

    fn forward(&self, x: &[f32]) -> (Vec<f32>, Vec<f32>, f32) {
        let z1: Vec<f32> = self
            .w1
            .iter()
            .zip(&self.b1)
            .map(|(row, b)| row.iter().zip(x).map(|(w, xi)| w * xi).sum::<f32>() + b)
            .collect();
        let h: Vec<f32> = z1.iter().map(|z| z.max(0.0)).collect();
        let o: Vec<f32> = self
            .w2
            .iter()
            .zip(&self.b2)
            .map(|(row, b)| row.iter().zip(&h).map(|(w, hi)| w * hi).sum::<f32>() + b)
            .collect();
        let maxl = o[..self.a].iter().copied().fold(f32::MIN, f32::max);
        let exps: Vec<f32> = o[..self.a].iter().map(|l| (l - maxl).exp()).collect();
        let sum: f32 = exps.iter().sum();
        let probs: Vec<f32> = exps.iter().map(|e| e / sum).collect();
        let value = 1.0 / (1.0 + (-o[self.a]).exp());
        (z1, probs, value)
    }

    fn train_step(&mut self, x: &[f32], action: usize, vtarget: f32, lr: f32) {
        let (z1, probs, value) = self.forward(x);
        let h: Vec<f32> = z1.iter().map(|z| z.max(0.0)).collect();
        let mut d_o = vec![0.0f32; self.a + 1];
        for k in 0..self.a {
            d_o[k] = probs[k] - if k == action { 1.0 } else { 0.0 };
        }
        d_o[self.a] = value - vtarget; // value head weight 1.0
        let mut dh = vec![0.0f32; h.len()];
        for k in 0..self.a + 1 {
            for j in 0..h.len() {
                dh[j] += d_o[k] * self.w2[k][j];
            }
        }
        for k in 0..self.a + 1 {
            for j in 0..h.len() {
                self.w2[k][j] -= lr * d_o[k] * h[j];
            }
            self.b2[k] -= lr * d_o[k];
        }
        for j in 0..h.len() {
            let dz1 = if z1[j] > 0.0 { dh[j] } else { 0.0 };
            for i in 0..FDIM {
                self.w1[j][i] -= lr * dz1 * x[i];
            }
            self.b1[j] -= lr * dz1;
        }
    }
}

/// A **dedicated** value regressor `v(s,t) ≈ dist/DNORM` (linear output, MSE) — no policy
/// head to corrupt the trunk. This is the heuristic best-first orders by (lower = closer).
struct ValueNet {
    w1: Vec<Vec<f32>>,
    b1: Vec<f32>,
    w2: Vec<f32>,
    b2: f32,
}

impl ValueNet {
    fn new(hid: usize, rng: &mut Rng) -> Self {
        let s1 = (1.0 / FDIM as f32).sqrt();
        let s2 = (1.0 / hid as f32).sqrt();
        ValueNet {
            w1: (0..hid)
                .map(|_| (0..FDIM).map(|_| rng.signed() * s1).collect())
                .collect(),
            b1: vec![0.0; hid],
            w2: (0..hid).map(|_| rng.signed() * s2).collect(),
            b2: 0.0,
        }
    }

    fn predict(&self, x: &[f32]) -> f32 {
        let h: Vec<f32> = self
            .w1
            .iter()
            .zip(&self.b1)
            .map(|(row, b)| (row.iter().zip(x).map(|(w, xi)| w * xi).sum::<f32>() + b).max(0.0))
            .collect();
        self.w2.iter().zip(&h).map(|(w, hi)| w * hi).sum::<f32>() + self.b2
    }

    fn train_step(&mut self, x: &[f32], target: f32, lr: f32) {
        let z1: Vec<f32> = self
            .w1
            .iter()
            .zip(&self.b1)
            .map(|(row, b)| row.iter().zip(x).map(|(w, xi)| w * xi).sum::<f32>() + b)
            .collect();
        let h: Vec<f32> = z1.iter().map(|z| z.max(0.0)).collect();
        let pred = self.w2.iter().zip(&h).map(|(w, hi)| w * hi).sum::<f32>() + self.b2;
        let d = pred - target;
        for j in 0..h.len() {
            let dz1 = if z1[j] > 0.0 { d * self.w2[j] } else { 0.0 };
            self.w2[j] -= lr * d * h[j];
            for i in 0..FDIM {
                self.w1[j][i] -= lr * dz1 * x[i];
            }
            self.b1[j] -= lr * dz1;
        }
        self.b2 -= lr * d;
    }
}

// ───────────────────────────────── search methods ────────────────────────────────
fn rollout(start: u16, target: u16, t: &[Vec<u16>], net: &Net, maxd: usize) -> bool {
    let mut v = start;
    let mut seen = HashSet::new();
    for _ in 0..maxd {
        if v == target {
            return true;
        }
        if !seen.insert(v) {
            break; // cycle
        }
        let probs = net.forward(&feats(v, target)).1;
        let mut bi = 0;
        for i in 1..probs.len() {
            if probs[i] > probs[bi] {
                bi = i;
            }
        }
        v = t[bi][v as usize];
    }
    v == target
}

fn bfs(start: u16, target: u16, t: &[Vec<u16>], budget: usize) -> bool {
    if start == target {
        return true;
    }
    let mut seen = HashSet::new();
    let mut q = VecDeque::new();
    seen.insert(start);
    q.push_back(start);
    let mut expanded = 0;
    while let Some(v) = q.pop_front() {
        if expanded >= budget {
            return false;
        }
        expanded += 1;
        for tab in t {
            let nv = tab[v as usize];
            if seen.insert(nv) {
                if nv == target {
                    return true;
                }
                q.push_back(nv);
            }
        }
    }
    false
}

/// Best-first ordered by a priority (higher = expand first), complete within `budget`.
fn best_first(
    start: u16,
    target: u16,
    t: &[Vec<u16>],
    budget: usize,
    prio: impl Fn(u16) -> i64,
) -> bool {
    if start == target {
        return true;
    }
    let mut seen = HashSet::new();
    seen.insert(start);
    let mut heap: BinaryHeap<(i64, u16)> = BinaryHeap::new();
    heap.push((prio(start), start));
    let mut expanded = 0;
    while let Some((_, v)) = heap.pop() {
        if v == target {
            return true;
        }
        if expanded >= budget {
            return false;
        }
        expanded += 1;
        for tab in t {
            let nv = tab[v as usize];
            if seen.insert(nv) {
                heap.push((prio(nv), nv));
            }
        }
    }
    false
}

fn mlp_mcts(start: u16, target: u16, t: &[Vec<u16>], net: &Net, budget: usize) -> bool {
    if start == target {
        return true;
    }
    let a_n = t.len();
    struct S {
        prior: Vec<f32>,
        n: Vec<u32>,
        w: Vec<f32>,
    }
    let mut tree: HashMap<u16, S> = HashMap::new();
    let c = 1.5f32;
    let mut expansions = 0;
    while expansions < budget {
        let mut v = start;
        let mut path: Vec<(u16, usize)> = Vec::new();
        let leaf_value: f32;
        loop {
            if v == target {
                return true;
            }
            if !tree.contains_key(&v) {
                let (_, probs, value) = net.forward(&feats(v, target));
                tree.insert(
                    v,
                    S {
                        prior: probs,
                        n: vec![0; a_n],
                        w: vec![0.0; a_n],
                    },
                );
                expansions += 1;
                leaf_value = value;
                break;
            }
            let s = &tree[&v];
            let total: u32 = s.n.iter().sum();
            let sqrt_total = (total as f32).sqrt();
            let mut best_a = 0;
            let mut best_u = f32::MIN;
            for a in 0..a_n {
                let q = if s.n[a] > 0 {
                    s.w[a] / s.n[a] as f32
                } else {
                    0.0
                };
                let u = q + c * s.prior[a] * sqrt_total / (1.0 + s.n[a] as f32);
                if u > best_u {
                    best_u = u;
                    best_a = a;
                }
            }
            path.push((v, best_a));
            v = t[best_a][v as usize];
            if path.len() > 60 {
                leaf_value = 0.0;
                break;
            }
        }
        for (sv, sa) in path {
            if let Some(st) = tree.get_mut(&sv) {
                st.n[sa] += 1;
                st.w[sa] += leaf_value;
            }
        }
    }
    false
}

fn main() {
    let acts = actions();
    println!(
        "building transition tables over the VM ({} lossy-bit actions × 65536 states)…",
        acts.len()
    );
    let t = tables(&acts);
    println!("building reverse adjacency…");
    let preds = reverse_adj(&t);

    const L: usize = 8;
    const MIND: u32 = 6; // far targets only — where blind search is stressed
    const T_EVAL: usize = 120;

    // ── train value net across the FULL distance range (fixes the distribution shift) ──
    println!("training the value net on BFS distances (dist 1..12)…");
    let mut trng = Rng::new(0x7A1E);
    let mut ex: Vec<(Vec<f32>, usize, f32)> = Vec::new();
    for _ in 0..500 {
        let (_, target) = gen_far(&mut trng, &t, &preds, L, MIND);
        let dist = dist_to(target, &preds);
        for _ in 0..80 {
            let s = trng.u16();
            let ds = dist[s as usize];
            if ds == 0 || ds == u32::MAX || ds > 12 {
                continue;
            }
            let mut ba = 0;
            let mut bd = u32::MAX;
            for (a, tab) in t.iter().enumerate() {
                let d = dist[tab[s as usize] as usize];
                if d < bd {
                    bd = d;
                    ba = a;
                }
            }
            ex.push((feats(s, target), ba, (1.0 - ds as f32 / DNORM).max(0.0)));
        }
    }
    let mut net = Net::new(acts.len(), 128, &mut Rng::new(0x515E));
    for _ in 0..40 {
        for i in (1..ex.len()).rev() {
            ex.swap(i, trng.below(i + 1));
        }
        for (x, a, v) in &ex {
            net.train_step(x, *a, *v, 0.1);
        }
    }
    let vmae: f32 = ex
        .iter()
        .map(|(x, _, v)| (net.forward(x).2 - v).abs())
        .sum::<f32>()
        / ex.len() as f32;
    println!("combined net (for MCTS) — value MAE {:.3}", vmae);

    // dedicated value regressor for best-first — no policy head to corrupt the trunk
    let mut vnet = ValueNet::new(128, &mut Rng::new(0x5A1D));
    for _ in 0..60 {
        for i in (1..ex.len()).rev() {
            ex.swap(i, trng.below(i + 1));
        }
        for (x, _, v) in &ex {
            vnet.train_step(x, *v, 0.05);
        }
    }
    let vmae2: f32 = ex
        .iter()
        .map(|(x, _, v)| (vnet.predict(x) - v).abs())
        .sum::<f32>()
        / ex.len() as f32;
    println!(
        "dedicated value net (for best-first) — MAE {:.3} (≈{:.1} steps)\n",
        vmae2,
        vmae2 * DNORM
    );

    // ── budget sweep where blind BFS breaks ──
    let budgets = [64usize, 128, 256, 512];
    let val_prio =
        |s: u16, tgt: u16| -> i64 { (vnet.predict(&feats(s, tgt)) * 1_000_000.0) as i64 };
    let ham_prio = |s: u16, tgt: u16| -> i64 { -((s ^ tgt).count_ones() as i64) };

    // fixed eval set
    let mut rng = Rng::new(0xEFA1);
    let inst: Vec<(u16, u16)> = (0..T_EVAL)
        .map(|_| gen_far(&mut rng, &t, &preds, L, MIND))
        .collect();

    let pct = |h: usize| 100.0 * h as f32 / T_EVAL as f32;
    let run = |f: &dyn Fn(u16, u16) -> bool| pct(inst.iter().filter(|(s, g)| f(*s, *g)).count());

    println!("Non-metric composition (far targets, dist ≥ {MIND}) — solved %, {T_EVAL} tasks\n");
    println!(
        "  {:>7}   {:>9}   {:>11}   {:>13}   {:>9}",
        "budget", "blind BFS", "Hamming-bf", "MLP value-bf", "MLP+MCTS"
    );
    println!("  {}", "-".repeat(62));
    let (mut last_ham, mut last_val) = (0.0f32, 0.0f32);
    for &b in &budgets {
        let blind = run(&|s, g| bfs(s, g, &t, b));
        let ham = run(&|s, g| best_first(s, g, &t, b, |x| ham_prio(x, g)));
        let val = run(&|s, g| best_first(s, g, &t, b, |x| val_prio(x, g)));
        let mc = run(&|s, g| mlp_mcts(s, g, &t, &net, b));
        (last_ham, last_val) = (ham, val);
        println!(
            "  {:>7}   {:>8.0}%   {:>10.0}%   {:>12.0}%   {:>8.0}%",
            b, blind, ham, val, mc
        );
    }
    let ceiling = run(&|s, g| bfs(s, g, &t, usize::MAX));
    let one_pass = run(&|s, g| rollout(s, g, &t, &net, 40));
    println!("\n  BFS unbounded (ceiling): {ceiling:.0}%    one-pass MLP rollout: {one_pass:.0}%");
    println!("\n  GATE (reproduced): one-pass {one_pass:.0}% ≪ BFS-unbounded {ceiling:.0}%, blind BFS breaks under budget");
    println!("    -> composition over lossy bit-cells is UN-FOLDABLE — search with backtracking is required.");
    let edge = last_val - last_ham;
    let verdict = if edge > 3.0 {
        "value-best-first BEATS the Hamming heuristic (the documented learned-organ win)"
    } else if edge < -3.0 {
        "value-best-first LOSES to the Hamming heuristic in this clean-room build"
    } else {
        "value-best-first ≈ Hamming heuristic — clean-room build did NOT reproduce the documented dominance"
    };
    println!(
        "  LADDER: {verdict} (val {last_val:.0}% vs Hamming {last_ham:.0}% @ budget {}).",
        budgets[budgets.len() - 1]
    );
    println!("  (Documented in chuk-soma docs/roadmap.md B3′: value-bf 28→94% ≫ Hamming, MCTS loses — banked/authoritative.)");
}
