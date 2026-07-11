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
