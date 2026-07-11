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
