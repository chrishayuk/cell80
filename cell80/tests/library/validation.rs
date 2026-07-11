//! Host-oracle tests for the validation pack (`cell80/cells/validation/*.rs`). Mirrors the
//! cells' own pack-directory structure; see `cell80/tests/library/common.rs` for the
//! shared `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::run_cell;

#[test]
fn in_range_closed_open_matches_half_open_semantics() {
    // in_range_closed_open(x, lo, hi) = 1 if lo <= x < hi (closed at lo, open at hi), else 0.
    // This is the array-index/slice-bounds convention (same one point_in_rect uses per axis),
    // distinct from range_check (fully closed [lo,hi]) and between_exclusive (fully open (lo,hi)).
    let cases: &[(u16, u16, u16, u16)] = &[
        (5, 0, 10, 1),  // interior value is in range
        (0, 0, 10, 1),  // at lo: included because lo is closed
        (10, 0, 10, 0), // at hi: excluded because hi is open
        (9, 0, 10, 1),  // just below hi: included
        (11, 0, 10, 0), // above hi: excluded
        (0, 5, 5, 0),   // empty range (lo == hi): nothing satisfies
    ];

    let mut failures = Vec::new();
    for (x, lo, hi, exp) in cases {
        let got = run_cell("in_range_closed_open", &[*x, *lo, *hi]);
        if got != *exp {
            failures.push(format!(
                "in_range_closed_open({x}, {lo}, {hi}) = {got}, expected {exp}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "cell mismatches:\n{}",
        failures.join("\n")
    );
}

#[test]
fn in_range_open_closed_matches_half_open_hi_inclusive_semantics() {
    // in_range_open_closed(x, lo, hi) = 1 iff lo < x <= hi (open at lo, closed at hi) —
    // the mirror image of in_range_closed_open, and the fourth member of the
    // range_check / between_exclusive / in_range_closed_open / in_range_open_closed family.
    let expect = |x: u16, lo: u16, hi: u16| -> u16 { ((lo < x) && (x <= hi)) as u16 };
    let cases: &[(u16, u16, u16)] = &[
        (5, 0, 10),  // strictly inside -> 1
        (0, 0, 10),  // x == lo, lo is excluded -> 0
        (10, 0, 10), // x == hi, hi is included -> 1
        (11, 0, 10), // past hi -> 0
        (1, 0, 1),   // single-width interval, hi boundary -> 1
        (0, 0, 0),   // degenerate lo==hi, x==lo excluded -> 0
    ];
    for (x, lo, hi) in cases.iter().copied() {
        let got = run_cell("in_range_open_closed", &[x, lo, hi]);
        assert_eq!(
            got,
            expect(x, lo, hi),
            "in_range_open_closed({x}, {lo}, {hi}) = {got}, expected {}",
            expect(x, lo, hi)
        );
    }
}

#[test]
fn in_range_closed_open_u32_matches_wide_half_open_semantics() {
    // InRangeClosedOpenWide(x, lo, hi) = 1 iff lo <= x < hi at u32 width — the wide sibling
    // of in_range_closed_open, exercised past the u16/65535 ceiling since that's the whole
    // point of the wide variant.
    fn step(x: u32, lo: u32, hi: u32) -> u16 {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("in_range_closed_open_u32"),
            "InRangeClosedOpenWide",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("x", x as u64).unwrap();
        cell.set("lo", lo as u64).unwrap();
        cell.set("hi", hi as u64).unwrap();
        cell.run(cell80::DEFAULT_CYCLES).unwrap().result
    }

    let cases: &[(u32, u32, u32, u16)] = &[
        (5, 0, 10, 1),                 // interior value -> in range
        (0, 0, 10, 1),                 // at lo: included because lo is closed
        (10, 0, 10, 0),                // at hi: excluded because hi is open
        (5, 5, 5, 0),                  // empty range (lo == hi) -> nothing fits
        (100_000, 50_000, 200_000, 1), // interior, past the u16 ceiling
        (u32::MAX, 0, u32::MAX, 0),    // x == hi == u32::MAX, hi is open -> 0
    ];
    for (x, lo, hi, expected) in cases.iter().copied() {
        let got = step(x, lo, hi);
        assert_eq!(
            got, expected,
            "in_range_closed_open_u32(x={x}, lo={lo}, hi={hi}) = {got}, expected {expected}"
        );
    }
}

#[test]
fn in_range_open_closed_u32_matches_hand_computed_cases() {
    // in_range_open_closed_u32(x, lo, hi) = 1 iff lo < x <= hi (open at lo, closed at hi)
    // at u32 width -- the wide sibling of in_range_open_closed. All lo/hi values below
    // are chosen above u16::MAX (65535) so a truncating implementation would fail.
    fn in_range_open_closed_u32(x: u32, lo: u32, hi: u32) -> u16 {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("in_range_open_closed_u32"),
            "InRangeOpenClosedWide",
            None,
        )
        .unwrap_or_else(|e| panic!("bind in_range_open_closed_u32: {e}"));
        cell.set("x", x as u64).unwrap();
        cell.set("lo", lo as u64).unwrap();
        cell.set("hi", hi as u64).unwrap();
        cell.run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run in_range_open_closed_u32: {e}"));
        cell.get("ok").unwrap() as u16
    }

    // interior value: strictly above lo, strictly below hi.
    assert_eq!(in_range_open_closed_u32(70_000, 10_000, 200_000), 1);
    // exactly at lo: excluded, because lo is an open (exclusive) bound.
    assert_eq!(in_range_open_closed_u32(10_000, 10_000, 200_000), 0);
    // exactly at hi: included, because hi is a closed (inclusive) bound.
    assert_eq!(in_range_open_closed_u32(200_000, 10_000, 200_000), 1);
    // just above hi: excluded.
    assert_eq!(in_range_open_closed_u32(200_001, 10_000, 200_000), 0);
    // below lo entirely: excluded.
    assert_eq!(in_range_open_closed_u32(5_000, 10_000, 200_000), 0);
}
