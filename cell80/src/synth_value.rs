//! **Learned value heuristic for synthesis (gated)** — the trained rider for the
//! [`synthesize_with`](crate::synthesize_with) seam.
//!
//! [`ValueHeuristic::train`] fits a small distance net over an op-set's *own* transition
//! tables: sample reachable training targets, BFS the true distance-to-target field over
//! the u16 domain (the ops are precomputed tables, so this is exact), and train a softmax
//! head over **distance bins** `0..=16` on states drawn from *every* layer — near, far,
//! and unreachable alike. The search priority is the **expected bin** (expected steps to
//! the target), summed over the examples.
//!
//! Why bins over the full range, and not a saturating "closeness" score: search lives on
//! the **far frontier**, and a target that clips everything beyond `d` steps to the same
//! value cannot rank that frontier at all. This was measured, not guessed — the
//! reproduction experiment (chuk-soma `experiments/setb/value_search.rs`, on this crate's
//! real cells) has the full-range bins head beating the hand Hamming heuristic at **every**
//! node budget (30→83% vs 13→62%, budgets 64→2048), while a control trained with the
//! saturating recipe *loses* to Hamming everywhere. Notably the control had the *best*
//! mid-range MAE — value accuracy where the data is dense does not buy search utility;
//! far-frontier ordering does.
//!
//! **Still gated at the seam.** [`synthesize`](crate::synthesize) stays hand-Hamming by
//! default; this heuristic earns in per op-family via `examples/synth_value_gate.rs`
//! (learned vs hand at equal budget, pre-registered tie band, raw report). On smooth or
//! shallow op-sets the hand heuristic is expected to tie or win — don't pay the training
//! cost there.
//!
//! Training cost is a one-time, per-op-family build (seconds to ~a minute at the default
//! config: reverse adjacency over the u16 domain + a few hundred BFS fields + SGD), meant
//! to be amortised like an index. Everything is deterministic in the config seed.

use std::collections::VecDeque;

use crate::synth::Op;

/// Feature layout per (state, target) pair: state bits, target bits, xor bits.
const FDIM: usize = 48;
/// Distance-bin head: classes `0..=DCAP`, top bin = "`>= DCAP` steps or unreachable".
const DCAP: usize = 16;
/// Fixed-point scale for the i64 heuristic (1/16th-step resolution). At this scale the
/// heuristic dominates A*'s `g` term, so the search behaves as (greedy) best-first on the
/// learned value — the regime the gate experiment validated.
const H_SCALE: f32 = 16.0;

// ─────────────────────────── deterministic rng (splitmix64) ───────────────────────────

struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn u16(&mut self) -> u16 {
        (self.next_u64() & 0xFFFF) as u16
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n.max(1) as u64) as usize
    }
    fn signed_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32 * 2.0 - 1.0
    }
}

// ─────────────────────────────── the tiny bins net ───────────────────────────────

/// Minimal MLP softmax classifier (FDIM → hid ReLU → DCAP+1), dependency-free.
struct BinsNet {
    hid: usize,
    w1: Vec<f32>, // hid × FDIM
    b1: Vec<f32>,
    w2: Vec<f32>, // (DCAP+1) × hid
    b2: Vec<f32>,
}

impl BinsNet {
    fn new(hid: usize, rng: &mut Rng) -> Self {
        let s1 = (1.0 / FDIM as f32).sqrt();
        let s2 = (1.0 / hid as f32).sqrt();
        BinsNet {
            hid,
            w1: (0..hid * FDIM).map(|_| rng.signed_f32() * s1).collect(),
            b1: vec![0.0; hid],
            w2: (0..(DCAP + 1) * hid)
                .map(|_| rng.signed_f32() * s2)
                .collect(),
            b2: vec![0.0; DCAP + 1],
        }
    }

    fn forward(&self, x: &[f32]) -> (Vec<f32>, Vec<f32>) {
        let h: Vec<f32> = (0..self.hid)
            .map(|j| {
                let row = &self.w1[j * FDIM..(j + 1) * FDIM];
                (row.iter().zip(x).map(|(w, xi)| w * xi).sum::<f32>() + self.b1[j]).max(0.0)
            })
            .collect();
        let logits: Vec<f32> = (0..=DCAP)
            .map(|k| {
                let row = &self.w2[k * self.hid..(k + 1) * self.hid];
                row.iter().zip(&h).map(|(w, hi)| w * hi).sum::<f32>() + self.b2[k]
            })
            .collect();
        (h, logits)
    }

    /// Expected distance bin under the softmax — the value estimate, in steps.
    fn expected_steps(&self, x: &[f32]) -> f32 {
        let (_, logits) = self.forward(x);
        let m = logits.iter().copied().fold(f32::MIN, f32::max);
        let exps: Vec<f32> = logits.iter().map(|l| (l - m).exp()).collect();
        let sum: f32 = exps.iter().sum();
        exps.iter()
            .enumerate()
            .map(|(k, e)| k as f32 * e / sum)
            .sum()
    }

    fn train_step(&mut self, x: &[f32], y: usize, lr: f32) {
        let (h, logits) = self.forward(x);
        let m = logits.iter().copied().fold(f32::MIN, f32::max);
        let exps: Vec<f32> = logits.iter().map(|l| (l - m).exp()).collect();
        let sum: f32 = exps.iter().sum();
        let mut dlogit: Vec<f32> = exps.iter().map(|e| e / sum).collect();
        dlogit[y] -= 1.0;
        let mut dh = vec![0.0f32; self.hid];
        for (k, &dl) in dlogit.iter().enumerate() {
            let row = &mut self.w2[k * self.hid..(k + 1) * self.hid];
            for (j, (w, hj)) in row.iter_mut().zip(&h).enumerate() {
                dh[j] += dl * *w;
                *w -= lr * dl * hj;
            }
            self.b2[k] -= lr * dl;
        }
        for j in 0..self.hid {
            if h[j] <= 0.0 {
                continue; // ReLU gate
            }
            let row = &mut self.w1[j * FDIM..(j + 1) * FDIM];
            for (w, xi) in row.iter_mut().zip(x) {
                *w -= lr * dh[j] * xi;
            }
            self.b1[j] -= lr * dh[j];
        }
    }
}

fn feats(state: u16, target: u16) -> [f32; FDIM] {
    let x = state ^ target;
    let mut f = [0.0f32; FDIM];
    for b in 0..16 {
        f[b] = ((state >> b) & 1) as f32;
        f[16 + b] = ((target >> b) & 1) as f32;
        f[32 + b] = ((x >> b) & 1) as f32;
    }
    f
}

// ─────────────────────────────────── training ───────────────────────────────────

/// Training configuration for [`ValueHeuristic::train`]. The defaults are the gate
/// experiment's values; they are a build cost (per op-family), not a per-call cost.
#[derive(Debug, Clone)]
pub struct ValueTrainConfig {
    /// Deterministic seed (targets, sampling, init).
    pub seed: u64,
    /// Reachable training targets to sample (each costs one BFS over the u16 domain).
    pub targets: usize,
    /// States sampled per distance layer per target (layers `1..=16` + far/unreachable).
    pub per_layer: usize,
    /// Random-walk length used to sample reachable targets.
    pub walk: usize,
    /// SGD epochs / learning rate / hidden width.
    pub epochs: usize,
    pub lr: f32,
    pub hidden: usize,
}

impl Default for ValueTrainConfig {
    fn default() -> Self {
        ValueTrainConfig {
            seed: 0xB3B1_75EE,
            targets: 200,
            per_layer: 6,
            walk: 8,
            epochs: 30,
            lr: 0.05,
            hidden: 128,
        }
    }
}

/// A trained per-op-family value heuristic for [`synthesize_with`](crate::synthesize_with).
///
/// Build once per op-set with [`train`](Self::train); plug in with
/// `synthesize_with(examples, ops, depth, budget, &|s, t| vh.h(s, t))` or the
/// [`synthesize`](Self::synthesize) convenience. `held_out_mae` is the net's distance
/// error in steps on targets excluded from training (a quality signal, **not** the gate —
/// the gate is end-to-end solve rate vs the hand heuristic; see the module doc).
pub struct ValueHeuristic {
    net: BinsNet,
    /// Distance MAE (steps, bins capped at 16) on held-out targets.
    pub held_out_mae: f32,
}

/// Predecessor lists over the u16 domain for a set of ops (reverse adjacency).
fn reverse_adj(ops: &[Op]) -> Vec<Vec<u16>> {
    let mut preds: Vec<Vec<u16>> = vec![Vec::new(); 1 << 16];
    for op in ops {
        for v in 0..=u16::MAX {
            preds[op.apply(v) as usize].push(v);
        }
    }
    preds
}

/// True distance-to-target field (BFS over predecessors); `u32::MAX` = unreachable.
fn dist_field(target: u16, preds: &[Vec<u16>]) -> Vec<u32> {
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

/// Per-layer (feature, bin) examples from one target's field — full-range coverage,
/// including the far/unreachable bucket (top bin). This coverage is the load-bearing part
/// of the recipe (see the module doc).
fn layered_examples(
    rng: &mut Rng,
    target: u16,
    field: &[u32],
    per_layer: usize,
) -> Vec<([f32; FDIM], usize)> {
    let mut layers: Vec<Vec<u16>> = vec![Vec::new(); DCAP + 1]; // index = bin - 1
    for (s, &d) in field.iter().enumerate() {
        let bin = match d {
            0 => continue,
            d if (d as usize) < DCAP => d as usize,
            _ => DCAP, // >= DCAP or unreachable
        };
        layers[bin - 1].push(s as u16);
    }
    let mut out = Vec::new();
    for (i, layer) in layers.iter().enumerate() {
        if layer.is_empty() {
            continue;
        }
        for _ in 0..per_layer {
            let s = layer[rng.below(layer.len())];
            out.push((feats(s, target), i + 1));
        }
    }
    out
}

impl ValueHeuristic {
    /// Train a value heuristic on `ops`' own transition structure. Deterministic in
    /// `cfg.seed`. Panics if `ops` is empty.
    pub fn train(ops: &[Op], cfg: &ValueTrainConfig) -> Self {
        assert!(
            !ops.is_empty(),
            "cannot train a value heuristic on zero ops"
        );
        let mut rng = Rng(cfg.seed);
        let preds = reverse_adj(ops);

        // Reachable targets = random-walk endpoints; ~10% held out for the MAE signal.
        let mut targets = Vec::new();
        let mut seen = std::collections::HashSet::new();
        while targets.len() < cfg.targets {
            let mut v = rng.u16();
            for _ in 0..cfg.walk {
                v = ops[rng.below(ops.len())].apply(v);
            }
            if seen.insert(v) {
                targets.push(v);
            }
        }
        let holdout = (cfg.targets / 10).max(1);
        let (held, trained) = targets.split_at(holdout);

        let mut ex: Vec<([f32; FDIM], usize)> = Vec::new();
        for &t in trained {
            let field = dist_field(t, &preds);
            ex.extend(layered_examples(&mut rng, t, &field, cfg.per_layer));
        }
        let mut net = BinsNet::new(cfg.hidden, &mut Rng(cfg.seed ^ 0x5EED));
        let mut idx: Vec<usize> = (0..ex.len()).collect();
        for _ in 0..cfg.epochs {
            for i in (1..idx.len()).rev() {
                idx.swap(i, rng.below(i + 1));
            }
            for &i in &idx {
                let (x, bin) = &ex[i];
                net.train_step(x, *bin, cfg.lr);
            }
        }

        let mut err = 0.0f32;
        let mut n = 0usize;
        for &t in held {
            let field = dist_field(t, &preds);
            for (x, bin) in layered_examples(&mut rng, t, &field, 2) {
                err += (net.expected_steps(&x) - bin as f32).abs();
                n += 1;
            }
        }
        ValueHeuristic {
            net,
            held_out_mae: if n > 0 { err / n as f32 } else { f32::NAN },
        }
    }

    /// The heuristic value for a joint search state: expected remaining steps summed over
    /// the (state, target) example pairs, in 1/16th-step fixed point (lower = closer) —
    /// plug directly into [`synthesize_with`](crate::synthesize_with).
    pub fn h(&self, state: &[u16], targets: &[u16]) -> i64 {
        state
            .iter()
            .zip(targets)
            .map(|(&s, &t)| (self.net.expected_steps(&feats(s, t)) * H_SCALE) as i64)
            .sum()
    }

    /// [`crate::synthesize`] with this learned heuristic in the seam.
    pub fn synthesize(
        &self,
        examples: &[(u16, u16)],
        ops: &[Op],
        max_depth: usize,
        budget: usize,
    ) -> Option<crate::Plan> {
        crate::synthesize_with(examples, ops, max_depth, budget, &|s, t| self.h(s, t))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Cartridge, CartridgeOpts, CellConfig};

    fn cell(id: &str, src: &str) -> Cartridge {
        Cartridge::compile(
            src,
            CellConfig::sandboxed(),
            CartridgeOpts {
                id: Some(id.into()),
                ..Default::default()
            },
        )
        .unwrap()
    }

    fn lossy_ops() -> Vec<Op> {
        let xor = cell("mask_xor", "fn run(a: u16, b: u16) -> u16 { a ^ b }");
        let and = cell("mask_and", "fn run(a: u16, b: u16) -> u16 { a & b }");
        let or = cell("mask_or", "fn run(a: u16, b: u16) -> u16 { a | b }");
        let swap = cell(
            "swap_bytes",
            "fn run(x: u16) -> u16 { (x << 8u16) | (x >> 8u16) }",
        );
        vec![
            Op::from_cell("xor_0f0f", &xor, 0x0F0F),
            Op::from_cell("and_ff00", &and, 0xFF00),
            Op::from_cell("or_00ff", &or, 0x00FF),
            Op::from_cell("swap", &swap, 0),
        ]
    }

    /// A tiny training config so the test suite stays fast (quality is the gate
    /// example's job; these tests check wiring and determinism).
    fn tiny() -> ValueTrainConfig {
        ValueTrainConfig {
            targets: 12,
            per_layer: 3,
            epochs: 4,
            hidden: 32,
            ..Default::default()
        }
    }

    #[test]
    fn trains_and_still_solves_through_the_seam() {
        let ops = lossy_ops();
        let vh = ValueHeuristic::train(&ops, &tiny());
        assert!(vh.held_out_mae.is_finite());

        // Hidden 2-step chain; the learned heuristic must still find *a* satisfying chain
        // (correctness of the seam — dominance over the hand heuristic is the gate
        // example, not this test).
        let hidden = |v: u16| ops[3].apply(ops[0].apply(v)); // xor_0f0f then swap
        let examples: Vec<(u16, u16)> =
            [7u16, 300, 41000].iter().map(|&x| (x, hidden(x))).collect();
        let plan = vh.synthesize(&examples, &ops, 4, 50_000).expect("solves");
        let run = |steps: &[String], mut v: u16| {
            for s in steps {
                v = ops.iter().find(|o| &o.name == s).unwrap().apply(v);
            }
            v
        };
        for &(x, y) in &examples {
            assert_eq!(run(&plan.steps, x), y);
        }
    }

    #[test]
    fn deterministic_in_the_seed() {
        let ops = lossy_ops();
        let a = ValueHeuristic::train(&ops, &tiny());
        let b = ValueHeuristic::train(&ops, &tiny());
        assert_eq!(a.h(&[0x1234], &[0xABCD]), b.h(&[0x1234], &[0xABCD]));
        assert_eq!(a.held_out_mae, b.held_out_mae);
    }

    #[test]
    fn goal_scores_below_far_states() {
        let ops = lossy_ops();
        let vh = ValueHeuristic::train(&ops, &tiny());
        // At the goal the expected remaining steps must be small relative to far pairs
        // (averaged — a tiny net is noisy per-pair).
        let goal: i64 = (0..50u16).map(|i| vh.h(&[i * 1301], &[i * 1301])).sum();
        let far: i64 = (0..50u16)
            .map(|i| vh.h(&[i * 1301], &[!(i * 1301) ^ 0x5A5A]))
            .sum();
        assert!(
            goal < far,
            "goal states must rank ahead of far states (goal {goal} vs far {far})"
        );
    }
}
