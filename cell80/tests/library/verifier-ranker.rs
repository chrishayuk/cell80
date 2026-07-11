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
