//! Host-oracle tests for the sequences pack (`cell80/cells/sequences/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::cell_src;
use cell80::{StateCell, DEFAULT_CYCLES};

#[test]
fn geometry_combinatorics_sequences_sequences_slice() {
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

    // arithmetic_series_sum: 3,5,7,9,11 (a=3,d=2,n=5) sums to 35.
    let (_, _, cell) = step(
        "arithmetic_series_sum",
        "ArithmeticSeriesSum",
        &[("a", 3), ("d", 2), ("n", 5)],
    );
    assert_eq!(cell.get("result"), Some(35));
    let (_, _, cell) = step(
        "arithmetic_series_sum",
        "ArithmeticSeriesSum",
        &[("a", 100), ("d", 0), ("n", 0)],
    );
    assert_eq!(cell.get("result"), Some(0));

    // geometric_series_sum: 2,6,18,54 (a=2,r=3,n=4) sums to 80.
    let (_, _, cell) = step(
        "geometric_series_sum",
        "GeometricSeriesSum",
        &[("a", 2), ("r", 3), ("n", 4)],
    );
    assert_eq!(cell.get("result"), Some(80));
    let (_, _, cell) = step(
        "geometric_series_sum",
        "GeometricSeriesSum",
        &[("a", 7), ("r", 0), ("n", 3)],
    );
    assert_eq!(cell.get("result"), Some(7)); // 7 + 0 + 0

}

#[test]
fn wave4_sequences_nth_term_sequences_slice() {
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

    // arithmetic_nth_u32: same sequence as arithmetic_series_sum's own test (3,5,7,9,11) —
    // the 5th term is 11, cross-checked against the already-shipped sum cell.
    let (_, _, cell) = step(
        "arithmetic_nth_u32",
        "ArithmeticNthWide",
        &[("start", 3), ("step", 2), ("n", 5)],
    );
    assert_eq!(cell.get("result"), Some(11));
    let (_, _, cell) = step(
        "arithmetic_nth_u32",
        "ArithmeticNthWide",
        &[("start", 3), ("step", 2), ("n", 1)],
    );
    assert_eq!(cell.get("result"), Some(3)); // n=1 is the first term
    let (_, report, _) = step(
        "arithmetic_nth_u32",
        "ArithmeticNthWide",
        &[("start", 3), ("step", 2), ("n", 0)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
    let (_, report, _) = step(
        "arithmetic_nth_u32",
        "ArithmeticNthWide",
        &[("start", 4_000_000_000), ("step", 4_000_000_000), ("n", 2)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // geometric_nth_checked_u32: same sequence as geometric_series_sum's own test
    // (2,6,18,54) — the 4th term is 54.
    let (_, _, cell) = step(
        "geometric_nth_checked_u32",
        "GeometricNthChecked",
        &[("start", 2), ("ratio", 3), ("n", 4)],
    );
    assert_eq!(cell.get("result"), Some(54));
    let (_, _, cell) = step(
        "geometric_nth_checked_u32",
        "GeometricNthChecked",
        &[("start", 2), ("ratio", 3), ("n", 1)],
    );
    assert_eq!(cell.get("result"), Some(2)); // n=1 is the first term
    let (_, report, _) = step(
        "geometric_nth_checked_u32",
        "GeometricNthChecked",
        &[("start", 2), ("ratio", 3), ("n", 0)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
    let (_, report, _) = step(
        "geometric_nth_checked_u32",
        "GeometricNthChecked",
        &[("start", 2), ("ratio", 100_000), ("n", 3)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // consecutive_sum_start: 5 consecutive integers (step=1) starting at 3 sum to 25
    // (3+4+5+6+7); 4 consecutive odd numbers (step=2) starting at 3 sum to 24 (3+5+7+9).
    let (_, _, cell) = step(
        "consecutive_sum_start",
        "ConsecutiveSumStart",
        &[("n", 5), ("sum", 25), ("step", 1)],
    );
    assert_eq!(cell.get("first"), Some(3));
    let (_, _, cell) = step(
        "consecutive_sum_start",
        "ConsecutiveSumStart",
        &[("n", 4), ("sum", 24), ("step", 2)],
    );
    assert_eq!(cell.get("first"), Some(3));
    let (_, report, _) = step(
        "consecutive_sum_start",
        "ConsecutiveSumStart",
        &[("n", 5), ("sum", 26), ("step", 1)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06)); // not exact
    let (_, report, _) = step(
        "consecutive_sum_start",
        "ConsecutiveSumStart",
        &[("n", 5), ("sum", 5), ("step", 1)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06)); // would go negative
    let (_, report, _) = step(
        "consecutive_sum_start",
        "ConsecutiveSumStart",
        &[("n", 0), ("sum", 5), ("step", 1)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06)); // n == 0
    // Wave 4, slice 3: sequences nth-term gap-fill — arithmetic_series_sum and
    // geometric_series_sum only ever summed a whole sequence, never returned a single
    // term; triangular had no inverse; and the original ~100-cell proposal's two
    // separately-named odd/even "consecutive sum" variants collapse into one
    // step-parameterized cell.

}
