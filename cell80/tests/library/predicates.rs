//! Host-oracle tests for the predicates pack (`cell80/cells/predicates/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::{cell_src, run_cell};
use cell80::{StateCell, DEFAULT_CYCLES};

#[test]
fn wave4_width_precision_predicates_slice() {
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

    // is_lt_u32 / is_gt_u32 / is_le_u32 / is_ge_u32: wide predicates, exercised past the
    // u16 ceiling (money totals in cents routinely exceed 65535).
    assert_eq!(
        step("is_lt_u32", "IsLtWide", &[("a", 100_000), ("b", 100_001)]).0,
        1
    );
    assert_eq!(
        step("is_lt_u32", "IsLtWide", &[("a", 100_001), ("b", 100_000)]).0,
        0
    );
    assert_eq!(
        step("is_lt_u32", "IsLtWide", &[("a", 100_000), ("b", 100_000)]).0,
        0
    );
    assert_eq!(
        step("is_gt_u32", "IsGtWide", &[("a", 100_001), ("b", 100_000)]).0,
        1
    );
    assert_eq!(
        step("is_gt_u32", "IsGtWide", &[("a", 100_000), ("b", 100_000)]).0,
        0
    );
    assert_eq!(
        step("is_le_u32", "IsLeWide", &[("a", 100_000), ("b", 100_000)]).0,
        1
    );
    assert_eq!(
        step("is_le_u32", "IsLeWide", &[("a", 100_001), ("b", 100_000)]).0,
        0
    );
    assert_eq!(
        step("is_ge_u32", "IsGeWide", &[("a", 100_000), ("b", 100_000)]).0,
        1
    );
    assert_eq!(
        step("is_ge_u32", "IsGeWide", &[("a", 100_000), ("b", 100_001)]).0,
        0
    );

    // Wave 4, slice 1: width/precision gap-fill, redirected from the dead PlanFix
    // role/op/slot-validator branch to the two concrete gaps PlanFix's own findings
    // named — a missing wide-comparison family (is_lt/is_gt/is_le/is_ge only existed at
    // u16; answer_eq_u32 was the only wide predicate) and a floor sibling for
    // frac_of_whole (which only has the exact-or-escalate variant; models routinely
    // write "90% of 23"-style reasoning that doesn't divide evenly).
}

#[test]
fn first_wave_predicates_cells_match_defined_behaviour() {
    let cases: &[(&str, &[u16], u16)] = &[
        ("eq", &[5, 5], 1),
        ("eq", &[5, 6], 0),
        ("neq", &[5, 6], 1),
        ("neq", &[5, 5], 0),
        ("is_lt", &[3, 5], 1),
        ("is_lt", &[5, 5], 0),
        ("is_le", &[5, 5], 1),
        ("is_le", &[6, 5], 0),
        ("is_gt", &[6, 5], 1),
        ("is_gt", &[5, 5], 0),
        ("is_ge", &[5, 5], 1),
        ("is_ge", &[4, 5], 0),
        ("is_zero", &[0], 1),
        ("is_zero", &[3], 0),
        ("nonzero", &[3], 1),
        ("nonzero", &[0], 0),
        ("is_even", &[4], 1),
        ("is_even", &[0], 1),
        ("is_even", &[7], 0),
        ("is_odd", &[7], 1),
        ("is_odd", &[4], 0),
    ];

    let mut failures = Vec::new();
    for (id, args, exp) in cases {
        let got = run_cell(id, args);
        if got != *exp {
            failures.push(format!("{id}({args:?}) = {got}, expected {exp}"));
        }
    }
    assert!(
        failures.is_empty(),
        "cell mismatches:\n{}",
        failures.join("\n")
    );
}
