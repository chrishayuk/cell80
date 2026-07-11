//! Host-oracle tests for the checked-arithmetic pack (`cell80/cells/checked-arithmetic/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::cell_src;
use cell80::{StateCell, DEFAULT_CYCLES};

#[test]
fn checked_arithmetic_state_cells_match_defined_behaviour() {
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

    // mul_u16_u16_to_u32: always exact, never escalates (max product fits u32 exactly).
    let (_, _, cell) = step(
        "mul_u16_u16_to_u32",
        "MulWide",
        &[("a", 65535), ("b", 65535)],
    );
    assert_eq!(cell.get("product"), Some(65_535u64 * 65_535));

    // add_checked_u32: normal case returns; overflow escalates.
    let (_, report, cell) = step("add_checked_u32", "AddChecked", &[("a", 10), ("b", 20)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("sum"), Some(30));
    let (_, report, _) = step(
        "add_checked_u32",
        "AddChecked",
        &[("a", (u32::MAX - 5) as u64), ("b", 10)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // sub_checked_u32: normal case returns; b > a escalates.
    let (_, report, cell) = step("sub_checked_u32", "SubChecked", &[("a", 30), ("b", 12)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("diff"), Some(18));
    let (_, report, _) = step("sub_checked_u32", "SubChecked", &[("a", 5), ("b", 12)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // div_exact_u32: evenly divisible returns; a remainder escalates (wrong-plan signal).
    let (_, report, cell) = step("div_exact_u32", "DivExact", &[("a", 100), ("b", 25)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("quotient"), Some(4));
    let (_, report, _) = step("div_exact_u32", "DivExact", &[("a", 100), ("b", 30)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // div_floor_u32 / div_ceil_u32 / mod_u32.
    let (_, _, cell) = step("div_floor_u32", "DivFloor", &[("a", 17), ("b", 5)]);
    assert_eq!(cell.get("quotient"), Some(3));
    let (_, _, cell) = step("div_ceil_u32", "DivCeil", &[("a", 17), ("b", 5)]);
    assert_eq!(cell.get("quotient"), Some(4));
    let (_, _, cell) = step("mod_u32", "ModU32", &[("a", 17), ("b", 5)]);
    assert_eq!(cell.get("rem"), Some(2));

    // fits_u16.
    assert_eq!(step("fits_u16", "FitsU16", &[("x", 65535)]).0, 1);
    assert_eq!(step("fits_u16", "FitsU16", &[("x", 65536)]).0, 0);
    // The GSM8K math-campaign foundation pack (Phase 2.3): checked u32 arithmetic that
    // escalates (Halt::Escalate(0xFF05), needs_wider_math) instead of silently wrapping —
    // distinct from safe_div/safe_mod's guard-and-sentinel convention, which hides a real
    // error behind an ordinary-looking 0.
}

#[test]
fn checked_arithmetic_wave2_cells_match_defined_behaviour() {
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

    // mul_checked_u32: exact case returns; overflow escalates.
    let (_, report, cell) = step("mul_checked_u32", "MulChecked", &[("a", 1000), ("b", 2000)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("product"), Some(2_000_000));
    let (_, report, _) = step(
        "mul_checked_u32",
        "MulChecked",
        &[("a", 100_000), ("b", 100_000)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // mul_add_checked_u32: a*b+c: exact case; multiply overflow; add overflow.
    let (_, _, cell) = step(
        "mul_add_checked_u32",
        "MulAddChecked",
        &[("a", 7), ("b", 6), ("c", 3)],
    );
    assert_eq!(cell.get("result"), Some(45));
    let (_, report, _) = step(
        "mul_add_checked_u32",
        "MulAddChecked",
        &[("a", 100_000), ("b", 100_000), ("c", 0)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
    let (_, report, _) = step(
        "mul_add_checked_u32",
        "MulAddChecked",
        &[("a", 4_000_000_000), ("b", 1), ("c", 4_000_000_000)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // mul_sub_checked_u32: a*b-c: exact case; c > product escalates.
    let (_, _, cell) = step(
        "mul_sub_checked_u32",
        "MulSubChecked",
        &[("a", 10), ("b", 5), ("c", 20)],
    );
    assert_eq!(cell.get("result"), Some(30));
    let (_, report, _) = step(
        "mul_sub_checked_u32",
        "MulSubChecked",
        &[("a", 3), ("b", 4), ("c", 100)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // mul3_checked_u32: a*b*c: exact case; overflow at either step escalates.
    let (_, _, cell) = step(
        "mul3_checked_u32",
        "Mul3Checked",
        &[("a", 2), ("b", 3), ("c", 4)],
    );
    assert_eq!(cell.get("product"), Some(24));
    let (_, report, _) = step(
        "mul3_checked_u32",
        "Mul3Checked",
        &[("a", 100_000), ("b", 100_000), ("c", 1)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // add3_checked_u32: a+b+c: exact case; overflow at either step escalates.
    let (_, _, cell) = step(
        "add3_checked_u32",
        "Add3Checked",
        &[("a", 1), ("b", 2), ("c", 3)],
    );
    assert_eq!(cell.get("sum"), Some(6));
    let (_, report, _) = step(
        "add3_checked_u32",
        "Add3Checked",
        &[("a", u32::MAX as u64), ("b", 1), ("c", 0)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // pow_checked_u32: 0^0 = 1 by convention; exact case; the last doubling to overflow.
    let (_, _, cell) = step("pow_checked_u32", "PowChecked", &[("base", 0), ("exp", 0)]);
    assert_eq!(cell.get("result"), Some(1));
    let (_, _, cell) = step("pow_checked_u32", "PowChecked", &[("base", 2), ("exp", 10)]);
    assert_eq!(cell.get("result"), Some(1024));
    let (_, report, _) = step("pow_checked_u32", "PowChecked", &[("base", 2), ("exp", 32)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // abs_diff_u32 / min_u32 / max_u32: wide siblings, exercised past the u16 ceiling.
    let (_, _, cell) = step(
        "abs_diff_u32",
        "AbsDiffWide",
        &[("a", 100_000), ("b", 30_000)],
    );
    assert_eq!(cell.get("diff"), Some(70_000));
    let (_, _, cell) = step("min_u32", "MinWide", &[("a", 100_000), ("b", 30_000)]);
    assert_eq!(cell.get("result"), Some(30_000));
    let (_, _, cell) = step("max_u32", "MaxWide", &[("a", 100_000), ("b", 30_000)]);
    assert_eq!(cell.get("result"), Some(100_000));

    // clamp_u32 / range_check_u32 / avg2_u32 / divides_u32: wide siblings.
    let (_, _, cell) = step(
        "clamp_u32",
        "ClampWide",
        &[("x", 5), ("lo", 10), ("hi", 100_000)],
    );
    assert_eq!(cell.get("result"), Some(10));
    let (_, _, cell) = step(
        "clamp_u32",
        "ClampWide",
        &[("x", 200_000), ("lo", 10), ("hi", 100_000)],
    );
    assert_eq!(cell.get("result"), Some(100_000));
    assert_eq!(
        step(
            "range_check_u32",
            "RangeCheckWide",
            &[("x", 100_000), ("lo", 10), ("hi", 200_000)]
        )
        .0,
        1
    );
    assert_eq!(
        step(
            "range_check_u32",
            "RangeCheckWide",
            &[("x", 5), ("lo", 10), ("hi", 200_000)]
        )
        .0,
        0
    );
    let (_, _, cell) = step("avg2_u32", "Avg2Wide", &[("a", 100_000), ("b", 100_002)]);
    assert_eq!(cell.get("result"), Some(100_001));
    assert_eq!(
        step("divides_u32", "DividesWide", &[("a", 7), ("b", 21)]).0,
        1
    );
    assert_eq!(
        step("divides_u32", "DividesWide", &[("a", 7), ("b", 22)]).0,
        0
    );

    // gcd_u32 / lcm_u32: wide siblings; lcm escalates on overflow, 0 if either input is 0.
    let (_, _, cell) = step("gcd_u32", "GcdWide", &[("a", 48), ("b", 18)]);
    assert_eq!(cell.get("result"), Some(6));
    let (_, _, cell) = step("lcm_u32", "LcmChecked", &[("a", 4), ("b", 6)]);
    assert_eq!(cell.get("result"), Some(12));
    let (_, _, cell) = step("lcm_u32", "LcmChecked", &[("a", 0), ("b", 6)]);
    assert_eq!(cell.get("result"), Some(0));

    // smag_add / smag_sub / smag_cmp: sign-magnitude (mag, neg) pairs, neg 0=nonneg/1=neg.
    let (_, _, cell) = step(
        "smag_add",
        "SmagAdd",
        &[("mag_a", 5), ("neg_a", 0), ("mag_b", 3), ("neg_b", 0)],
    );
    assert_eq!((cell.get("mag"), cell.get("neg")), (Some(8), Some(0))); // 5 + 3 = 8
    let (_, _, cell) = step(
        "smag_add",
        "SmagAdd",
        &[("mag_a", 5), ("neg_a", 1), ("mag_b", 3), ("neg_b", 0)],
    );
    assert_eq!((cell.get("mag"), cell.get("neg")), (Some(2), Some(1))); // -5 + 3 = -2
    let (_, _, cell) = step(
        "smag_add",
        "SmagAdd",
        &[("mag_a", 5), ("neg_a", 1), ("mag_b", 5), ("neg_b", 0)],
    );
    assert_eq!((cell.get("mag"), cell.get("neg")), (Some(0), Some(0))); // -5 + 5 = 0 (canonical)

    let (_, _, cell) = step(
        "smag_sub",
        "SmagSub",
        &[("mag_a", 5), ("neg_a", 0), ("mag_b", 3), ("neg_b", 0)],
    );
    assert_eq!((cell.get("mag"), cell.get("neg")), (Some(2), Some(0))); // 5 - 3 = 2
    let (_, _, cell) = step(
        "smag_sub",
        "SmagSub",
        &[("mag_a", 3), ("neg_a", 0), ("mag_b", 5), ("neg_b", 0)],
    );
    assert_eq!((cell.get("mag"), cell.get("neg")), (Some(2), Some(1))); // 3 - 5 = -2

    assert_eq!(
        step(
            "smag_cmp",
            "SmagCmp",
            &[("mag_a", 5), ("neg_a", 0), ("mag_b", 3), ("neg_b", 0)]
        )
        .0,
        2
    ); // 5 > 3
    assert_eq!(
        step(
            "smag_cmp",
            "SmagCmp",
            &[("mag_a", 5), ("neg_a", 1), ("mag_b", 3), ("neg_b", 0)]
        )
        .0,
        0
    ); // -5 < 3
    assert_eq!(
        step(
            "smag_cmp",
            "SmagCmp",
            &[("mag_a", 5), ("neg_a", 1), ("mag_b", 5), ("neg_b", 1)]
        )
        .0,
        1
    ); // -5 == -5
       // GSM8K math-campaign checked-arithmetic pack, second slice: closing the gap against
       // docs/math-campaign-spec.md's ~30-cell estimate — a checked multiply (the obvious
       // missing sibling of add_checked_u32/sub_checked_u32), fused multiply-add/subtract and
       // three-way variants, an exact checked power, wide siblings of several u16 cells that
       // can't represent values past 65535 (money/counts genuinely exceed that in this
       // campaign), and the sign-magnitude kernels docs/math-campaign-spec.md names as an M0
       // prerequisite (the dialect has no i32, so a signed difference is a (magnitude, sign)
       // pair instead).
}

#[test]
fn math_wave3_checked_arithmetic_slice() {
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

    // smag_mul: (-5)*3 = -15; (-4)*(-3) = 12.
    let (_, _, cell) = step(
        "smag_mul",
        "SmagMul",
        &[("mag_a", 5), ("neg_a", 1), ("mag_b", 3), ("neg_b", 0)],
    );
    assert_eq!((cell.get("mag"), cell.get("neg")), (Some(15), Some(1)));
    let (_, _, cell) = step(
        "smag_mul",
        "SmagMul",
        &[("mag_a", 4), ("neg_a", 1), ("mag_b", 3), ("neg_b", 1)],
    );
    assert_eq!((cell.get("mag"), cell.get("neg")), (Some(12), Some(0)));
    let (_, report, _) = step(
        "smag_mul",
        "SmagMul",
        &[
            ("mag_a", 100_000),
            ("neg_a", 0),
            ("mag_b", 100_000),
            ("neg_b", 0),
        ],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // smag_div: (-15)/3 = -5; (-12)/(-3) = 4; nonzero remainder escalates.
    let (_, _, cell) = step(
        "smag_div",
        "SmagDiv",
        &[("mag_a", 15), ("neg_a", 1), ("mag_b", 3), ("neg_b", 0)],
    );
    assert_eq!((cell.get("mag"), cell.get("neg")), (Some(5), Some(1)));
    let (_, _, cell) = step(
        "smag_div",
        "SmagDiv",
        &[("mag_a", 12), ("neg_a", 1), ("mag_b", 3), ("neg_b", 1)],
    );
    assert_eq!((cell.get("mag"), cell.get("neg")), (Some(4), Some(0)));
    let (_, report, _) = step(
        "smag_div",
        "SmagDiv",
        &[("mag_a", 10), ("neg_a", 0), ("mag_b", 3), ("neg_b", 0)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // GSM8K math-campaign, third slice: completes the sign-magnitude algebra
    // (smag_add/sub already landed a signed add/subtract; smag_mul/smag_div complete
    // multiply/divide — sign = same-sign-positive/different-sign-negative, magnitude
    // multiplied/divided with the pack's usual checked-overflow / exact-division
    // convention), two more fraction shapes (frac_avg2, frac_sub_from_whole — the
    // subtract-direction sibling of frac_add_whole), and lcm3 (the number-theory pack's
    // gcd/gcd3 pairing extended to lcm, inlining gcd's shared-kernel prelude call twice
    // since `lcm` itself isn't in `CELL_PRELUDE`).
}

#[test]
fn smag_max_returns_the_larger_signed_sign_magnitude_value() {
    // smag_max: larger of two (magnitude, sign) pairs, neg 0=nonneg/1=neg (per smag_add).
    // Covers same-sign, opposite-sign, negative-vs-negative (smaller magnitude wins),
    // a tie (keeps a's pair), and the out-of-domain escalation on a malformed neg field.
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

    // 5 vs 3 (both nonnegative) -> 5.
    let (_, _, cell) = step(
        "smag_max",
        "SmagMax",
        &[("mag_a", 5), ("neg_a", 0), ("mag_b", 3), ("neg_b", 0)],
    );
    assert_eq!((cell.get("mag"), cell.get("neg")), (Some(5), Some(0)));

    // -5 vs 3 -> 3 (opposite signs, b wins).
    let (_, _, cell) = step(
        "smag_max",
        "SmagMax",
        &[("mag_a", 5), ("neg_a", 1), ("mag_b", 3), ("neg_b", 0)],
    );
    assert_eq!((cell.get("mag"), cell.get("neg")), (Some(3), Some(0)));

    // -5 vs -8 -> -5 (both negative: smaller magnitude is the larger value).
    let (_, _, cell) = step(
        "smag_max",
        "SmagMax",
        &[("mag_a", 5), ("neg_a", 1), ("mag_b", 8), ("neg_b", 1)],
    );
    assert_eq!((cell.get("mag"), cell.get("neg")), (Some(5), Some(1)));

    // -5 vs -5 (tie) -> keeps a's (mag, neg).
    let (_, _, cell) = step(
        "smag_max",
        "SmagMax",
        &[("mag_a", 5), ("neg_a", 1), ("mag_b", 5), ("neg_b", 1)],
    );
    assert_eq!((cell.get("mag"), cell.get("neg")), (Some(5), Some(1)));

    // Out-of-domain neg_a escalates 0xFF06.
    let (_, report, _) = step(
        "smag_max",
        "SmagMax",
        &[("mag_a", 5), ("neg_a", 2), ("mag_b", 3), ("neg_b", 0)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
}

// round_div_checked_u32: round-to-nearest division of two u32 values (ties up), the wide,
// escalating sibling of round_div. Checks a non-tie case, an exact tie (ties up), the
// overflow-safe tie comparison near u32::MAX (where a naive 2*r >= b would silently wrap),
// and the b == 0 escalation path.
#[test]
fn round_div_checked_u32_matches_hand_computed_cases() {
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // 10 / 3 = 3.333... -> rounds down -> 3 (not a tie).
    let (report, cell) = step(
        "round_div_checked_u32",
        "RoundDivChecked",
        &[("a", 10), ("b", 3)],
    );
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("quotient"), Some(3));

    // 7 / 2 = 3.5 -> exact tie -> rounds up -> 4.
    let (report, cell) = step(
        "round_div_checked_u32",
        "RoundDivChecked",
        &[("a", 7), ("b", 2)],
    );
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("quotient"), Some(4));

    // Overflow-safe comparison: a=2_500_000_000, b=4_000_000_000, r/b = 0.625 >= 0.5 so this
    // rounds up to 1. A naive `2*r >= b` comparison would silently wrap (2*2_500_000_000
    // exceeds u32::MAX); the `r >= b - r` form used here never overflows.
    let (report, cell) = step(
        "round_div_checked_u32",
        "RoundDivChecked",
        &[("a", 2_500_000_000), ("b", 4_000_000_000)],
    );
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("quotient"), Some(1));

    // b == 0 halts with needs_wider_math (0xFF05).
    let (report, _) = step(
        "round_div_checked_u32",
        "RoundDivChecked",
        &[("a", 10), ("b", 0)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn sub3_checked_u32_matches_hand_computed_expectations() {
    // sub3_checked_u32: a-b-c, composing sub_checked_u32 twice — escalates the moment either
    // step would go negative, filling the missing arity-3 sibling of sub_checked_u32 (2-arg).
    fn step(a: u64, b: u64, c: u64) -> (cell80::Report, Option<u64>) {
        let mut cell = StateCell::bind(&cell_src("sub3_checked_u32"), "Sub3Checked", None)
            .unwrap_or_else(|e| panic!("bind sub3_checked_u32: {e}"));
        for (f, v) in [("a", a), ("b", b), ("c", c)] {
            cell.set(f, v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        let diff = cell.get("diff");
        (report, diff)
    }

    // Normal case, both steps stay nonnegative -> 100 - 30 - 20 = 50.
    let (report, diff) = step(100, 30, 20);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(diff, Some(50));

    // First step goes negative (b > a: 20 > 10) -> escalates before c is even applied.
    let (report, _) = step(10, 20, 5);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // First step fine (50 - 20 = 30) but second step goes negative (c=40 > 30) -> escalates.
    let (report, _) = step(50, 20, 40);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // Exact zero result at every step -> 25 - 25 - 0 = 0, no escalation.
    let (report, diff) = step(25, 25, 0);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(diff, Some(0));

    // Wide u32 values near the top of the range -> u32::MAX - 1 - 1 = u32::MAX - 2.
    let (report, diff) = step(u32::MAX as u64, 1, 1);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(diff, Some((u32::MAX - 2) as u64));
}

#[test]
fn add4_checked_u32_matches_defined_behaviour() {
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

    // add4_checked_u32: a+b+c+d, escalating the moment any sequential add step overflows u32
    // (composes add_checked_u32 three times: (a+b), +c, +d).

    // Exact case: comfortably within u32, no escalation.
    let (_, report, cell) = step(
        "add4_checked_u32",
        "Add4Checked",
        &[("a", 1), ("b", 2), ("c", 3), ("d", 4)],
    );
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("sum"), Some(10));

    // Exact boundary: sums to precisely u32::MAX, and every intermediate partial sum
    // (2e9, 3e9, u32::MAX) also stays in range, so no escalation fires.
    let (_, report, cell) = step(
        "add4_checked_u32",
        "Add4Checked",
        &[
            ("a", 1_000_000_000),
            ("b", 1_000_000_000),
            ("c", 1_000_000_000),
            ("d", 1_294_967_295),
        ],
    );
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("sum"), Some(u32::MAX as u64));

    // First-step overflow: a+b alone already exceeds u32::MAX.
    let (_, report, _) = step(
        "add4_checked_u32",
        "Add4Checked",
        &[("a", u32::MAX as u64), ("b", 1), ("c", 0), ("d", 0)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // Last-step overflow only: a+b+c stays in range (4_000_000_000), but adding d
    // pushes one past u32::MAX (4_294_967_296) -- escalation fires on the final add, not
    // an earlier one, proving every sequential step is checked, not just the first.
    let (_, report, _) = step(
        "add4_checked_u32",
        "Add4Checked",
        &[("a", 4_000_000_000), ("b", 0), ("c", 0), ("d", 294_967_296)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn is_coprime_u32_hand_computed() {
    // Host-oracle check for is_coprime_u32 (state cell IsCoprimeWide { a: u32, b: u32, ok: u16 }):
    // recomputes gcd(a, b) via the same inline Euclidean loop gcd_u32 uses, and asserts
    // ok/result is 1 iff that gcd is exactly 1 (coprime), else 0.
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report.result, cell)
    }

    // gcd(48, 18) = 6 -> not coprime
    let (r, cell) = step("is_coprime_u32", "IsCoprimeWide", &[("a", 48), ("b", 18)]);
    assert_eq!(r, 0);
    assert_eq!(cell.get("ok"), Some(0));

    // gcd(17, 13) = 1 (distinct primes) -> coprime
    let (r, cell) = step("is_coprime_u32", "IsCoprimeWide", &[("a", 17), ("b", 13)]);
    assert_eq!(r, 1);
    assert_eq!(cell.get("ok"), Some(1));

    // gcd(9, 28) = 1 (9 = 3^2, 28 = 2^2 * 7, no shared factor) -> coprime
    let (r, cell) = step("is_coprime_u32", "IsCoprimeWide", &[("a", 9), ("b", 28)]);
    assert_eq!(r, 1);
    assert_eq!(cell.get("ok"), Some(1));

    // gcd(0, 5) = 5 -> not coprime (gcd(0, n) = n)
    let (r, cell) = step("is_coprime_u32", "IsCoprimeWide", &[("a", 0), ("b", 5)]);
    assert_eq!(r, 0);
    assert_eq!(cell.get("ok"), Some(0));

    // gcd(1, 100) = 1 -> coprime (1 is coprime with everything)
    let (r, cell) = step("is_coprime_u32", "IsCoprimeWide", &[("a", 1), ("b", 100)]);
    assert_eq!(r, 1);
    assert_eq!(cell.get("ok"), Some(1));

    // gcd(100000, 100000) = 100000 -> not coprime (equal, non-1 values)
    let (r, cell) = step(
        "is_coprime_u32",
        "IsCoprimeWide",
        &[("a", 100_000), ("b", 100_000)],
    );
    assert_eq!(r, 0);
    assert_eq!(cell.get("ok"), Some(0));
}


#[test]
fn min3_u32_matches_hand_computed_cases() {
    // Checks Min3Wide::run against hand-computed min(min(a,b),c) over several wide u32 cases,
    // including a three-way tie, a partial tie, and values near u32::MAX to confirm no overflow
    // or truncation creeps in at the top of the range.
    fn check(a: u32, b: u32, c: u32) -> u32 {
        let mut cell = StateCell::bind(&cell_src("min3_u32"), "Min3Wide", None)
            .unwrap_or_else(|e| panic!("bind min3_u32: {e}"));
        cell.set("a", a as u64).unwrap();
        cell.set("b", b as u64).unwrap();
        cell.set("c", c as u64).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap_or_else(|e| panic!("run: {e}"));
        assert_eq!(report.halt, cell80::Halt::Returned);
        cell.get("result").unwrap() as u32
    }

    // (a, b, c, expected) -- hand-computed as min(min(a,b),c)
    let cases: &[(u32, u32, u32, u32)] = &[
        (5, 3, 8, 3),                                      // b smallest
        (100, 200, 50, 50),                                // c smallest
        (0, 0, 0, 0),                                       // all zero
        (u32::MAX, u32::MAX - 1, u32::MAX, u32::MAX - 1),   // wide values near u32::MAX
        (7, 7, 7, 7),                                        // three-way tie
        (10, 20, 10, 10),                                    // a and c tie for smallest
    ];

    let mut failures = Vec::new();
    for (a, b, c, exp) in cases {
        let got = check(*a, *b, *c);
        if got != *exp {
            failures.push(format!("min3_u32({a},{b},{c}) = {got}, expected {exp}"));
        }
    }
    assert!(failures.is_empty(), "cell mismatches:\n{}", failures.join("\n"));
}

// max3_u32: wide (u32) three-way max, exercised past the u16 ceiling and with ties.
#[test]
fn max3_u32_wide_three_way_max() {
    fn step(fields: &[(&str, u64)]) -> cell80::StateCell {
        let mut cell = StateCell::bind(&cell_src("max3_u32"), "Max3Wide", None)
            .unwrap_or_else(|e| panic!("bind max3_u32: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        assert_eq!(report.halt, cell80::Halt::Returned);
        cell
    }

    // c is the largest, well past u16 range.
    let cell = step(&[("a", 100_000), ("b", 30_000), ("c", 200_000)]);
    assert_eq!(cell.get("result"), Some(200_000));

    // a is the largest.
    let cell = step(&[("a", 500_000), ("b", 400_000), ("c", 300_000)]);
    assert_eq!(cell.get("result"), Some(500_000));

    // b is the largest.
    let cell = step(&[("a", 1), ("b", 999_999), ("c", 2)]);
    assert_eq!(cell.get("result"), Some(999_999));

    // a and c tie for largest.
    let cell = step(&[("a", 4_000_000_000), ("b", 1), ("c", 4_000_000_000)]);
    assert_eq!(cell.get("result"), Some(4_000_000_000));

    // boundary at u32::MAX.
    let cell = step(&[("a", u32::MAX as u64), ("b", (u32::MAX - 1) as u64), ("c", 0)]);
    assert_eq!(cell.get("result"), Some(u32::MAX as u64));
}

#[test]
fn sub4_checked_u32_matches_hand_computed_expectations() {
    // sub4_checked_u32: a-b-c-d, composing sub_checked_u32 three times -- escalates the
    // moment any sequential subtract step would go negative, filling the missing arity-4
    // sibling of sub_checked_u32 (2-arg) / sub3_checked_u32 (3-arg), matching add4_checked_u32.
    fn step(a: u64, b: u64, c: u64, d: u64) -> (cell80::Report, Option<u64>) {
        let mut cell = StateCell::bind(&cell_src("sub4_checked_u32"), "Sub4Checked", None)
            .unwrap_or_else(|e| panic!("bind sub4_checked_u32: {e}"));
        for (f, v) in [("a", a), ("b", b), ("c", c), ("d", d)] {
            cell.set(f, v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        let diff = cell.get("diff");
        (report, diff)
    }

    // Normal case, every step stays nonnegative -> 100-30-20-10 = 40.
    let (report, diff) = step(100, 30, 20, 10);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(diff, Some(40));

    // First step goes negative (b=20 > a=10) -> escalates immediately, before c or d apply.
    let (report, _) = step(10, 20, 5, 1);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // First step fine (50-20=30), second step goes negative (c=40 > 30) -> escalates.
    let (report, _) = step(50, 20, 40, 1);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // First two steps fine (50-20=30, 30-10=20), third step goes negative (d=25 > 20)
    // -> escalates on the final subtract, proving every sequential step is checked, not
    // just the first two (mirrors add4_checked_u32's last-step-only overflow case).
    let (report, _) = step(50, 20, 10, 25);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // Wide u32 boundary -> u32::MAX - 1 - 1 - 1 = u32::MAX - 3, no escalation.
    let (report, diff) = step(u32::MAX as u64, 1, 1, 1);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(diff, Some((u32::MAX - 3) as u64));
}

#[test]
fn mul4_checked_u32_matches_defined_behaviour() {
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

    // mul4_checked_u32: a*b*c*d, escalating the moment any sequential multiply step
    // overflows u32 (composes mul_checked_u32 three times: (a*b), *c, *d).

    // Small exact case: comfortably within u32, no escalation.
    let (_, report, cell) = step(
        "mul4_checked_u32",
        "Mul4Checked",
        &[("a", 2), ("b", 3), ("c", 4), ("d", 5)],
    );
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("product"), Some(120));

    // Zero case: any zero factor collapses the product to 0 with no overflow risk at
    // any step, regardless of how large the other factors are.
    let (_, report, cell) = step(
        "mul4_checked_u32",
        "Mul4Checked",
        &[("a", 0), ("b", 100), ("c", 100), ("d", 100)],
    );
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("product"), Some(0));

    // Exact boundary: 3*5*17*16_843_009 = u32::MAX exactly (u32::MAX factors as
    // 3*5*17*257*65537, grouped as 257*65537 = 16_843_009 for the 4th factor). Every
    // intermediate partial product (15, 255, 4_294_967_295) also stays in range, so no
    // escalation fires even though the final product lands exactly on the ceiling.
    let (_, report, cell) = step(
        "mul4_checked_u32",
        "Mul4Checked",
        &[("a", 3), ("b", 5), ("c", 17), ("d", 16_843_009)],
    );
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("product"), Some(u32::MAX as u64));

    // First-step overflow: a*b alone (100_000 * 100_000 = 10_000_000_000) already
    // exceeds u32::MAX, so escalation fires immediately regardless of c, d.
    let (_, report, _) = step(
        "mul4_checked_u32",
        "Mul4Checked",
        &[("a", 100_000), ("b", 100_000), ("c", 1), ("d", 1)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // Last-step overflow only: a*b=1_000_000 and (a*b)*c=1_000_000_000 both stay in
    // range, but the final *d (1_000_000_000 * 5 = 5_000_000_000) pushes past u32::MAX --
    // escalation fires on the final multiply, proving every sequential step is checked,
    // not just the first.
    let (_, report, _) = step(
        "mul4_checked_u32",
        "Mul4Checked",
        &[("a", 1000), ("b", 1000), ("c", 1000), ("d", 5)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn avg3_u32_hand_computed() {
    // Mirrors the checked-arithmetic pack's existing `step` helper shape (see
    // cell80/tests/library/checked-arithmetic.rs) using only cell_src/StateCell/DEFAULT_CYCLES
    // already imported at the top of that file.
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

    // avg3_u32: floor average of three wide u32 values via per-term divide-by-3 plus
    // remainder correction -- the arity-3 extension avg2_u32 lacks.

    // 1. Exact case, no remainder correction needed: (10+20+30)/3 = 20 exactly.
    let (_, report, cell) = step("avg3_u32", "Avg3Wide", &[("a", 10), ("b", 20), ("c", 30)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(20));

    // 2. Non-exact average, floors down: (10+10+11)/3 = 31/3 = 10.33 -> floor 10.
    let (_, _, cell) = step("avg3_u32", "Avg3Wide", &[("a", 10), ("b", 10), ("c", 11)]);
    assert_eq!(cell.get("result"), Some(10));

    // 3. Remainder-sum edge: each term contributes remainder 2 (2/3 = 0 r2), so the
    // remainder-correction term itself must floor(6/3) = 2 -- (2+2+2)/3 = 2 exactly.
    let (_, _, cell) = step("avg3_u32", "Avg3Wide", &[("a", 2), ("b", 2), ("c", 2)]);
    assert_eq!(cell.get("result"), Some(2));

    // 4. Large values whose sum would overflow u32 (3 * 4_000_000_000 = 12_000_000_000 >
    // u32::MAX) but the per-term divide-by-3 technique never forms that sum -- each term
    // floors to 1_333_333_333 r1, remainder-sum 3 contributes floor(3/3) = 1, giving back
    // exactly 4_000_000_000 (the shared value), proving the overflow-free claim.
    let (_, report, cell) = step(
        "avg3_u32",
        "Avg3Wide",
        &[("a", 4_000_000_000), ("b", 4_000_000_000), ("c", 4_000_000_000)],
    );
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(4_000_000_000));

    // 5. Triple u32::MAX: divides evenly by 3 (u32::MAX = 3 * 1_431_655_765), so every
    // per-term remainder is 0 and the average of three maxed-out values is exactly
    // u32::MAX itself -- again computed without ever summing past u32::MAX.
    let (_, _, cell) = step(
        "avg3_u32",
        "Avg3Wide",
        &[("a", u32::MAX as u64), ("b", u32::MAX as u64), ("c", u32::MAX as u64)],
    );
    assert_eq!(cell.get("result"), Some(u32::MAX as u64));
}

#[test]
fn gcd3_u32_matches_hand_computed_gcd_of_gcd() {
    // gcd3_u32: gcd(gcd(a,b),c) via two chained Euclidean loops, over the wide u32 domain.
    let step = |fields: &[(&str, u64)]| -> StateCell {
        let mut cell = StateCell::bind(&cell_src("gcd3_u32"), "Gcd3Wide", None)
            .unwrap_or_else(|e| panic!("bind gcd3_u32: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap();
        cell
    };

    // gcd(48,18)=6, then gcd(6,12)=6.
    let cell = step(&[("a", 48), ("b", 18), ("c", 12)]);
    assert_eq!(cell.get("result"), Some(6));

    // gcd(17,13)=1 (distinct primes, coprime), then gcd(1,5)=1.
    let cell = step(&[("a", 17), ("b", 13), ("c", 5)]);
    assert_eq!(cell.get("result"), Some(1));

    // gcd(100_000,75_000)=25_000, then gcd(25_000,50_000)=25_000 -- past the u16 ceiling.
    let cell = step(&[("a", 100_000), ("b", 75_000), ("c", 50_000)]);
    assert_eq!(cell.get("result"), Some(25_000));

    // gcd(0,0)=0, then gcd(0,7)=7 -- zero-edge case (gcd(0,n)=n).
    let cell = step(&[("a", 0), ("b", 0), ("c", 7)]);
    assert_eq!(cell.get("result"), Some(7));
}

#[test]
fn lcm3_u32_matches_defined_behaviour() {
    // Host-oracle check for lcm3_u32 (state cell Lcm3Checked { a: u32, b: u32, c: u32, result: u32 }):
    // computes lcm(lcm(a,b),c) by chaining two Euclid-gcd + checked-multiply steps, the same
    // way lcm_u32 does at arity 2. Zero convention: any input 0 gives result 0 (matches lcm_u32).
    // Overflow in either chained multiply escalates (Halt::Escalate(0xFF05), needs_wider_math).
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

    // lcm(4,6,8): lcm(4,6)=12, lcm(12,8)=24.
    let (_, report, cell) = step("lcm3_u32", "Lcm3Checked", &[("a", 4), ("b", 6), ("c", 8)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(24));

    // lcm(3,5,7): pairwise coprime, lcm = product = 105.
    let (_, report, cell) = step("lcm3_u32", "Lcm3Checked", &[("a", 3), ("b", 5), ("c", 7)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(105));

    // Zero convention: any of a/b/c being 0 yields result 0.
    let (_, report, cell) = step("lcm3_u32", "Lcm3Checked", &[("a", 0), ("b", 6), ("c", 8)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(0));
    let (_, report, cell) = step("lcm3_u32", "Lcm3Checked", &[("a", 4), ("b", 6), ("c", 0)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(0));

    // Overflow escalation: a=4_000_000_000, b=3 are coprime (a mod 3 == 1), so
    // lcm(a,b) = a*b = 12_000_000_000, which exceeds u32::MAX — the first chained
    // multiply overflows and the cell escalates before ever combining in c.
    let (_, report, _) = step(
        "lcm3_u32",
        "Lcm3Checked",
        &[("a", 4_000_000_000), ("b", 3), ("c", 1)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn smag_clamp_matches_hand_computed_cases() {
    // smag_clamp: clamp a signed (mag_x, neg_x) value into an inclusive signed range
    // [lo, hi] (each its own (mag, neg) pair, neg 0=nonneg/1=neg per smag_add), using
    // smag_cmp's sign-then-magnitude ordering. Covers: below lo, above hi, within range,
    // an inclusive boundary (x == lo exactly), and the out-of-domain escalation.
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

    // x = -8, range [-5, 10] -> -8 < -5, clamp to lo = -5 -> (mag=5, neg=1).
    let (_, report, cell) = step(
        "smag_clamp",
        "SmagClamp",
        &[
            ("mag_x", 8), ("neg_x", 1),
            ("mag_lo", 5), ("neg_lo", 1),
            ("mag_hi", 10), ("neg_hi", 0),
        ],
    );
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!((cell.get("mag"), cell.get("neg")), (Some(5), Some(1)));

    // x = 15, range [-5, 10] -> 15 > 10, clamp to hi = 10 -> (mag=10, neg=0).
    let (_, report, cell) = step(
        "smag_clamp",
        "SmagClamp",
        &[
            ("mag_x", 15), ("neg_x", 0),
            ("mag_lo", 5), ("neg_lo", 1),
            ("mag_hi", 10), ("neg_hi", 0),
        ],
    );
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!((cell.get("mag"), cell.get("neg")), (Some(10), Some(0)));

    // x = -3, range [-5, -1] -> within range, unchanged -> (mag=3, neg=1).
    let (_, report, cell) = step(
        "smag_clamp",
        "SmagClamp",
        &[
            ("mag_x", 3), ("neg_x", 1),
            ("mag_lo", 5), ("neg_lo", 1),
            ("mag_hi", 1), ("neg_hi", 1),
        ],
    );
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!((cell.get("mag"), cell.get("neg")), (Some(3), Some(1)));

    // x = -5 exactly equal to lo = -5 (inclusive boundary) -> stays x -> (mag=5, neg=1).
    let (_, report, cell) = step(
        "smag_clamp",
        "SmagClamp",
        &[
            ("mag_x", 5), ("neg_x", 1),
            ("mag_lo", 5), ("neg_lo", 1),
            ("mag_hi", 10), ("neg_hi", 0),
        ],
    );
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!((cell.get("mag"), cell.get("neg")), (Some(5), Some(1)));

    // Out-of-domain neg_hi escalates 0xFF06.
    let (_, report, _) = step(
        "smag_clamp",
        "SmagClamp",
        &[
            ("mag_x", 3), ("neg_x", 0),
            ("mag_lo", 1), ("neg_lo", 0),
            ("mag_hi", 10), ("neg_hi", 7),
        ],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
}
