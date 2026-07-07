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
