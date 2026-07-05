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
/// equality (`5,5`), magnitude, and identity. Three-argument (the full calling
/// convention); a lower-arity cell simply ignores the unused registers, so every cell
/// is probed uniformly — and the third column is what stops `clamp`/`min3`/
/// `between_exclusive` collapsing to the same degenerate constant (the arity-3
/// admission exemption this bank retired).
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
/// The third column keeps `lo ≤ hi`-shaped rows meaningful for `clamp(x, lo, hi)`
/// and gives ordered/violated variants for `between`/`min3`/`median3`; the last
/// three rows exist purely for arity-3 discrimination (mid/lo/hi permutations).
pub const DEFAULT_PROBES: &[[u16; 3]] = &[
    [3, 7, 12],
    [7, 3, 1],
    [0, 0, 0],
    [1, 1, 1],
    [5, 5, 9],
    [2, 9, 5],
    [10, 3, 7],
    [255, 1, 128],
    [100, 4, 50],
    [12, 12, 12],
    [1230, 0, 2],
    [65531, 3, 6],
    [5, 2, 9],
    [9, 5, 2],
    [2, 8, 4],
    // Verifier-shape separators (the state-fingerprint gate found these blind
    // spots over the real library): `[4,2,4]` satisfies `a*b - c == d` but not
    // `a*b + c == d`; `[7,0,0]` satisfies `a+b+c == d` (and two-of-three-equal)
    // but not the fused forms; `[12,3,4]` satisfies exact `a/b == c` but not
    // `a^b == c`; `[9000,2500,40]` puts a *large* second value where every prior
    // row had ≤ 12 — basis-point cells rounded identically below 12 bps.
    [4, 2, 4],
    [7, 0, 0],
    [12, 3, 4],
    [9000, 2500, 40],
];

/// A cell's behavioural fingerprint: one entry per probe — for a value cell the
/// primary result; for a **state cell** the result folded with a position-sensitive
/// digest of every post-run scalar field, because state cells conventionally return
/// a status flag (`ok = 1`) and keep the *answer in output fields* (`add_checked_u32`
/// vs `abs_diff_u32` both return `1` on every probe; their `sum`/`diff` fields are
/// where the behaviour lives). `None` = the run did not return cleanly (a
/// budget/halt outcome is itself a distinguishing signal).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fingerprint {
    /// One entry per probe, in probe order.
    pub outputs: Vec<Option<u16>>,
}

/// Fold post-run scalar fields into the probe entry: position-sensitive (a rotate
/// per field index) so reordered values diverge, and covering the full width of
/// wide fields. Deterministic; identical layouts + identical behaviour ⇒ identical
/// digests.
fn digest_state(result: u16, fields: &[(String, u64)]) -> u16 {
    let mut d = result;
    for (i, (_, v)) in fields.iter().enumerate() {
        let r = (i as u32 * 5 + 3) % 16;
        d ^= (*v as u16).rotate_left(r);
        d ^= ((*v >> 16) as u16).rotate_left((r + 7) % 16);
        d = d.rotate_left(1);
    }
    d
}

/// How many convention registers a *tuple* return spreads across (`HL`, `DE`, `BC`): its
/// element count, capped at the 3-register convention. A scalar return — including `u32`,
/// whose `HL:DE` pair is one value — is `1`, so its fingerprint stays the primary register
/// alone (existing single-value fingerprints are unchanged). `u32` returns keeping only
/// their low word is a separate, out-of-scope gap.
fn ret_reg_count(ret: &str) -> usize {
    match ret
        .trim()
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
    {
        Some(inner) if !inner.trim().is_empty() => inner
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .count()
            .min(3),
        _ => 1,
    }
}

/// Fold the *secondary* return registers (`DE`, `BC`) into the primary result for a
/// tuple-returning free function, position-sensitively (a rotate per register index). A
/// scalar return declares `n_regs == 1`, so the loop never runs and the digest is exactly
/// `regs[0]` — every existing single-value fingerprint is byte-identical. Without this a
/// `sort3` returning `(min, mid, max)` fingerprints as `min3` (only `HL` = `min` digested),
/// a false duplicate — the real payload (`mid`/`max`) lives in the registers this folds in.
fn digest_regs(regs: &[u16; 3], n_regs: usize) -> u16 {
    let mut d = regs[0];
    for (i, v) in regs.iter().take(n_regs).enumerate().skip(1) {
        let r = (i as u32 * 5 + 3) % 16;
        d ^= v.rotate_left(r);
        d = d.rotate_left(1);
    }
    d
}

impl Fingerprint {
    /// Run `cart`'s entry on each probe and record `result`. Value cells take the
    /// probe triple in the convention registers (args beyond the cell's arity land
    /// in unused registers, harmless); **state cells** take it through their named
    /// scalar fields, assigned cyclically in declaration order (`field i ←
    /// probe[i % 3]`) — deterministic per layout, so identical-layout duplicates
    /// (the real dupe risk) fingerprint identically.
    pub fn compute(cart: &Cartridge, probes: &[[u16; 3]], budget: u64) -> Self {
        let mut runner = Runner::new(&cart.program);
        let entry = cart.manifest.entry.as_str();
        // A tuple-returning free function spreads its payload across HL/DE/BC — digest all
        // the declared registers, not just HL (a scalar declares 1, so it is unchanged).
        let n_ret = ret_reg_count(&cart.manifest.signature.ret);
        let state: Vec<(u16, crate::Ty, usize)> = cart
            .manifest
            .state_addrs
            .iter()
            .filter(|(_, _, ty)| ty.capacity().is_none()) // scalars only
            .enumerate()
            .map(|(i, (_, addr, ty))| (*addr, *ty, i))
            .collect();
        let outputs = probes
            .iter()
            .map(|p| {
                let run = if state.is_empty() {
                    runner.run(Some(entry), p, budget)
                } else {
                    let inputs: Vec<(u16, crate::Ty, u64)> = state
                        .iter()
                        .map(|(addr, ty, i)| (*addr, *ty, p[i % 3] as u64))
                        .collect();
                    runner.run_with_inputs(Some(entry), &[crate::STATE_BASE], &inputs, budget)
                };
                match run {
                    Ok(r) if r.returned && state.is_empty() => Some(digest_regs(&r.regs, n_ret)),
                    Ok(r) if r.returned => {
                        // The behaviour of a state cell lives in its output fields.
                        let reads: Vec<(String, u16, crate::Ty)> = cart
                            .manifest
                            .state_addrs
                            .iter()
                            .filter(|(_, _, ty)| ty.capacity().is_none())
                            .cloned()
                            .collect();
                        Some(digest_state(r.result, &runner.read_named(&reads)))
                    }
                    _ => None,
                }
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

/// [`rank_by_examples`] for **state cells**: each example is named fields in →
/// expected `result` out. The structured sibling the campaign's plan cells and
/// every `Struct::run` cell need — two-register probes can't drive them.
pub(crate) fn rank_field_examples_iter<'a>(
    carts: impl Iterator<Item = &'a Cartridge>,
    examples: &[(Vec<(String, u64)>, u16)],
    budget: u64,
) -> Vec<&'a Manifest> {
    let mut scored: Vec<(i32, &Manifest)> = carts
        .filter(|c| !c.manifest.state_addrs.is_empty())
        .map(|c| {
            let mut runner = Runner::new(&c.program);
            let entry = c.manifest.entry.as_str();
            let addrs = &c.manifest.state_addrs;
            let hits = examples
                .iter()
                .filter(|(fields, want)| {
                    // Every named field must exist on this cell (a miss = no match,
                    // not an error — ranking is a sieve).
                    let inputs: Option<Vec<(u16, crate::Ty, u64)>> = fields
                        .iter()
                        .map(|(name, val)| {
                            addrs
                                .iter()
                                .find(|(n, _, ty)| n == name && ty.capacity().is_none())
                                .map(|(_, addr, ty)| (*addr, *ty, *val))
                        })
                        .collect();
                    let Some(inputs) = inputs else { return false };
                    matches!(
                        runner.run_with_inputs(Some(entry), &[crate::STATE_BASE], &inputs, budget),
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

    fn state_cell(id: &str, entry: &str, src: &str) -> Cartridge {
        Cartridge::compile(
            src,
            CellConfig::sandboxed(),
            CartridgeOpts {
                id: Some(id.into()),
                entry: Some(entry.into()),
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
    fn ret_reg_count_and_scalar_identity() {
        assert_eq!(ret_reg_count("u16"), 1);
        assert_eq!(ret_reg_count("u32"), 1); // one value (low word only — documented gap)
        assert_eq!(ret_reg_count("()"), 1);
        assert_eq!(ret_reg_count("(u16, u16)"), 2);
        assert_eq!(ret_reg_count("(u16, u16, u16)"), 3);
        // n_regs == 1 is the identity: the digest is exactly the primary register, so no
        // existing single-value fingerprint moves.
        assert_eq!(digest_regs(&[42, 7, 99], 1), 42);
        // Folding a secondary register changes the digest (order-sensitively).
        assert_ne!(digest_regs(&[42, 7, 0], 2), digest_regs(&[42, 0, 0], 2));
    }

    #[test]
    fn tuple_return_folds_secondary_registers() {
        // A tuple-returning free function whose FIRST value equals a scalar cell's, but
        // whose real payload lives in DE/BC. Before digesting the secondaries this
        // fingerprinted as the scalar — the false-duplicate `sort3`-vs-`min3` case.
        let src_min = "fn run(a: u16, b: u16, c: u16) -> u16 \
                       { let mut m = a; if b < m { m = b; } if c < m { m = c; } m }";
        let src_triple = "fn run(a: u16, b: u16, c: u16) -> (u16, u16, u16) \
                          { let mut m = a; if b < m { m = b; } if c < m { m = c; } (m, b, c) }";
        let min3 = cell("min3", src_min);
        let triple = cell("min_bc", src_triple);
        let (fp_min3, fp_triple) = (Fingerprint::of(&min3), Fingerprint::of(&triple));
        assert!(
            fp_min3.agreement(&fp_triple) < 1.0,
            "a tuple return must fingerprint apart from a scalar sharing its first value"
        );
        // The digest stays deterministic — an identical tuple cell agrees fully.
        let triple2 = cell("min_bc2", src_triple);
        assert_eq!(fp_triple.agreement(&Fingerprint::of(&triple2)), 1.0);
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

    #[test]
    fn arity3_cells_are_distinguished_by_the_third_column() {
        // The retired exemption's proof: clamp and min3 used to collapse to the
        // same degenerate constant when the third register defaulted to 0.
        let clamp = cell(
            "clamp",
            "fn run(x: u16, lo: u16, hi: u16) -> u16 { let mut r = x; if x < lo { r = lo; } if x > hi { r = hi; } r }",
        );
        let min3 = cell(
            "min3",
            "fn run(a: u16, b: u16, c: u16) -> u16 { let mut m = a; if b < m { m = b; } if c < m { m = c; } m }",
        );
        let (fc, fm) = (Fingerprint::of(&clamp), Fingerprint::of(&min3));
        assert!(fc.agreement(&fm) < 1.0, "third column must separate them");
    }

    #[test]
    fn state_cells_fingerprint_via_fields() {
        // State cells are driven through named fields; different behaviours
        // (manhattan vs chebyshev) separate, identical layouts agree exactly.
        let man = state_cell(
            "manhattan",
            "Pts::run",
            "struct Pts { x1: u16, y1: u16, x2: u16, y2: u16, dist: u16 }\nimpl Pts { fn run(&mut self) -> u16 { let dx = if self.x1 > self.x2 { self.x1 - self.x2 } else { self.x2 - self.x1 }; let dy = if self.y1 > self.y2 { self.y1 - self.y2 } else { self.y2 - self.y1 }; self.dist = dx + dy; self.dist } }",
        );
        let cheb = state_cell(
            "chebyshev",
            "Cheb::run",
            "struct Cheb { x1: u16, y1: u16, x2: u16, y2: u16, dist: u16 }\nimpl Cheb { fn run(&mut self) -> u16 { let dx = if self.x1 > self.x2 { self.x1 - self.x2 } else { self.x2 - self.x1 }; let dy = if self.y1 > self.y2 { self.y1 - self.y2 } else { self.y2 - self.y1 }; let d = if dx > dy { dx } else { dy }; self.dist = d; self.dist } }",
        );
        let (fm, fc) = (Fingerprint::of(&man), Fingerprint::of(&cheb));
        assert!(
            !fm.outputs.iter().all(Option::is_none),
            "state cells must run"
        );
        assert!(
            fm.agreement(&fc) < 1.0,
            "different distance metrics separate"
        );
        assert_eq!(fm, Fingerprint::of(&man), "deterministic");
    }

    #[test]
    fn field_examples_route_state_cells() {
        let man = state_cell(
            "manhattan",
            "Pts::run",
            "struct Pts { x1: u16, y1: u16, x2: u16, y2: u16, dist: u16 }\nimpl Pts { fn run(&mut self) -> u16 { let dx = if self.x1 > self.x2 { self.x1 - self.x2 } else { self.x2 - self.x1 }; let dy = if self.y1 > self.y2 { self.y1 - self.y2 } else { self.y2 - self.y1 }; self.dist = dx + dy; self.dist } }",
        );
        let cheb = state_cell(
            "chebyshev",
            "Cheb::run",
            "struct Cheb { x1: u16, y1: u16, x2: u16, y2: u16, dist: u16 }\nimpl Cheb { fn run(&mut self) -> u16 { let dx = if self.x1 > self.x2 { self.x1 - self.x2 } else { self.x2 - self.x1 }; let dy = if self.y1 > self.y2 { self.y1 - self.y2 } else { self.y2 - self.y1 }; let d = if dx > dy { dx } else { dy }; self.dist = d; self.dist } }",
        );
        let lib = [man, cheb];
        // |3-10| + |4-8| = 11 (manhattan); max(7,4) = 7 (chebyshev).
        let ex = |out: u16| {
            vec![(
                vec![
                    ("x1".to_string(), 3u64),
                    ("y1".to_string(), 4u64),
                    ("x2".to_string(), 10u64),
                    ("y2".to_string(), 8u64),
                ],
                out,
            )]
        };
        let hits = rank_field_examples_iter(lib.iter(), &ex(11), DEFAULT_CYCLES);
        assert_eq!(hits[0].id, "manhattan");
        let hits = rank_field_examples_iter(lib.iter(), &ex(7), DEFAULT_CYCLES);
        assert_eq!(hits[0].id, "chebyshev");
        // An unknown field name matches nothing (a sieve, not an error).
        let hits = rank_field_examples_iter(
            lib.iter(),
            &[(vec![("bogus".to_string(), 1u64)], 1)],
            DEFAULT_CYCLES,
        );
        assert!(hits.is_empty());
    }
}
