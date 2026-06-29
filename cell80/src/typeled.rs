//! **Type-led re-ranking** (roadmap #3) — re-rank the text candidates by a signal only a
//! *typed, executable* artifact carries: what the cell actually *does*.
//!
//! Motivation: [`TfidfIndex`] ranks by words, but the confusable bounds family —
//! `range_check`, `clamp`, `between_exclusive` — shares *every* bounds word and has the
//! *identical* signature `(x, lo, hi) -> u16`, so text can't separate them. What separates
//! them is **behaviour**: `range_check`/`between_exclusive` return 0/1 (predicates), while
//! `clamp` returns a value. [`TypeLedIndex`] runs each cell on a probe bank to learn whether
//! it is a **predicate** (the label is free and language-agnostic — the verifier *is* the
//! signal), fits a **corpus-learned** predicate↔transform query intent from those labels (no
//! hardcoded vocabulary), and re-ranks: `final = cosine · (1 + β·agreement)`, bounded to only
//! re-order near-ties. Deterministic, no external model.
//!
//! **Measured honestly (`examples/retrieval_compare`, 98 cells):** the predicate axis is a
//! *wash* vs plain tf-idf on paraphrase (≈45→43 P@1, within noise). The reason is structural,
//! not a bug — of tf-idf's hard top-1 misses, the majority name a *same-shape sibling*
//! (min/max, gcd/lcm, manhattan/chebyshev) that no arity/predicate signal can tell apart, and
//! even the different-shape misses need the target shape inferred from paraphrase text, which
//! hits the same vocabulary gap. So this module stands as the principled verifier-grounded
//! re-ranker and the place to add further structural axes (arity, param names), but the
//! language-independent lever for the same-shape siblings is **behavioural I/O-example
//! routing** ([`rank_by_examples`](crate::rank_by_examples)), not more text-derived intent.
use std::collections::HashMap;

use super::{Cartridge, Manifest, Runner, TfidfIndex, DEFAULT_CYCLES};

/// A 3-arg probe bank chosen so boolean **predicates toggle** between 0 and 1. A 2-arg bank
/// (e.g. [`DEFAULT_PROBES`](crate::DEFAULT_PROBES)) leaves a 3-arg predicate like
/// `range_check(x, lo, hi)` stuck on its unused-register default (`hi = 0` → always false),
/// which would misread it as a constant. Lower-arity cells ignore the extra columns, exactly
/// as in [`Fingerprint`](crate::Fingerprint).
const PRED_PROBES: &[[u16; 3]] = &[
    [5, 0, 10],
    [15, 0, 10],
    [5, 6, 10],
    [0, 0, 10],
    [7, 1, 3],
    [2, 2, 8],
    [9, 0, 9],
    [1, 3, 5],
];

/// Whether a cell behaves like a boolean **predicate** on [`PRED_PROBES`]: every clean output
/// is 0 or 1, and *both* values occur — so a constant-0 transformer (or a cell that never
/// returns cleanly) isn't mistaken for one.
fn is_predicate(cart: &Cartridge) -> bool {
    let mut runner = Runner::new(&cart.program);
    let entry = cart.manifest.entry.as_str();
    let outs: Vec<u16> = PRED_PROBES
        .iter()
        .filter_map(|p| match runner.run(Some(entry), &p[..], DEFAULT_CYCLES) {
            Ok(r) if r.returned => Some(r.result),
            _ => None,
        })
        .collect();
    !outs.is_empty() && outs.iter().all(|&v| v <= 1) && outs.contains(&0) && outs.contains(&1)
}

/// Lowercased word tokens of a manifest's discriminating text (summary + tags + id) — the
/// vocabulary the predicate signal is *learned* over. No char-grams here: the shape cues are
/// whole words ("if", "returns", "into", "clamp"), and tf-idf already carries the morphology.
fn tokens(m: &Manifest) -> Vec<String> {
    let text = format!("{} {} {}", m.summary, m.tags.join(" "), m.id);
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// A **learned** predicate↔transform signal — no hardcoded vocabulary. The labels are free
/// (behavioural [`is_predicate`]), so the discriminating power of each manifest word is fit
/// from the corpus as a smoothed log-odds: a token in predicate manifests but not transformer
/// ones (e.g. "if", "returns") gets a positive weight; a transformer-only token ("into",
/// "clamp") a negative one; a word shared by both (bounds vocabulary) lands near zero. Grows
/// with the library and follows the manifests' own language — nothing is typed by hand.
struct ShapeAffinity {
    /// token → log-odds(predicate vs transformer). Absent tokens contribute 0.
    weight: HashMap<String, f32>,
    /// Whether both classes were present at fit time; if not, the signal is meaningless and
    /// [`intent`](Self::intent) stays 0 (pure-text fallback).
    usable: bool,
}

impl ShapeAffinity {
    fn fit(manifests: &[Manifest], predicate: &[bool]) -> Self {
        let n_pred = predicate.iter().filter(|&&p| p).count() as f32;
        let n_trans = predicate.len() as f32 - n_pred;
        if n_pred == 0.0 || n_trans == 0.0 {
            return ShapeAffinity {
                weight: HashMap::new(),
                usable: false,
            };
        }
        // Document frequency of each token within each class (count each token once per cell).
        let (mut df_pred, mut df_trans) = (HashMap::<String, f32>::new(), HashMap::new());
        for (m, &is_pred) in manifests.iter().zip(predicate) {
            let uniq: std::collections::HashSet<String> = tokens(m).into_iter().collect();
            for t in uniq {
                *if is_pred {
                    df_pred.entry(t).or_default()
                } else {
                    df_trans.entry(t).or_default()
                } += 1.0;
            }
        }
        let weight = df_pred
            .keys()
            .chain(df_trans.keys())
            .map(|t| {
                let dp = df_pred.get(t).copied().unwrap_or(0.0);
                let dt = df_trans.get(t).copied().unwrap_or(0.0);
                // Smoothed log-odds: ln P(t|pred) - ln P(t|trans).
                let lp = ((dp + 0.5) / (n_pred + 1.0)).ln();
                let lt = ((dt + 0.5) / (n_trans + 1.0)).ln();
                (t.clone(), lp - lt)
            })
            .collect();
        ShapeAffinity {
            weight,
            usable: true,
        }
    }

    /// Query intent in `[-1, 1]`: sum the learned weights of the query's known tokens, squashed.
    /// Unknown tokens (not in any manifest) contribute 0 — the honest paraphrase ceiling.
    fn intent(&self, query: &str) -> f32 {
        if !self.usable {
            return 0.0;
        }
        let sum: f32 = query
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| !t.is_empty())
            .filter_map(|t| self.weight.get(&t.to_lowercase()))
            .sum();
        (sum / Self::SQUASH).tanh()
    }

    /// Divisor before `tanh`, so a few strongly-skewed tokens approach ±1 without one word
    /// saturating the signal. Tuned against `examples/retrieval_compare`.
    const SQUASH: f32 = 6.0;
}

/// A type-led search index: [`TfidfIndex`] for the text signal, plus a per-cell behavioural
/// **predicate** flag used to re-rank. Built from cartridges (not just manifests) because the
/// structural signal comes from *running* each cell.
pub struct TypeLedIndex {
    text: TfidfIndex,
    /// `id -> is_predicate` (behavioural).
    predicate: HashMap<String, bool>,
    /// Corpus-learned query intent on the predicate↔transform axis.
    affinity: ShapeAffinity,
}

impl TypeLedIndex {
    /// Re-rank strength: `final = cosine · (1 + BETA · agreement)`, `agreement ∈ [-1, 1]`.
    /// Bounded so a perfect structural agreement scales a score by ×1.5 and a perfect
    /// disagreement by ×0.5 — enough to flip near-ties, not enough to beat a clear text win.
    const BETA: f32 = 0.5;

    /// Build the index: fingerprint each cell's predicate behaviour (O(n) probe runs — cheap),
    /// fit TF-IDF over the manifests, and fit the corpus-learned predicate-intent model from
    /// the behavioural labels. No hand-tuned vocabulary anywhere.
    pub fn build(carts: Vec<Cartridge>) -> Self {
        let manifests: Vec<Manifest> = carts.iter().map(|c| c.manifest.clone()).collect();
        let labels: Vec<bool> = carts.iter().map(is_predicate).collect();
        let predicate: HashMap<String, bool> = manifests
            .iter()
            .zip(&labels)
            .map(|(m, &p)| (m.id.clone(), p))
            .collect();
        let affinity = ShapeAffinity::fit(&manifests, &labels);
        TypeLedIndex {
            text: TfidfIndex::build(manifests),
            predicate,
            affinity,
        }
    }

    pub fn len(&self) -> usize {
        self.text.len()
    }
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Rank `query`'s positive-cosine text candidates, re-scored by predicate agreement.
    /// Best first, ties broken by id, up to `limit`.
    pub fn search(&self, query: &str, limit: usize) -> Vec<&Manifest> {
        let intent = self.affinity.intent(query);
        let mut cands: Vec<(f32, &Manifest)> = self
            .text
            .scored(query, usize::MAX)
            .into_iter()
            .map(|(cos, m)| {
                let agree = match self.predicate.get(&m.id) {
                    Some(true) => intent,   // cell is a predicate: agrees with predicate intent
                    Some(false) => -intent, // transformer: agrees with transform intent
                    None => 0.0,
                };
                (cos * (1.0 + Self::BETA * agree), m)
            })
            .collect();
        cands.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.id.cmp(&b.1.id))
        });
        cands.into_iter().take(limit).map(|(_, m)| m).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CartridgeOpts, CellConfig};

    fn cell(id: &str, summary: &str, src: &str) -> Cartridge {
        Cartridge::compile(
            src,
            CellConfig::sandboxed(),
            CartridgeOpts {
                id: Some(id.into()),
                summary: summary.into(),
                ..Default::default()
            },
        )
        .unwrap()
    }

    // range_check and clamp share the identical signature (x,lo,hi)->u16 and bounds vocabulary;
    // only behaviour (and the words the manifests *naturally* use) separate them.
    fn range_check() -> Cartridge {
        cell(
            "range_check",
            "returns 1 if the value is inside the bounds",
            "fn run(x: u16, lo: u16, hi: u16) -> u16 { ((lo <= x) && (x <= hi)) as u16 }",
        )
    }
    fn clamp() -> Cartridge {
        cell(
            "clamp",
            "limit the value into the bounds keeping it inside",
            "fn run(x: u16, lo: u16, hi: u16) -> u16 { let mut r = x; if r < lo { r = lo; } if r > hi { r = hi; } r }",
        )
    }
    fn is_even() -> Cartridge {
        cell(
            "is_even",
            "returns 1 if the number is even",
            "fn run(n: u16) -> u16 { (n % 2u16 == 0u16) as u16 }",
        )
    }
    fn add_sat() -> Cartridge {
        cell(
            "add_sat",
            "add two numbers into a sum",
            "fn run(a: u16, b: u16) -> u16 { a + b }",
        )
    }

    /// A small mixed library so the affinity model has both classes to learn from.
    fn lib() -> Vec<Cartridge> {
        vec![range_check(), is_even(), clamp(), add_sat()]
    }

    #[test]
    fn predicate_detection_separates_identical_signatures() {
        // Same signature (x,lo,hi)->u16, opposite behaviour — the case text can't see.
        assert!(is_predicate(&range_check()), "range_check returns 0/1");
        assert!(!is_predicate(&clamp()), "clamp returns a value, not 0/1");
    }

    #[test]
    fn intent_is_learned_from_the_corpus_not_hardcoded() {
        let carts = lib();
        let manifests: Vec<Manifest> = carts.iter().map(|c| c.manifest.clone()).collect();
        let labels: Vec<bool> = carts.iter().map(is_predicate).collect();
        let aff = ShapeAffinity::fit(&manifests, &labels);
        // "is"/"returns" occur only in the predicate manifests → predicate-leaning;
        // "limit"/"into"/"add" only in the transformers → transform-leaning.
        assert!(aff.intent("is the value inside the bounds") > 0.0);
        assert!(aff.intent("limit the value into the bounds") < 0.0);
        // A word in no manifest contributes nothing — the honest paraphrase ceiling.
        assert_eq!(aff.intent("xyzzy"), 0.0);
    }

    #[test]
    fn type_led_lifts_the_predicate_for_a_predicate_query() {
        let idx = TypeLedIndex::build(lib());
        // A yes/no bounds query should prefer the predicate over the same-shaped transformer.
        assert_eq!(
            idx.search("is the value inside the bounds", 2)[0].id,
            "range_check"
        );
        // And a transform query should prefer the transformer.
        assert_eq!(
            idx.search("limit the value into the bounds", 2)[0].id,
            "clamp"
        );
    }
}
