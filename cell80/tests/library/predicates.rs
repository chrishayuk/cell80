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

#[test]
fn neq_u32_wide_not_equal_predicate() {
    // neq_u32: wide sibling of neq, exercised past the u16 ceiling (money totals in
    // cents routinely exceed 65535) — completes the six-operator wide comparison
    // family alongside is_lt_u32/is_gt_u32/is_le_u32/is_ge_u32.
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> u16 {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // Equal, past the u16 ceiling -> 0.
    assert_eq!(
        step("neq_u32", "NeqWide", &[("a", 100_000), ("b", 100_000)]),
        0
    );
    // Off by one at wide width -> 1.
    assert_eq!(
        step("neq_u32", "NeqWide", &[("a", 100_000), ("b", 100_001)]),
        1
    );
    // Order-independent: reversed operands, still not-equal -> 1.
    assert_eq!(
        step("neq_u32", "NeqWide", &[("a", 100_001), ("b", 100_000)]),
        1
    );
    // Both zero -> equal -> 0.
    assert_eq!(step("neq_u32", "NeqWide", &[("a", 0), ("b", 0)]), 0);
    // One zero, one nonzero -> 1.
    assert_eq!(step("neq_u32", "NeqWide", &[("a", 0), ("b", 1)]), 1);
    // Large equal values that would collide under mod-65536 confusion if this
    // were mistakenly wired to u16 width -> confirms genuine u32 comparison -> 0.
    assert_eq!(
        step(
            "neq_u32",
            "NeqWide",
            &[("a", 4_000_000_000), ("b", 4_000_000_000)]
        ),
        0
    );
}

// is_zero_u32 (IsZeroWide): wide-width zero check, exercised past the u16 ceiling —
// money-cents totals and other wide balances routinely exceed 65535, and is_zero
// (which is u16-only) can't safely check those without truncation risk first.
#[test]
fn is_zero_u32_wide_zero_predicate() {
    fn step(x: u64) -> u16 {
        let mut cell = StateCell::bind(&cell_src("is_zero_u32"), "IsZeroWide", None)
            .unwrap_or_else(|e| panic!("bind is_zero_u32: {e}"));
        cell.set("x", x).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    assert_eq!(step(0), 1, "0 is zero");
    assert_eq!(step(1), 0, "1 is not zero");
    assert_eq!(step(65_536), 0, "65536 (just past u16 ceiling) is not zero");
    assert_eq!(step(4_294_967_295), 0, "u32::MAX is not zero");
    assert_eq!(step(100_000), 0, "a money-cents-scale balance is not zero");
}

#[test]
fn nonzero_u32_wide_predicate() {
    // nonzero_u32 / NonzeroWide: wide sibling of nonzero, exercised past the u16 ceiling
    // (money totals in cents routinely exceed 65535).
    fn step(fields: &[(&str, u64)]) -> u16 {
        let mut cell = StateCell::bind(&cell_src("nonzero_u32"), "NonzeroWide", None)
            .unwrap_or_else(|e| panic!("bind nonzero_u32: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // x = 0 -> 0 (the zero case)
    assert_eq!(step(&[("x", 0)]).clone(), 0);
    // x = 1 -> 1 (smallest nonzero value)
    assert_eq!(step(&[("x", 1)]), 1);
    // x = 65_535 -> 1 (u16 max, still nonzero — sanity check at the old narrow ceiling)
    assert_eq!(step(&[("x", 65_535)]), 1);
    // x = 65_536 -> 1 (exceeds u16 range entirely; only representable because the field is u32)
    assert_eq!(step(&[("x", 65_536)]), 1);
    // x = 4_294_967_295 -> 1 (u32 max, upper bound of the wide field)
    assert_eq!(step(&[("x", 4_294_967_295)]), 1);
}
