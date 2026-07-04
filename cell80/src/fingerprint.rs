//! **Behavioural fingerprints** — the verifier-grounded retrieval signal a text index
//! can't see. Two cells whose *manifests collide* ("the ? of two numbers" → `min` vs
//! `max`) have *different behaviour*: on the probe `(3, 7)` one returns 3, the other 7.
//! Running a cell on a fixed probe bank (on the deterministic VM) turns that behaviour
//! into a comparable vector, so confusable cells can be told apart — and a query carrying
//! input→output **examples** can be matched *by behaviour*. That is exactly where
//! paraphrase text retrieval is a coin-flip (roadmap item 3): ground the representation in
//! the cell's outcome, not its description.
//!
//! Deterministic, no external model: the probes and the Z80 VM are pure, so a fingerprint
//! is reproducible. Ported from the `soma-cell` experiment, adapted to cell80's [`Runner`].
use super::{Cartridge, Manifest, Runner, DEFAULT_CYCLES};

/// A default probe bank separating the common confusable families — order (`3,7` vs `7,3`),
/// equality (`5,5`), magnitude, and identity. Two-argument; a lower-arity cell simply
/// ignores the unused register, so every cell is probed uniformly.
///
/// `[1230, 0]` was added by the admission gate (roadmap 2.2): a multi-digit,
/// Luhn-checksum-valid value distinguishing `luhn_check` from `is_zero`, which the original
/// ten probes couldn't separate (none of them happened to be Luhn-valid, so `luhn_check`
/// degenerated to "is n exactly 0" on this bank alone) — a live example of the "widening the
/// probe bank is the honest fix" note in `admission.rs`.
///
/// `[65531, 3]` was added by the signed-deltas pack (library-growth.md "Next waves"): every
/// prior probe is non-negative when reinterpreted as `i16` (all ≤ 1230), so an `i16`-domain
/// cell's negative branch never fires on this bank alone — `sign_i16` degenerated to
/// `nonzero` (both only ever emitting `0`/`1` here). `65531` is `-5` as an `i16` bit
/// pattern, giving the first negative-domain probe.
pub const DEFAULT_PROBES: &[[u16; 2]] = &[
    [3, 7],
    [7, 3],
    [0, 0],
    [1, 1],
    [5, 5],
    [2, 9],
    [10, 3],
    [255, 1],
    [100, 4],
    [12, 12],
    [1230, 0],
    [65531, 3],
];

/// A cell's behavioural fingerprint: its primary result on each probe, or `None` if the run
/// did not return cleanly (a budget/halt outcome is itself a distinguishing signal).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fingerprint {
    /// One entry per probe, in probe order.
    pub outputs: Vec<Option<u16>>,
}

impl Fingerprint {
    /// Run `cart`'s entry on each probe and record `result`. Args beyond the cell's arity
    /// land in unused registers (harmless), so the same bank fingerprints any arity.
    pub fn compute(cart: &Cartridge, probes: &[[u16; 2]], budget: u64) -> Self {
        let mut runner = Runner::new(&cart.program);
        let entry = cart.manifest.entry.as_str();
        let outputs = probes
            .iter()
            .map(|p| match runner.run(Some(entry), p, budget) {
                Ok(r) if r.returned => Some(r.result),
                _ => None,
            })
            .collect();
        Fingerprint { outputs }
    }

    /// Fingerprint with [`DEFAULT_PROBES`] and the default cycle budget.
    pub fn of(cart: &Cartridge) -> Self {
        Self::compute(cart, DEFAULT_PROBES, DEFAULT_CYCLES)
    }

    /// Fraction of probes on which two fingerprints agree (matching `None`s count as
    /// agreement). `1.0` = behaviourally indistinguishable on this bank; `0.0` = always
    /// differ. Returns `1.0` for two empty fingerprints.
    pub fn agreement(&self, other: &Fingerprint) -> f32 {
        let n = self.outputs.len().min(other.outputs.len());
        if n == 0 {
            return 1.0;
        }
        let same = self
            .outputs
            .iter()
            .zip(&other.outputs)
            .filter(|(a, b)| a == b)
            .count();
        same as f32 / n as f32
    }
}

/// **Retrieval by I/O example** — rank cells by how well their *behaviour* reproduces the
/// given `(inputs, expected_output)` examples, the thing a text index cannot do. Each cell
/// is run on every example; its score is the number of examples it reproduces. Best first,
/// ties broken by id, only positive matches returned — so an empty result means *no cell in
/// the library reproduces these examples*. Args beyond a cell's arity are ignored, as in
/// [`Fingerprint::compute`].
pub fn rank_by_examples<'a>(
    carts: &'a [Cartridge],
    examples: &[(Vec<u16>, u16)],
    budget: u64,
) -> Vec<&'a Manifest> {
    rank_examples_iter(carts.iter(), examples, budget)
}

/// The engine behind [`rank_by_examples`], over any iterator of cartridges — so a warm
/// [`CellHost`](crate::CellHost) can route its catalog (a map, not a slice) by behaviour
/// without cloning. Deterministic: ties broken by id, so iteration order doesn't matter.
pub(crate) fn rank_examples_iter<'a>(
    carts: impl Iterator<Item = &'a Cartridge>,
    examples: &[(Vec<u16>, u16)],
    budget: u64,
) -> Vec<&'a Manifest> {
    let mut scored: Vec<(i32, &Manifest)> = carts
        .map(|c| {
            let mut runner = Runner::new(&c.program);
            let entry = c.manifest.entry.as_str();
            let hits = examples
                .iter()
                .filter(|(inp, want)| {
                    matches!(
                        runner.run(Some(entry), inp, budget),
                        Ok(r) if r.returned && r.result == *want
                    )
                })
                .count() as i32;
            (hits, &c.manifest)
        })
        .filter(|(s, _)| *s > 0)
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.id.cmp(&b.1.id)));
    scored.into_iter().map(|(_, m)| m).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CartridgeOpts, CellConfig};

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

    fn min_cell() -> Cartridge {
        cell(
            "min",
            "fn run(a: u16, b: u16) -> u16 { let mut m = a; if b < a { m = b; } m }",
        )
    }
    fn max_cell() -> Cartridge {
        cell(
            "max",
            "fn run(a: u16, b: u16) -> u16 { let mut m = a; if b > a { m = b; } m }",
        )
    }

    #[test]
    fn confusable_siblings_have_different_fingerprints() {
        // min and max share every manifest word ("the ? of two numbers") but their behaviour
        // separates them — the whole point of a verifier-grounded signal.
        let (fp_min, fp_max) = (Fingerprint::of(&min_cell()), Fingerprint::of(&max_cell()));
        assert!(
            fp_min.agreement(&fp_max) < 1.0,
            "min and max must be behaviourally distinguishable"
        );
        // On the ordered probe (3, 7): min → 3, max → 7.
        assert_eq!(fp_min.outputs[0], Some(3));
        assert_eq!(fp_max.outputs[0], Some(7));
    }

    #[test]
    fn fingerprint_is_deterministic() {
        let c = min_cell();
        assert_eq!(Fingerprint::of(&c), Fingerprint::of(&c));
    }

    #[test]
    fn retrieval_by_example_picks_the_behaviour_not_the_words() {
        let lib = vec![min_cell(), max_cell()];
        // Examples that only `max` reproduces: max(3,7)=7, max(10,3)=10.
        let want_max =
            rank_by_examples(&lib, &[(vec![3, 7], 7), (vec![10, 3], 10)], DEFAULT_CYCLES);
        assert_eq!(want_max[0].id, "max");
        // Flip the expected outputs and `min` wins — same words, different behaviour.
        let want_min = rank_by_examples(&lib, &[(vec![3, 7], 3), (vec![10, 3], 3)], DEFAULT_CYCLES);
        assert_eq!(want_min[0].id, "min");
    }
}
