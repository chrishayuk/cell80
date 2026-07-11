//! Host-oracle tests for the verifier-ranker pack (`cell80/cells/verifier-ranker/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::{cell_src, run_cell};
use cell80::{StateCell, DEFAULT_CYCLES};

#[test]
fn verifier_ranker_cells_match_defined_behaviour() {
    fn verify(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, cell80::Halt) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report.result, report.halt)
    }

    // The GSM8K math-campaign verifier/ranker pack (Phase 2.3, M1 pack 4/5) — each cell
    // re-derives one side of a candidate plan's claimed equation and returns a plain 0/1
    // verdict, never escalating (a verifier always answers; escalation is for the
    // arithmetic packs that *compute* a value). answer_eq is an alias of the predicates
    // pack's `eq`; multi-plan agreement/tie-break are already covered by
    // `majority3`/`mode3` (ranking-stats) — neither needed new code.
    assert_eq!(run_cell("sum_equals", &[3, 4, 7]), 1);
    assert_eq!(run_cell("sum_equals", &[3, 4, 8]), 0);
    // 40000 + 30000 wraps to 4464 in u16; sum_equals must not false-positive on that.
    assert_eq!(run_cell("sum_equals", &[40000, 30000, 4464]), 0);

    assert_eq!(run_cell("diff_equals", &[10, 3, 7]), 1);
    assert_eq!(run_cell("diff_equals", &[10, 3, 6]), 0);
    assert_eq!(run_cell("diff_equals", &[3, 10, 0]), 0); // a < b → 0, not a wrapped u16

    let (ok, halt) = verify(
        "product_equals_u32",
        "ProductEquals",
        &[("a", 12), ("b", 5), ("total", 60)],
    );
    assert_eq!((ok, halt), (1, cell80::Halt::Returned));
    let (ok, _) = verify(
        "product_equals_u32",
        "ProductEquals",
        &[("a", 12), ("b", 5), ("total", 61)],
    );
    assert_eq!(ok, 0);
    // a genuine u32*u32 overflow is a false claim, not an escalation — a verifier always
    // returns a verdict.
    let (ok, halt) = verify(
        "product_equals_u32",
        "ProductEquals",
        &[("a", 4_294_967_295), ("b", 2), ("total", 0)],
    );
    assert_eq!((ok, halt), (0, cell80::Halt::Returned));

    let (ok, _) = verify(
        "quotient_equals_exact_u32",
        "QuotientEqualsExact",
        &[("a", 48), ("b", 12), ("quotient", 4)],
    );
    assert_eq!(ok, 1);
    let (ok, _) = verify(
        "quotient_equals_exact_u32",
        "QuotientEqualsExact",
        &[("a", 50), ("b", 12), ("quotient", 4)],
    );
    assert_eq!(ok, 0); // remainder 2 — inexact
    let (ok, halt) = verify(
        "quotient_equals_exact_u32",
        "QuotientEqualsExact",
        &[("a", 48), ("b", 0), ("quotient", 4)],
    );
    assert_eq!((ok, halt), (0, cell80::Halt::Returned)); // divide-by-zero is a false verdict too
}

#[test]
fn verifier_ranker_wave2_cells_match_defined_behaviour() {
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report.result, cell)
    }

    // GSM8K math-campaign verifier/ranker pack, second slice — wide (u32) siblings of the
    // first slice's u16-scoped verifiers (money/count totals in this campaign routinely
    // exceed 65535), reverse-equation counterparts for every checked-arithmetic wave-2
    // shape (mul/mul3/mul_add/mul_sub/pow), and a constraint check for the sign-magnitude
    // kernels. answer_in_options / a general options-membership verifier remains
    // deliberately not built (docs/library-growth.md already weighed and deferred it: GSM8K
    // is free-response, thin motivation).

    assert_eq!(
        step(
            "answer_eq_u32",
            "AnswerEqWide",
            &[("a", 100_000), ("b", 100_000)]
        )
        .0,
        1
    );
    assert_eq!(
        step("answer_eq_u32", "AnswerEqWide", &[("a", 100_000), ("b", 1)]).0,
        0
    );

    assert_eq!(
        step(
            "sum_equals_u32",
            "SumEqualsWide",
            &[("a", 100_000), ("b", 50_000), ("total", 150_000)]
        )
        .0,
        1
    );
    assert_eq!(
        step(
            "sum_equals_u32",
            "SumEqualsWide",
            &[("a", 100_000), ("b", 50_000), ("total", 1)]
        )
        .0,
        0
    );

    assert_eq!(
        step(
            "diff_equals_u32",
            "DiffEqualsWide",
            &[("a", 100_000), ("b", 30_000), ("remainder", 70_000)]
        )
        .0,
        1
    );
    assert_eq!(
        step(
            "diff_equals_u32",
            "DiffEqualsWide",
            &[("a", 30_000), ("b", 100_000), ("remainder", 0)]
        )
        .0,
        0
    ); // a < b: never a match, unsigned

    assert_eq!(
        step(
            "sum3_equals_u32",
            "Sum3EqualsWide",
            &[("a", 1), ("b", 2), ("c", 3), ("total", 6)]
        )
        .0,
        1
    );
    assert_eq!(
        step(
            "sum3_equals_u32",
            "Sum3EqualsWide",
            &[("a", u32::MAX as u64), ("b", 1), ("c", 0), ("total", 0)]
        )
        .0,
        0
    ); // overflow: never a false-positive match

    assert_eq!(
        step(
            "product3_equals_u32",
            "Product3EqualsWide",
            &[("a", 2), ("b", 3), ("c", 4), ("total", 24)]
        )
        .0,
        1
    );
    assert_eq!(
        step(
            "product3_equals_u32",
            "Product3EqualsWide",
            &[("a", 100_000), ("b", 100_000), ("c", 1), ("total", 0)]
        )
        .0,
        0
    );

    assert_eq!(
        step(
            "mul_add_equals_u32",
            "MulAddEqualsWide",
            &[("a", 7), ("b", 6), ("c", 3), ("total", 45)]
        )
        .0,
        1
    );
    assert_eq!(
        step(
            "mul_add_equals_u32",
            "MulAddEqualsWide",
            &[("a", 7), ("b", 6), ("c", 3), ("total", 46)]
        )
        .0,
        0
    );

    assert_eq!(
        step(
            "mul_sub_equals_u32",
            "MulSubEqualsWide",
            &[("a", 10), ("b", 5), ("c", 20), ("total", 30)]
        )
        .0,
        1
    );
    assert_eq!(
        step(
            "mul_sub_equals_u32",
            "MulSubEqualsWide",
            &[("a", 3), ("b", 4), ("c", 100), ("total", 0)]
        )
        .0,
        0
    ); // c > product: never a false-positive match

    assert_eq!(
        step(
            "pow_equals_u32",
            "PowEqualsWide",
            &[("base", 2), ("exp", 10), ("total", 1024)]
        )
        .0,
        1
    );
    assert_eq!(
        step(
            "pow_equals_u32",
            "PowEqualsWide",
            &[("base", 2), ("exp", 32), ("total", 0)]
        )
        .0,
        0
    ); // overflow: never a false-positive match

    assert_eq!(
        step("smag_is_nonneg", "SmagIsNonneg", &[("mag", 5), ("neg", 0)]).0,
        1
    );
    assert_eq!(
        step("smag_is_nonneg", "SmagIsNonneg", &[("mag", 5), ("neg", 1)]).0,
        0
    );
    assert_eq!(
        step("smag_is_nonneg", "SmagIsNonneg", &[("mag", 0), ("neg", 1)]).0,
        1
    ); // negative zero is nonnegative

    assert_eq!(
        step(
            "agree3_u32",
            "Agree3Wide",
            &[("a", 100_000), ("b", 100_000), ("c", 1)]
        )
        .0,
        1
    );
    assert_eq!(
        step(
            "agree3_u32",
            "Agree3Wide",
            &[("a", 100_000), ("b", 1), ("c", 2)]
        )
        .0,
        0
    );

    assert_eq!(
        step(
            "answer_within_tolerance_u32",
            "AnswerWithinToleranceWide",
            &[
                ("candidate", 100_005),
                ("actual", 100_000),
                ("tolerance", 10)
            ]
        )
        .0,
        1
    );
    assert_eq!(
        step(
            "answer_within_tolerance_u32",
            "AnswerWithinToleranceWide",
            &[
                ("candidate", 100_050),
                ("actual", 100_000),
                ("tolerance", 10)
            ]
        )
        .0,
        0
    );
}

#[test]
fn math_wave3_verifier_ranker_slice() {
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        let result = report.result;
        (result, report, cell)
    }

    // smag_eq: the smag family's missing verifier — equal magnitude+sign match; a
    // negative-zero canonicalizes to nonnegative so it still equals a plain zero.
    assert_eq!(
        step(
            "smag_eq",
            "SmagEq",
            &[("mag_a", 5), ("neg_a", 1), ("mag_b", 5), ("neg_b", 1)]
        )
        .0,
        1
    );
    assert_eq!(
        step(
            "smag_eq",
            "SmagEq",
            &[("mag_a", 5), ("neg_a", 1), ("mag_b", 5), ("neg_b", 0)]
        )
        .0,
        0
    );
    assert_eq!(
        step(
            "smag_eq",
            "SmagEq",
            &[("mag_a", 0), ("neg_a", 1), ("mag_b", 0), ("neg_b", 0)]
        )
        .0,
        1
    ); // negative zero == positive zero
}

#[test]
fn wave4_verifier_ranker_gap_fill_cells_match_defined_behaviour() {
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> u16 {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // percent_equals_bps: 5% of 1000 is 50, so 1000 -> 1050 is a true 500-bps increase.
    assert_eq!(
        step(
            "percent_equals_bps",
            "PercentEqualsBps",
            &[("before", 1000), ("after", 1050), ("bps", 500)]
        ),
        1
    );
    assert_eq!(
        step(
            "percent_equals_bps",
            "PercentEqualsBps",
            &[("before", 1000), ("after", 1051), ("bps", 500)]
        ),
        0
    );
    assert_eq!(
        step(
            "percent_equals_bps",
            "PercentEqualsBps",
            &[("before", 4_294_967_295), ("after", 0), ("bps", 2)]
        ),
        0 // multiply overflow -> claim doesn't hold, never escalates
    );

    // parts_sum_to_total4_u32: exact match, mismatch, and a wrapping add (claim doesn't
    // hold, never escalates).
    assert_eq!(
        step(
            "parts_sum_to_total4_u32",
            "PartsSumToTotal4Wide",
            &[("a", 10), ("b", 20), ("c", 30), ("d", 40), ("total", 100)]
        ),
        1
    );
    assert_eq!(
        step(
            "parts_sum_to_total4_u32",
            "PartsSumToTotal4Wide",
            &[("a", 10), ("b", 20), ("c", 30), ("d", 40), ("total", 99)]
        ),
        0
    );
    assert_eq!(
        step(
            "parts_sum_to_total4_u32",
            "PartsSumToTotal4Wide",
            &[
                ("a", 4_294_967_295),
                ("b", 1),
                ("c", 0),
                ("d", 0),
                ("total", 0)
            ]
        ),
        0
    );

    // nonnegative_after_delta: mirrors apply_delta_clamped's own sign-handling idiom.
    assert_eq!(run_cell("nonnegative_after_delta", &[10, 65531]), 1); // delta -5
    assert_eq!(run_cell("nonnegative_after_delta", &[3, 65531]), 0); // delta -5
    assert_eq!(run_cell("nonnegative_after_delta", &[0, 0]), 1);
    assert_eq!(
        run_cell("nonnegative_after_delta", &[100, 65436]),
        1 // delta -100, exactly zero still counts as nonnegative
    );
    // Wave 4, slice 4: verifier-ranker gap-fill — the three genuinely-motivated survivors
    // of the original ~100-cell proposal's category G (7 of the other 10 proposed cells
    // were exact duplicates of already-shipped verifier-ranker cells, per the wave-4 pack
    // note in docs/library-growth.md).
}

#[test]
fn linear_eq_holds_matches_defined_behaviour() {
    // Mined from chuk-math-gym's linear_equations domain: verify a candidate x against
    // ax + b == cx + d in one call, exact (no float tolerance) — the fused sibling of
    // linear_solve_1var's solve step so a solved x round-trips with zero error.
    fn holds(a: i16, b: i16, c: i16, d: i16, x: i16) -> u64 {
        fn bits(v: i16) -> u64 {
            (v as u16) as u64
        }
        let mut cell =
            StateCell::bind(&cell_src("linear_eq_holds"), "LinearEqHolds", None).unwrap();
        cell.set("a", bits(a)).unwrap();
        cell.set("b", bits(b)).unwrap();
        cell.set("c", bits(c)).unwrap();
        cell.set("d", bits(d)).unwrap();
        cell.set("x", bits(x)).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap();
        cell.get("ok").unwrap()
    }

    assert_eq!(holds(2, 3, 5, -6, 3), 1); // the true solution
    assert_eq!(holds(2, 3, 5, -6, 4), 0); // a wrong candidate
    assert_eq!(holds(1, 0, 2, 3, -3), 1); // a negative solution
    assert_eq!(holds(1, 0, 2, 3, -2), 0);
    assert_eq!(holds(0, 5, 0, 5, 100), 1); // degenerate identity: any x holds
}

#[test]
fn gcd_equals_u32_matches_hand_computed_expectations() {
    fn verify(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, cell80::Halt) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report.result, report.halt)
    }

    // gcd(48, 18): 48 % 18 = 12; 18 % 12 = 6; 12 % 6 = 0 -> gcd = 6. Claim matches.
    let (ok, halt) = verify(
        "gcd_equals_u32",
        "GcdEqualsWide",
        &[("a", 48), ("b", 18), ("g", 6)],
    );
    assert_eq!((ok, halt), (1, cell80::Halt::Returned));

    // Same a, b but a false claim (5 instead of the true gcd 6) -> 0.
    let (ok, _) = verify(
        "gcd_equals_u32",
        "GcdEqualsWide",
        &[("a", 48), ("b", 18), ("g", 5)],
    );
    assert_eq!(ok, 0);

    // gcd(0, 0): loop never runs (y starts at 0), x stays 0 -> gcd = 0. Claim matches.
    let (ok, _) = verify(
        "gcd_equals_u32",
        "GcdEqualsWide",
        &[("a", 0), ("b", 0), ("g", 0)],
    );
    assert_eq!(ok, 1);

    // gcd(0, 7): one iteration folds x=0,y=7 into x=7,y=0 -> gcd = 7. Claim matches.
    let (ok, _) = verify(
        "gcd_equals_u32",
        "GcdEqualsWide",
        &[("a", 0), ("b", 7), ("g", 7)],
    );
    assert_eq!(ok, 1);

    // Two consecutive u32 integers are always coprime: gcd(u32::MAX, u32::MAX - 1) = 1.
    // A false claim of 2 on the same inputs must verify to 0, not silently pass.
    let (ok, _) = verify(
        "gcd_equals_u32",
        "GcdEqualsWide",
        &[("a", 4_294_967_295), ("b", 4_294_967_294), ("g", 1)],
    );
    assert_eq!(ok, 1);
    let (ok, _) = verify(
        "gcd_equals_u32",
        "GcdEqualsWide",
        &[("a", 4_294_967_295), ("b", 4_294_967_294), ("g", 2)],
    );
    assert_eq!(ok, 0);
}

// Verifies lcm_equals_u32: LcmEqualsWide { a, b, l } -> u16, 1 if l is the true wide
// LCM of a and b, else 0. Covers the normal correct/incorrect cases, the a==0/b==0
// zero-convention edge cases (matching lcm_u32's own definition that lcm(0, x) = 0),
// and a genuine u32 overflow in (a/g)*b, which must read as "claim false" (0), not
// an escalation — a verifier always returns a verdict, never halts.
#[test]
fn lcm_equals_u32_hand_computed() {
    fn run(fields: &[(&str, u64)]) -> u16 {
        let mut cell = StateCell::bind(&cell_src("lcm_equals_u32"), "LcmEqualsWide", None)
            .unwrap_or_else(|e| panic!("bind lcm_equals_u32: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // lcm(4, 6) = 12, claim matches -> 1
    assert_eq!(run(&[("a", 4), ("b", 6), ("l", 12)]), 1);
    // lcm(4, 6) = 12, claim is wrong (24) -> 0
    assert_eq!(run(&[("a", 4), ("b", 6), ("l", 24)]), 0);
    // a == 0 -> lcm defined as 0 by lcm_u32's convention, claim matches -> 1
    assert_eq!(run(&[("a", 0), ("b", 5), ("l", 0)]), 1);
    // a == 0 -> true lcm is 0, but claim says 5 -> 0
    assert_eq!(run(&[("a", 0), ("b", 5), ("l", 5)]), 0);
    // both zero -> true lcm is 0, claim matches -> 1
    assert_eq!(run(&[("a", 0), ("b", 0), ("l", 0)]), 1);
    // gcd(3_000_000_000, 3_000_000_006) = 6, so a/g * b = 500_000_000 * 3_000_000_006
    // overflows u32 (true value ~1.5e18); the overflow-detection idiom fires, so the
    // verifier must return 0 even against the wrapped product value itself.
    assert_eq!(
        run(&[
            ("a", 3_000_000_000),
            ("b", 3_000_000_006),
            ("l", 770_072_064), // (500_000_000 * 3_000_000_006) mod 2^32, hand-computed
        ]),
        0
    );
}

#[test]
fn quotient_equals_ceil_u32_matches_hand_computed_cases() {
    // Checks quotient_equals_ceil_u32: the verifier counterpart of div_ceil_u32 — recomputes
    // ceiling division (q = a/b, bump by 1 if there's a remainder) and compares against a
    // claimed quotient, distinguishing it from both quotient_equals_floor_u32 and
    // quotient_equals_exact_u32 on inexact divisions.
    fn verify(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, cell80::Halt) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report.result, report.halt)
    }

    // 48 / 12 = 4 exactly (r=0), ceil quotient is 4, claimed 4 -> matches -> 1
    let (ok, _) = verify(
        "quotient_equals_ceil_u32",
        "QuotientEqualsCeil",
        &[("a", 48), ("b", 12), ("quotient", 4)],
    );
    assert_eq!(ok, 1);

    // 50 / 12 = 4 remainder 2 (inexact), ceil rounds up to 5, claimed 5 -> matches -> 1
    // (this is the case where quotient_equals_floor_u32 and quotient_equals_exact_u32
    // would both say 0 — ceil is genuinely distinct here)
    let (ok, _) = verify(
        "quotient_equals_ceil_u32",
        "QuotientEqualsCeil",
        &[("a", 50), ("b", 12), ("quotient", 5)],
    );
    assert_eq!(ok, 1);

    // 50 / 12 ceil is 5, claimed 4 (the floor value) -> mismatch -> 0
    let (ok, _) = verify(
        "quotient_equals_ceil_u32",
        "QuotientEqualsCeil",
        &[("a", 50), ("b", 12), ("quotient", 4)],
    );
    assert_eq!(ok, 0);

    // b == 0 -> always 0 regardless of claimed quotient
    let (ok, halt) = verify(
        "quotient_equals_ceil_u32",
        "QuotientEqualsCeil",
        &[("a", 48), ("b", 0), ("quotient", 4)],
    );
    assert_eq!((ok, halt), (0, cell80::Halt::Returned));

    // 0 / 5 = 0 exactly, ceil quotient is 0, claimed 0 -> matches -> 1
    let (ok, _) = verify(
        "quotient_equals_ceil_u32",
        "QuotientEqualsCeil",
        &[("a", 0), ("b", 5), ("quotient", 0)],
    );
    assert_eq!(ok, 1);

    // 13 / 12 = 1 remainder 1, ceil rounds up to 2; claimed 1 (off by the round-up) -> mismatch -> 0
    let (ok, _) = verify(
        "quotient_equals_ceil_u32",
        "QuotientEqualsCeil",
        &[("a", 13), ("b", 12), ("quotient", 1)],
    );
    assert_eq!(ok, 0);
}

#[test]
fn remainder_equals_u32_matches_defined_behaviour() {
    fn verify(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, cell80::Halt) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report.result, report.halt)
    }

    // remainder_equals_u32 — verifier counterpart of mod_u32 at wide (u32) width: returns 1 if
    // b != 0 and a % b == rem, else 0 (including b == 0, checked without a divide-by-zero halt).
    let (ok, halt) = verify(
        "remainder_equals_u32",
        "RemainderEqualsU32",
        &[("a", 50), ("b", 12), ("rem", 2)],
    );
    assert_eq!((ok, halt), (1, cell80::Halt::Returned));
    let (ok, _) = verify(
        "remainder_equals_u32",
        "RemainderEqualsU32",
        &[("a", 50), ("b", 12), ("rem", 0)],
    );
    assert_eq!(ok, 0); // true remainder is 2, not 0
    let (ok, halt) = verify(
        "remainder_equals_u32",
        "RemainderEqualsU32",
        &[("a", 100), ("b", 0), ("rem", 100)],
    );
    assert_eq!((ok, halt), (0, cell80::Halt::Returned)); // b == 0 is a false verdict too, never a halt
    let (ok, _) = verify(
        "remainder_equals_u32",
        "RemainderEqualsU32",
        &[("a", 4_294_967_295), ("b", 10), ("rem", 5)],
    );
    assert_eq!(ok, 1); // u32::MAX % 10 == 5 at wide width
}

#[test]
fn smag_add_equals_recomputes_the_combine_and_canonicalizes_zero() {
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, cell80::Halt) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report.result, report.halt)
    }

    // smag_add_equals: the sign-magnitude family's missing arithmetic-combine verifier --
    // recomputes smag_add's same-sign-add / opposite-sign-subtract rule and checks it
    // against a claimed (mag_c, neg_c), canonicalizing zero-magnitude to nonnegative on
    // both sides, never escalating (a genuine overflow is a false claim, not a halt).

    // Same-sign add, no overflow: 5 + 3 = 8 (+). Claim matches.
    assert_eq!(
        step(
            "smag_add_equals",
            "SmagAddEquals",
            &[
                ("mag_a", 5),
                ("neg_a", 0),
                ("mag_b", 3),
                ("neg_b", 0),
                ("mag_c", 8),
                ("neg_c", 0)
            ]
        )
        .0,
        1
    );
    // Same-sign add (both negative): -5 + -3 = -8, but claim says (8, +) -- sign mismatch.
    assert_eq!(
        step(
            "smag_add_equals",
            "SmagAddEquals",
            &[
                ("mag_a", 5),
                ("neg_a", 1),
                ("mag_b", 3),
                ("neg_b", 1),
                ("mag_c", 8),
                ("neg_c", 0)
            ]
        )
        .0,
        0
    );
    // Opposite sign, |a| >= |b|: 10 + -4 = 6 (+), sign follows the larger-magnitude operand.
    assert_eq!(
        step(
            "smag_add_equals",
            "SmagAddEquals",
            &[
                ("mag_a", 10),
                ("neg_a", 0),
                ("mag_b", 4),
                ("neg_b", 1),
                ("mag_c", 6),
                ("neg_c", 0)
            ]
        )
        .0,
        1
    );
    // Opposite sign, |b| > |a|: 4 + -10 = -6, sign follows the larger-magnitude operand.
    assert_eq!(
        step(
            "smag_add_equals",
            "SmagAddEquals",
            &[
                ("mag_a", 4),
                ("neg_a", 0),
                ("mag_b", 10),
                ("neg_b", 1),
                ("mag_c", 6),
                ("neg_c", 1)
            ]
        )
        .0,
        1
    );
    // Opposite sign, exact cancellation: 7 + -7 = 0. Claimed side is a "negative zero"
    // (0, -), which must canonicalize to nonnegative too so the two zeros still match.
    assert_eq!(
        step(
            "smag_add_equals",
            "SmagAddEquals",
            &[
                ("mag_a", 7),
                ("neg_a", 0),
                ("mag_b", 7),
                ("neg_b", 1),
                ("mag_c", 0),
                ("neg_c", 1)
            ]
        )
        .0,
        1
    );
    // Same-sign add overflows u32 (u32::MAX + 1): a real overflow is a false claim, not a
    // halt -- caught by the wrapping-add + overflow-detect idiom, never add_checked_u32.
    let (ok, halt) = step(
        "smag_add_equals",
        "SmagAddEquals",
        &[
            ("mag_a", 4_294_967_295),
            ("neg_a", 0),
            ("mag_b", 1),
            ("neg_b", 0),
            ("mag_c", 0),
            ("neg_c", 0),
        ],
    );
    assert_eq!((ok, halt), (0, cell80::Halt::Returned));
}

#[test]
fn smag_mul_equals_matches_hand_computed_expectations() {
    // Reverse-equation counterpart of smag_mul: recompute mag_a*mag_b via the same
    // wrapping-multiply-and-divide-back overflow-detect idiom product_equals_u32 uses,
    // derive the expected sign (same-positive/different-negative, zero-magnitude
    // canonicalizing to nonnegative per smag_mul's own rule), and compare both against
    // the claimed (mag_c, neg_c). Never escalates -- overflow just means "no match".
    fn verify(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, cell80::Halt) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report.result, report.halt)
    }

    // 5 * 3 = 15, both nonnegative -> claimed (15, 0) matches -> 1.
    let (ok, halt) = verify(
        "smag_mul_equals",
        "SmagMulEquals",
        &[
            ("mag_a", 5),
            ("neg_a", 0),
            ("mag_b", 3),
            ("neg_b", 0),
            ("mag_c", 15),
            ("neg_c", 0),
        ],
    );
    assert_eq!((ok, halt), (1, cell80::Halt::Returned));

    // Same magnitudes, but claimed sign is wrong -> 0.
    let (ok, _) = verify(
        "smag_mul_equals",
        "SmagMulEquals",
        &[
            ("mag_a", 5),
            ("neg_a", 0),
            ("mag_b", 3),
            ("neg_b", 0),
            ("mag_c", 15),
            ("neg_c", 1),
        ],
    );
    assert_eq!(ok, 0);

    // 5 * 3 = 15, opposite signs -> product negative; claimed (15, 1) matches -> 1.
    let (ok, _) = verify(
        "smag_mul_equals",
        "SmagMulEquals",
        &[
            ("mag_a", 5),
            ("neg_a", 1),
            ("mag_b", 3),
            ("neg_b", 0),
            ("mag_c", 15),
            ("neg_c", 1),
        ],
    );
    assert_eq!(ok, 1);

    // Zero magnitude with neg_a=1 -> product magnitude 0 canonicalizes to nonnegative
    // regardless of input sign; claimed (0, 1) also canonicalizes neg_c to 0 since
    // mag_c == 0 -> both land on (0, 0) -> match -> 1.
    let (ok, _) = verify(
        "smag_mul_equals",
        "SmagMulEquals",
        &[
            ("mag_a", 0),
            ("neg_a", 1),
            ("mag_b", 100),
            ("neg_b", 0),
            ("mag_c", 0),
            ("neg_c", 1),
        ],
    );
    assert_eq!(ok, 1);

    // 6 * 7 = 42, opposite signs -> (42, 1); claimed magnitude 41 is wrong -> 0.
    let (ok, _) = verify(
        "smag_mul_equals",
        "SmagMulEquals",
        &[
            ("mag_a", 6),
            ("neg_a", 0),
            ("mag_b", 7),
            ("neg_b", 1),
            ("mag_c", 41),
            ("neg_c", 1),
        ],
    );
    assert_eq!(ok, 0);

    // Overflowing magnitude multiply: mag_a = u32::MAX, mag_b = 2. wrapping_mul gives
    // 4_294_967_294, and 4_294_967_294 / 4_294_967_295 == 0 != mag_b (2), so the
    // overflow-detect idiom flags it and the verifier returns 0 regardless of the
    // claimed value -- never halts, always a verdict.
    let (ok, halt) = verify(
        "smag_mul_equals",
        "SmagMulEquals",
        &[
            ("mag_a", 4_294_967_295),
            ("neg_a", 0),
            ("mag_b", 2),
            ("neg_b", 0),
            ("mag_c", 4_294_967_294),
            ("neg_c", 0),
        ],
    );
    assert_eq!((ok, halt), (0, cell80::Halt::Returned));
}

#[test]
fn smag_div_equals_matches_hand_computed_expectations() {
    // Verifies smag_div_equals: given a claimed sign-magnitude quotient (mag_c, neg_c) of
    // (mag_a, neg_a) / (mag_b, neg_b), returns 1 iff mag_b != 0, mag_a divides mag_b evenly,
    // the magnitudes match, and the sign follows same-positive/different-negative
    // (zero-magnitude canonicalized to nonnegative) -- this is the reverse-equation
    // counterpart of the checked-arithmetic smag_div cell (which escalates on a nonzero
    // remainder; this one always returns a plain 0/1 verdict instead).
    fn step(mag_a: u64, neg_a: u64, mag_b: u64, neg_b: u64, mag_c: u64, neg_c: u64) -> u16 {
        let mut cell = StateCell::bind(&cell_src("smag_div_equals"), "SmagDivEquals", None)
            .unwrap_or_else(|e| panic!("bind smag_div_equals: {e}"));
        cell.set("mag_a", mag_a).unwrap();
        cell.set("neg_a", neg_a).unwrap();
        cell.set("mag_b", mag_b).unwrap();
        cell.set("neg_b", neg_b).unwrap();
        cell.set("mag_c", mag_c).unwrap();
        cell.set("neg_c", neg_c).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // 48 / 12 = 4 exactly, same sign (both nonnegative) -> claimed (4, +) matches -> 1.
    assert_eq!(step(48, 0, 12, 0, 4, 0), 1);
    // -48 / 12 = -4 exactly, opposite signs -> claimed (4, -) matches -> 1.
    assert_eq!(step(48, 1, 12, 0, 4, 1), 1);
    // -48 / 12 = -4 exactly, but claim says (4, +) -- wrong sign -> 0.
    assert_eq!(step(48, 1, 12, 0, 4, 0), 0);
    // 50 / 12 has remainder 2 (inexact) -> false regardless of the claimed quotient.
    assert_eq!(step(50, 0, 12, 0, 4, 0), 0);
    // mag_b == 0 (division by zero) -> always false.
    assert_eq!(step(48, 0, 0, 0, 0, 0), 0);
    // 0 / 5 = 0; dividend is a "negative zero" (mag_a=0, neg_a=1) and the claimed quotient
    // is also a "negative zero" (mag_c=0, neg_c=1) -- both canonicalize to nonnegative
    // zero, so the verdict is still 1.
    assert_eq!(step(0, 1, 5, 0, 0, 1), 1);
}

#[test]
fn clamp_equals_u32_matches_hand_computed_expectations() {
    fn verify(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, cell80::Halt) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report.result, report.halt)
    }

    // Case 1: x within [lo, hi] -> v = x = 50. claimed = 50 -> match.
    let (ok, halt) = verify(
        "clamp_equals_u32",
        "ClampEqualsWide",
        &[("x", 50), ("lo", 10), ("hi", 100), ("claimed", 50)],
    );
    assert_eq!((ok, halt), (1, cell80::Halt::Returned));

    // Case 2: x above hi -> v = hi = 100. claimed = 100 -> match.
    let (ok, _) = verify(
        "clamp_equals_u32",
        "ClampEqualsWide",
        &[("x", 500), ("lo", 10), ("hi", 100), ("claimed", 100)],
    );
    assert_eq!(ok, 1);

    // Case 3: x below lo -> v = lo = 10. claimed = 10 -> match.
    let (ok, _) = verify(
        "clamp_equals_u32",
        "ClampEqualsWide",
        &[("x", 2), ("lo", 10), ("hi", 100), ("claimed", 10)],
    );
    assert_eq!(ok, 1);

    // Case 4: x above hi, but claimed is wrong (500 instead of clamped 100) -> 0.
    let (ok, _) = verify(
        "clamp_equals_u32",
        "ClampEqualsWide",
        &[("x", 500), ("lo", 10), ("hi", 100), ("claimed", 500)],
    );
    assert_eq!(ok, 0);

    // Case 5: wide values beyond u16 range, x within range -> v = x = 4_000_000_000.
    // claimed matches -> 1.
    let (ok, _) = verify(
        "clamp_equals_u32",
        "ClampEqualsWide",
        &[
            ("x", 4_000_000_000),
            ("lo", 1_000_000_000),
            ("hi", 4_294_967_295),
            ("claimed", 4_000_000_000),
        ],
    );
    assert_eq!(ok, 1);

    // Case 6: lo == hi (degenerate range) -> v = lo = hi = 42 regardless of x. claimed
    // matches -> 1.
    let (ok, _) = verify(
        "clamp_equals_u32",
        "ClampEqualsWide",
        &[("x", 999), ("lo", 42), ("hi", 42), ("claimed", 42)],
    );
    assert_eq!(ok, 1);
}

#[test]
fn max_equals_u32_matches_hand_computed_expectations() {
    fn verify(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, cell80::Halt) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report.result, report.halt)
    }

    // Case 1: a > b -> max = a = 10. claimed = 10 -> match.
    let (ok, halt) = verify(
        "max_equals_u32",
        "MaxEqualsWide",
        &[("a", 10), ("b", 5), ("claimed", 10)],
    );
    assert_eq!((ok, halt), (1, cell80::Halt::Returned));

    // Case 2: b > a -> max = b = 10. claimed = 10 -> match.
    let (ok, _) = verify(
        "max_equals_u32",
        "MaxEqualsWide",
        &[("a", 5), ("b", 10), ("claimed", 10)],
    );
    assert_eq!(ok, 1);

    // Case 3: tie (a == b == 7) -> max_u32's own tie-break has b win, but the value is
    // the same either way (7), so claimed = 7 -> match.
    let (ok, _) = verify(
        "max_equals_u32",
        "MaxEqualsWide",
        &[("a", 7), ("b", 7), ("claimed", 7)],
    );
    assert_eq!(ok, 1);

    // Case 4: a > b -> max = 100, but claimed is wrong (99) -> 0.
    let (ok, _) = verify(
        "max_equals_u32",
        "MaxEqualsWide",
        &[("a", 100), ("b", 50), ("claimed", 99)],
    );
    assert_eq!(ok, 0);

    // Case 5: wide values beyond u16 range -> max = 4_000_000_000, claimed matches -> 1.
    let (ok, _) = verify(
        "max_equals_u32",
        "MaxEqualsWide",
        &[
            ("a", 4_000_000_000),
            ("b", 3_000_000_000),
            ("claimed", 4_000_000_000),
        ],
    );
    assert_eq!(ok, 1);

    // Case 6: both zero -> max = 0, but claimed is wrong (1) -> 0.
    let (ok, _) = verify(
        "max_equals_u32",
        "MaxEqualsWide",
        &[("a", 0), ("b", 0), ("claimed", 1)],
    );
    assert_eq!(ok, 0);
}

#[test]
fn min_equals_u32_matches_hand_computed_cases() {
    // Verifies MinEqualsWide against hand-computed expected verdicts: claimed must equal
    // whichever of a/b is smaller (ties count as equal), covering a<b, a>b, a==b, a
    // deliberately-wrong claim, and values near u32::MAX to exercise the wide domain.
    fn verify(a: u64, b: u64, claimed: u64) -> u16 {
        let mut cell = StateCell::bind(&cell_src("min_equals_u32"), "MinEqualsWide", None)
            .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("a", a).unwrap();
        cell.set("b", b).unwrap();
        cell.set("claimed", claimed).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        assert_eq!(report.halt, cell80::Halt::Returned);
        report.result
    }

    // a < b, claimed matches the true min (a) -> 1
    assert_eq!(verify(5, 10, 5), 1);
    // a > b, claimed matches the true min (b) -> 1
    assert_eq!(verify(10, 5, 5), 1);
    // a == b, claimed matches -> 1
    assert_eq!(verify(5, 5, 5), 1);
    // a < b, claimed is the wrong (larger) value -> 0
    assert_eq!(verify(5, 10, 10), 0);
    // wide values near u32::MAX -- min is the smaller of the two, claimed matches -> 1
    assert_eq!(verify(4_294_967_295, 100, 100), 1);
    // wide values near u32::MAX, claimed wrong -> 0
    assert_eq!(verify(4_294_967_295, 100, 4_294_967_295), 0);
}

// Verifies isqrt_equals_u32: IsqrtEqualsWide { n, r } -> u16, 1 if r is the true wide
// integer square root of n (the largest r with r*r <= n), else 0. Covers a perfect
// square, a non-perfect-square floor case (both correct and an off-by-one wrong claim),
// the n=0 edge case, and the top of the u32 domain (n = u32::MAX) with both a correct
// and an incorrect claim, mirroring gcd_equals_u32/lcm_equals_u32's recompute-and-compare shape.
#[test]
fn isqrt_equals_u32_matches_hand_computed_expectations() {
    fn verify(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, cell80::Halt) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report.result, report.halt)
    }

    // n = 100 is a perfect square, isqrt(100) = 10. claim r=10 -> match.
    let (ok, halt) = verify(
        "isqrt_equals_u32",
        "IsqrtEqualsWide",
        &[("n", 100), ("r", 10)],
    );
    assert_eq!((ok, halt), (1, cell80::Halt::Returned));

    // n = 99 is not a perfect square: 9*9=81 <= 99 < 100=10*10, so isqrt(99) = 9.
    let (ok, _) = verify(
        "isqrt_equals_u32",
        "IsqrtEqualsWide",
        &[("n", 99), ("r", 9)],
    );
    assert_eq!(ok, 1);

    // Same n=99, but claim r=10 (one too high) -> false.
    let (ok, _) = verify(
        "isqrt_equals_u32",
        "IsqrtEqualsWide",
        &[("n", 99), ("r", 10)],
    );
    assert_eq!(ok, 0);

    // n = 0 -> isqrt(0) = 0. claim r=0 -> match.
    let (ok, _) = verify("isqrt_equals_u32", "IsqrtEqualsWide", &[("n", 0), ("r", 0)]);
    assert_eq!(ok, 1);

    // n = u32::MAX = 4_294_967_295. 65535*65535 = 4_294_836_225 <= n, and
    // 65536*65536 = 4_294_967_296 > n, so isqrt(u32::MAX) = 65535. claim matches -> 1.
    let (ok, _) = verify(
        "isqrt_equals_u32",
        "IsqrtEqualsWide",
        &[("n", 4_294_967_295), ("r", 65535)],
    );
    assert_eq!(ok, 1);

    // Same n = u32::MAX, but a false claim one too high (65536) -> 0.
    let (ok, _) = verify(
        "isqrt_equals_u32",
        "IsqrtEqualsWide",
        &[("n", 4_294_967_295), ("r", 65536)],
    );
    assert_eq!(ok, 0);
}
