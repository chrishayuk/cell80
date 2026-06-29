//! A learned, verifier-grounded **cell selector** — routing decided *here, in the cell
//! layer*, not by the LLM. The LLM writes the request; this picks the cell.
//!
//! The design is ported from the SOMA Track-A experiments (`chuk-soma`, V3b/V3-CP), whose
//! findings transfer directly:
//!
//! * **V3-CP (continual stability):** one **frozen MLP slot per cell**, trained when the
//!   cell is added and never touched again. Adding a cell is O(1) and *cannot* disturb the
//!   cells already learned — the property a growing library (or one monolithic softmax over
//!   all cells) can't have. This is why selection must not live in one big model.
//! * **V3b (designation + honest abstention):** when two cells are *behaviourally identical*
//!   (their verifier fingerprints agree), no signal can separate them, so the router
//!   **abstains** (returns both) rather than guessing.
//!
//! cell80's edge over the SOMA toy: labels there were oracle-authored; here the **verifier
//! is free**, so a cell's [`Fingerprint`](crate::Fingerprint) is a real, abundant signal —
//! used to mine hard negatives (the confusable siblings) and to drive the abstain decision.
//!
//! Deterministic and dependency-free (hand-written MLP + SGD, seeded PRNG), matching the
//! rest of cell80 — no external model, reproducible.
use std::collections::HashMap;

use crate::Fingerprint;

// ── deterministic PRNG (SplitMix64) ───────────────────────────────────────────────
/// A small seeded PRNG so slot initialisation and training order are reproducible (I7).
#[derive(Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform `f32` in `[-1.0, 1.0)`.
    pub fn signed(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32 * 2.0 - 1.0
    }
    /// Uniform index in `[0, n)`.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n.max(1) as u64) as usize
    }
}

// ── text features (hashing trick over idf-weighted tokens + char-3-grams) ──────────
fn tokens(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_lowercase)
        .collect()
}

fn char_ngrams(s: &str, n: usize) -> Vec<String> {
    let chars: Vec<char> = s.to_lowercase().chars().collect();
    if chars.len() < n {
        let w: String = chars.into_iter().collect();
        return if w.is_empty() { vec![] } else { vec![w] };
    }
    chars.windows(n).map(|w| w.iter().collect()).collect()
}

fn feats(text: &str) -> Vec<String> {
    let mut f: Vec<String> = tokens(text).into_iter().map(|t| format!("t:{t}")).collect();
    f.extend(char_ngrams(text, 3).into_iter().map(|g| format!("g:{g}")));
    f
}

fn hash_str(s: &str) -> u64 {
    let mut h: u64 = 1469598103934665603;
    for b in s.bytes() {
        h = (h ^ b as u64).wrapping_mul(1099511628211);
    }
    h
}

/// A fixed-width text encoder: idf-weighted token + char-3-gram features hashed (with a
/// sign bit, to de-bias collisions) into a dense `dim`-vector, L2-normalised so the MLP
/// sees a bounded input. idf is fit over a corpus so generic words contribute little.
#[derive(Clone)]
pub struct Encoder {
    dim: usize,
    idf: HashMap<String, f32>,
}

impl Encoder {
    /// Fit idf over `corpus` (the cell manifests + any seed queries). `idf = ln((N+1)/(df+1)) + 1`.
    pub fn fit(dim: usize, corpus: &[String]) -> Self {
        let mut df: HashMap<String, u32> = HashMap::new();
        for doc in corpus {
            let mut seen: Vec<String> = feats(doc);
            seen.sort_unstable();
            seen.dedup();
            for f in seen {
                *df.entry(f).or_insert(0) += 1;
            }
        }
        let n = corpus.len() as f32;
        let idf = df
            .into_iter()
            .map(|(f, d)| (f, ((n + 1.0) / (d as f32 + 1.0)).ln() + 1.0))
            .collect();
        Encoder { dim, idf }
    }

    /// Encode `text` into a dense, L2-normalised feature vector. Unseen features get idf 1.0.
    pub fn encode(&self, text: &str) -> Vec<f32> {
        let mut v = vec![0.0f32; self.dim];
        for f in feats(text) {
            let w = *self.idf.get(&f).unwrap_or(&1.0);
            let h = hash_str(&f);
            let bucket = (h % self.dim as u64) as usize;
            let sign = if (h >> 63) & 1 == 1 { 1.0 } else { -1.0 };
            v[bucket] += sign * w;
        }
        let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut v {
                *x /= norm;
            }
        }
        v
    }
}

// ── one frozen MLP slot per cell: in → hidden(ReLU) → 1 (sigmoid) ──────────────────
#[derive(Clone)]
struct Slot {
    w1: Vec<Vec<f32>>, // [hid][in]
    b1: Vec<f32>,      // [hid]
    w2: Vec<f32>,      // [hid]
    b2: f32,
}

impl Slot {
    fn new(n_in: usize, hid: usize, rng: &mut Rng) -> Self {
        let scale = (1.0 / n_in as f32).sqrt();
        let w1 = (0..hid)
            .map(|_| (0..n_in).map(|_| rng.signed() * scale).collect())
            .collect();
        let w2 = (0..hid)
            .map(|_| rng.signed() * (1.0 / hid as f32).sqrt())
            .collect();
        Slot {
            w1,
            b1: vec![0.0; hid],
            w2,
            b2: 0.0,
        }
    }

    /// Hidden pre-activations and the sigmoid score.
    fn forward(&self, x: &[f32]) -> (Vec<f32>, f32) {
        let z1: Vec<f32> = self
            .w1
            .iter()
            .zip(&self.b1)
            .map(|(row, b)| row.iter().zip(x).map(|(w, xi)| w * xi).sum::<f32>() + b)
            .collect();
        let h: Vec<f32> = z1.iter().map(|&z| z.max(0.0)).collect();
        let z2 = self.w2.iter().zip(&h).map(|(w, hi)| w * hi).sum::<f32>() + self.b2;
        (z1, sigmoid(z2))
    }

    fn score(&self, x: &[f32]) -> f32 {
        self.forward(x).1
    }

    /// One binary-cross-entropy SGD step toward `target` ∈ {0,1}.
    fn train_step(&mut self, x: &[f32], target: f32, lr: f32) {
        let (z1, p) = self.forward(x);
        let h: Vec<f32> = z1.iter().map(|&z| z.max(0.0)).collect();
        let dz2 = p - target; // d(BCE)/d(z2) with sigmoid
        for (j, hj) in h.iter().enumerate() {
            let dh = dz2 * self.w2[j];
            let dz1 = if z1[j] > 0.0 { dh } else { 0.0 };
            self.w2[j] -= lr * dz2 * hj;
            for (i, xi) in x.iter().enumerate() {
                self.w1[j][i] -= lr * dz1 * xi;
            }
            self.b1[j] -= lr * dz1;
        }
        self.b2 -= lr * dz2;
    }
}

fn sigmoid(z: f32) -> f32 {
    1.0 / (1.0 + (-z).exp())
}

// ── the router: per-cell frozen slots + verifier-grounded abstain ──────────────────
struct CellSlot {
    id: String,
    fp: Option<Fingerprint>,
    slot: Slot,
}

/// What the router decided for a query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Routed {
    /// A single confident pick.
    Cell(String),
    /// Top candidates are behaviourally indistinguishable (verifier fingerprints agree) and
    /// score within `eps` — the honest "I can't tell these apart, here are both" (V3b).
    Abstain(Vec<String>),
    /// No slot scored positively.
    None,
}

/// Turn a behavioural [`Fingerprint`] into a fixed-width feature vector for a slot: one
/// feature per probe, `output / scale[i]` (`-1.0` for a non-returning probe). Pass per-probe
/// `scales` (e.g. the max output seen at that probe across the library) so features stay
/// bounded regardless of the raw integer magnitudes — this is how the verifier's behavioural
/// signal becomes a learnable input.
pub fn fingerprint_features(fp: &Fingerprint, scales: &[f32]) -> Vec<f32> {
    fp.outputs
        .iter()
        .enumerate()
        .map(|(i, o)| match o {
            Some(v) => *v as f32 / scales.get(i).copied().unwrap_or(1.0).max(1.0),
            None => -1.0,
        })
        .collect()
}

/// A learned cell selector: one frozen MLP slot per cell (V3-CP), with verifier
/// fingerprints driving honest abstention (V3b).
pub struct SlotRouter {
    dim: usize,
    hid: usize,
    cells: Vec<CellSlot>,
}

impl SlotRouter {
    pub fn new(dim: usize, hid: usize) -> Self {
        SlotRouter {
            dim,
            hid,
            cells: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Train and **freeze** a slot for one cell (the V3-CP continual-growth op: existing
    /// slots are untouched). `pos` are encoded positive queries for this cell; `negs` are
    /// encoded negatives (other cells' queries — confusable siblings make the best ones).
    /// `fp` is the cell's verifier fingerprint, used later for abstention.
    #[allow(clippy::too_many_arguments)]
    pub fn add_cell(
        &mut self,
        id: &str,
        fp: Option<Fingerprint>,
        pos: &[Vec<f32>],
        negs: &[Vec<f32>],
        epochs: usize,
        lr: f32,
        rng: &mut Rng,
    ) {
        let mut slot = Slot::new(self.dim, self.hid, rng);
        // Balance: each epoch, every positive once, plus an equal number of sampled negatives.
        for _ in 0..epochs {
            for x in pos {
                slot.train_step(x, 1.0, lr);
            }
            for _ in 0..pos.len().max(1) {
                if !negs.is_empty() {
                    slot.train_step(&negs[rng.below(negs.len())], 0.0, lr);
                }
            }
        }
        self.cells.push(CellSlot {
            id: id.to_string(),
            fp,
            slot,
        });
    }

    /// Every cell's score for `feat`, best first (ties broken by id for determinism).
    pub fn rank(&self, feat: &[f32]) -> Vec<(&str, f32)> {
        let mut scored: Vec<(&str, f32)> = self
            .cells
            .iter()
            .map(|c| (c.id.as_str(), c.slot.score(feat)))
            .collect();
        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(b.0))
        });
        scored
    }

    /// The single best cell, or `None` if nothing scored above 0.5.
    pub fn top(&self, feat: &[f32]) -> Option<&str> {
        self.rank(feat)
            .into_iter()
            .find(|(_, s)| *s > 0.5)
            .map(|(id, _)| id)
    }

    /// Route with **honest abstention**: if the top two are within `eps` *and* their
    /// verifier fingerprints are identical, no signal can separate them, so return both.
    pub fn route(&self, feat: &[f32], eps: f32) -> Routed {
        let ranked = self.rank(feat);
        let Some(&(top_id, top_score)) = ranked.first() else {
            return Routed::None;
        };
        if top_score <= 0.5 {
            return Routed::None;
        }
        if let Some(&(snd_id, snd_score)) = ranked.get(1) {
            if top_score - snd_score < eps && self.identical_behaviour(top_id, snd_id) {
                let mut both = vec![top_id.to_string(), snd_id.to_string()];
                both.sort();
                return Routed::Abstain(both);
            }
        }
        Routed::Cell(top_id.to_string())
    }

    fn fingerprint(&self, id: &str) -> Option<&Fingerprint> {
        self.cells
            .iter()
            .find(|c| c.id == id)
            .and_then(|c| c.fp.as_ref())
    }

    /// Whether two cells are behaviourally indistinguishable on their fingerprints.
    fn identical_behaviour(&self, a: &str, b: &str) -> bool {
        match (self.fingerprint(a), self.fingerprint(b)) {
            (Some(fa), Some(fb)) => fa.agreement(fb) >= 1.0,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A toy 2-class feature space: positives cluster near +x, negatives near -x.
    fn cluster(center: f32, rng: &mut Rng, n: usize, dim: usize) -> Vec<Vec<f32>> {
        (0..n)
            .map(|_| {
                let mut v = vec![0.0f32; dim];
                v[0] = center + rng.signed() * 0.1;
                v[1] = rng.signed() * 0.1;
                v
            })
            .collect()
    }

    #[test]
    fn a_slot_learns_to_separate_two_clusters() {
        let mut rng = Rng::new(1);
        let dim = 4;
        let mut r = SlotRouter::new(dim, 8);
        let pos = cluster(1.0, &mut rng, 20, dim);
        let neg = cluster(-1.0, &mut rng, 20, dim);
        r.add_cell("pos", None, &pos, &neg, 200, 0.2, &mut rng);
        // A fresh positive-side point scores high; a negative-side point scores low.
        assert!(
            r.cells[0].slot.score(&{
                let mut v = vec![0.0; dim];
                v[0] = 1.0;
                v
            }) > 0.7
        );
        assert!(
            r.cells[0].slot.score(&{
                let mut v = vec![0.0; dim];
                v[0] = -1.0;
                v
            }) < 0.3
        );
    }

    #[test]
    fn encoder_is_deterministic_and_normalised() {
        let enc = Encoder::fit(
            64,
            &[
                "the smaller of two numbers".into(),
                "the larger of two numbers".into(),
            ],
        );
        let a = enc.encode("the smaller of two numbers");
        assert_eq!(a, enc.encode("the smaller of two numbers"));
        let norm = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }

    #[test]
    fn abstains_when_two_cells_share_a_fingerprint() {
        // Two cells with identical behaviour (same fingerprint) and near-identical text →
        // the router must return both, not guess.
        let dim = 8;
        let mut rng = Rng::new(7);
        let enc = Encoder::fit(dim, &["duplicate of a cell".into()]);
        let fp = Fingerprint {
            outputs: vec![Some(1), Some(2), Some(3)],
        };
        let q: Vec<Vec<f32>> = vec![enc.encode("a duplicated behaviour")];
        let other = vec![enc.encode("something unrelated entirely")];
        let mut r = SlotRouter::new(dim, 8);
        r.add_cell("dup_a", Some(fp.clone()), &q, &other, 300, 0.2, &mut rng);
        r.add_cell("dup_b", Some(fp), &q, &other, 300, 0.2, &mut rng);
        match r.route(&enc.encode("a duplicated behaviour"), 0.2) {
            Routed::Abstain(ids) => assert_eq!(ids, vec!["dup_a".to_string(), "dup_b".to_string()]),
            other => panic!("expected abstain, got {other:?}"),
        }
    }
}
