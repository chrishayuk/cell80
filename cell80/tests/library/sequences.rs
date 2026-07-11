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

#[test]
fn series_sum_endpoints_sequences_slice() {
    // Local helper: bind SeriesSum, set first/last/count, run, return (report, cell).
    fn step(first: u64, last: u64, count: u64) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src("series_sum"), "SeriesSum", None)
            .unwrap_or_else(|e| panic!("bind series_sum: {e}"));
        cell.set("first", first).unwrap();
        cell.set("last", last).unwrap();
        cell.set("count", count).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // series_sum: 3,5,7,9,11 (first=3,last=11,count=5) sums to 35 — same sequence as
    // arithmetic_series_sum's own sanity test (a=3,d=2,n=5), cross-checked via the
    // endpoint framing instead of the (a,d,n) framing.
    let (report, cell) = step(3, 11, 5);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(35));

    // The avg2-floor trap: first=1,last=4,count=2 must give 5, not 4. avg2(1,4) floors
    // 2.5 down to 2 before any multiply, so composing via avg2-then-multiply-by-count
    // would silently corrupt this odd-endpoint-sum case (2*2=4); series_sum multiplies
    // count*(first+last)=10 first, then divides by 2, giving the exact 5.
    let (report, cell) = step(1, 4, 2);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(5));

    // count=0 is the empty series: sums to 0 regardless of first/last.
    let (report, cell) = step(5, 100, 0);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(0));

    // first=1,last=2,count=3 is not a valid integer arithmetic series (the implied
    // d = (2-1)/(3-1) = 0.5), so count*(first+last) = 9 is odd; series_sum escalates
    // out_of_domain (0xFF06) instead of silently floor-dividing to a wrong answer.
    let (report, _cell) = step(1, 2, 3);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
}

#[test]
fn arithmetic_term_index_recovers_n_sequences_slice() {
    // arithmetic_term_index inverts arithmetic_nth_u32: given start/step/term, recover
    // the 1-indexed term number n that produced it, escalating when term is out of
    // range or the gap isn't an exact multiple of step.
    fn step(fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("arithmetic_term_index"),
            "ArithmeticTermIndex",
            None,
        )
        .unwrap_or_else(|e| panic!("bind arithmetic_term_index: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // Same sequence as arithmetic_nth_u32's own test (3,5,7,9,11): term 11 is the 5th term.
    let (_, cell) = step(&[("start", 3), ("step", 2), ("term", 11)]);
    assert_eq!(cell.get("n"), Some(5));

    // term == start is always the first term (n=1), regardless of step.
    let (_, cell) = step(&[("start", 3), ("step", 2), ("term", 3)]);
    assert_eq!(cell.get("n"), Some(1));

    // A larger, non-trivial case: start=100, step=25, term=500 -> gap=400, idx=16, n=17.
    let (_, cell) = step(&[("start", 100), ("step", 25), ("term", 500)]);
    assert_eq!(cell.get("n"), Some(17));

    // term < start can never be a term of the sequence -> out_of_domain.
    let (report, _) = step(&[("start", 10), ("step", 5), ("term", 5)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // gap not an exact multiple of step -> out_of_domain (a wrong-plan signal).
    let (report, _) = step(&[("start", 3), ("step", 2), ("term", 4)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // step == 0 is ambiguous (every term of a constant sequence looks the same) -> out_of_domain.
    let (report, _) = step(&[("start", 5), ("step", 0), ("term", 5)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
}

#[test]
fn geometric_term_index_slice() {
    // Local bind/set/run helper, same shape as this file's other step() closures.
    fn step(fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("geometric_term_index"),
            "GeometricTermIndex",
            None,
        )
        .unwrap_or_else(|e| panic!("bind geometric_term_index: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // geometric_term_index: inverse of geometric_nth_checked_u32. Sequence 3,6,12,24
    // (start=3, ratio=2) -- 24 is the 4th term.
    let (report, cell) = step(&[("start", 3), ("ratio", 2), ("term", 24)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("n"), Some(4));

    // The first term always matches immediately at n=1, even when ratio==1 makes every
    // later term identical to it.
    let (report, cell) = step(&[("start", 5), ("ratio", 1), ("term", 5)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("n"), Some(1));

    // ratio==1 with a term that never equals start is a fixed point that will never be
    // reached -- escalates rather than looping forever.
    let (report, _) = step(&[("start", 5), ("ratio", 1), ("term", 7)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // start==0 makes every term 0 regardless of ratio; a nonzero target is unreachable.
    let (report, _) = step(&[("start", 0), ("ratio", 5), ("term", 7)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // ratio==0 collapses the sequence to start, then 0, 0, 0, ... -- the 2nd term is 0.
    let (report, cell) = step(&[("start", 2), ("ratio", 0), ("term", 0)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("n"), Some(2));

    // Unbounded growth (1, 65536, 65536*65536, ...) overflows u32 before ever reaching an
    // unreachable target -- escalates via the same checked-multiply bound as the forward cell.
    let (report, _) = step(&[("start", 1), ("ratio", 65536), ("term", 999_999_999)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn collatz_stopping_time_hand_computed() {
    // Hand-computed Collatz (3n+1 / n/2) stopping times, cross-checked against the
    // classic textbook sequences before trusting any compiled output.
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // n=1: already at 1, zero steps needed regardless of max_steps.
    let (_, cell) = step(
        "collatz_stopping_time",
        "CollatzStoppingTime",
        &[("n", 1), ("max_steps", 5)],
    );
    assert_eq!(cell.get("steps"), Some(0));

    // n=6: 6->3->10->5->16->8->4->2->1 is 8 steps (classic textbook example).
    let (_, cell) = step(
        "collatz_stopping_time",
        "CollatzStoppingTime",
        &[("n", 6), ("max_steps", 8)],
    );
    assert_eq!(cell.get("steps"), Some(8));

    // Same n=6 but bound one short of the true stopping time (7 < 8) escalates
    // out_of_domain rather than silently truncating or lying about the count.
    let (report, _) = step(
        "collatz_stopping_time",
        "CollatzStoppingTime",
        &[("n", 6), ("max_steps", 7)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // n=16 is a pure power of two: 16->8->4->2->1, exactly log2(16)=4 steps.
    let (_, cell) = step(
        "collatz_stopping_time",
        "CollatzStoppingTime",
        &[("n", 16), ("max_steps", 4)],
    );
    assert_eq!(cell.get("steps"), Some(4));

    // n=0 is out of domain (Collatz undefined at 0).
    let (report, _) = step(
        "collatz_stopping_time",
        "CollatzStoppingTime",
        &[("n", 0), ("max_steps", 5)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // n = u32::MAX (odd): 3n+1 overflows u32 on the very first step -> needs_wider_math,
    // not a silent wraparound.
    let (report, _) = step(
        "collatz_stopping_time",
        "CollatzStoppingTime",
        &[("n", 4_294_967_295), ("max_steps", 5)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}
