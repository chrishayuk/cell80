//! Host-oracle tests for the sequences pack (`cell80/cells/sequences/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::{cell_src, run_cell};
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

#[test]
fn collatz_max_value_hand_computed() {
    // Hand-computed Collatz (3n+1 / n/2) peak values, cross-checked against the
    // classic textbook trajectories before trusting any compiled output.
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // n=1: already at 1, peak is n itself regardless of max_steps.
    let (_, cell) = step(
        "collatz_max_value",
        "CollatzMaxValue",
        &[("n", 1), ("max_steps", 5)],
    );
    assert_eq!(cell.get("max_value"), Some(1));

    // n=7: 7->22->11->34->17->52->26->13->40->20->10->5->16->8->4->2->1 is 16 steps
    // (classic textbook example); the peak value visited is 52 (reached right after 17).
    let (_, cell) = step(
        "collatz_max_value",
        "CollatzMaxValue",
        &[("n", 7), ("max_steps", 16)],
    );
    assert_eq!(cell.get("max_value"), Some(52));

    // n=6: 6->3->10->5->16->8->4->2->1 (8 steps); peak is 16, distinct from collatz_stopping_time's
    // scalar output (a count, 8) -- this cell reports the value 16 instead.
    let (_, cell) = step(
        "collatz_max_value",
        "CollatzMaxValue",
        &[("n", 6), ("max_steps", 8)],
    );
    assert_eq!(cell.get("max_value"), Some(16));

    // Same n=7 but bound one short of the true stopping time (15 < 16) escalates
    // out_of_domain rather than silently truncating or lying about the peak.
    let (report, _) = step(
        "collatz_max_value",
        "CollatzMaxValue",
        &[("n", 7), ("max_steps", 15)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // n=0 is out of domain (Collatz undefined at 0).
    let (report, _) = step(
        "collatz_max_value",
        "CollatzMaxValue",
        &[("n", 0), ("max_steps", 5)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // n = u32::MAX (odd): 3n+1 overflows u32 on the very first step -> needs_wider_math,
    // not a silent wraparound.
    let (report, _) = step(
        "collatz_max_value",
        "CollatzMaxValue",
        &[("n", 4_294_967_295), ("max_steps", 5)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn sequences_is_happy_number() {
    // is_happy_number: repeated sum-of-squared-digits, 1 = reaches 1 (happy), 0 = enters the
    // known non-happy cycle (detected by a bounded return to 4, its entry point).
    // n=0 is a special-cased fixed point (digit-square-sum of 0 is 0 forever) -- not happy.
    assert_eq!(run_cell("is_happy_number", &[0]), 0);
    // n=1: trivially happy (already 1).
    assert_eq!(run_cell("is_happy_number", &[1]), 1);
    // n=2: 2 -> 4 -> ... enters the cycle at 4 -- not happy.
    assert_eq!(run_cell("is_happy_number", &[2]), 0);
    // n=4: the cycle's entry point itself -- not happy by definition.
    assert_eq!(run_cell("is_happy_number", &[4]), 0);
    // n=7: 7 -> 49 -> 97 -> 130 -> 10 -> 1 -- happy.
    assert_eq!(run_cell("is_happy_number", &[7]), 1);
    // n=19: 19 -> 82 -> 68 -> 100 -> 1 -- happy.
    assert_eq!(run_cell("is_happy_number", &[19]), 1);
    // n=65535 (max u16 input): 65535 -> 120 -> 5 -> 25 -> 29 -> 85 -> 89 -> 145 -> 42 -> 20 -> 4 -- not happy.
    assert_eq!(run_cell("is_happy_number", &[65535]), 0);
}

#[test]
fn kaprekar_stopping_time_slice() {
    // kaprekar_stopping_time: steps of Kaprekar's routine (sort digits desc minus asc,
    // repeat) for a zero-padded 4-digit n to reach 6174. Hand-computed traces:
    //   1    -> "0001"->999(1)->8991(2)->8082(3)->8532(4)->6174(5)            = 5
    //   3524 -> 3524->3087(1)->8352(2)->6174(3)                               = 3
    //   999  -> "0999"->8991(1)->8082(2)->8532(3)->6174(4)                    = 4
    //   1234 -> 1234->3087(1)->8352(2)->6174(3)                               = 3
    //   5000 -> 5000->4995(1)->5355(2)->1998(3)->8082(4)->8532(5)->6174(6)    = 6
    //   2111 -> 2111->999(1)->8991(2)->8082(3)->8532(4)->6174(5)              = 5
    // 6174 itself is 0 steps; repdigits (all four zero-padded digits equal) never
    // converge and escalate, as does any n outside the 4-digit domain.
    fn run(id: &str, args: &[u16]) -> cell80::Report {
        let mut r =
            cell80::Runner::compile(&cell_src(id)).unwrap_or_else(|e| panic!("compile {id}: {e}"));
        r.run(None, args, DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run {id}: {e}"))
    }

    assert_eq!(run("kaprekar_stopping_time", &[6174]).result, 0);
    assert_eq!(run("kaprekar_stopping_time", &[1]).result, 5);
    assert_eq!(run("kaprekar_stopping_time", &[3524]).result, 3);
    assert_eq!(run("kaprekar_stopping_time", &[999]).result, 4);
    assert_eq!(run("kaprekar_stopping_time", &[1234]).result, 3);
    assert_eq!(run("kaprekar_stopping_time", &[5000]).result, 6);
    assert_eq!(run("kaprekar_stopping_time", &[2111]).result, 5);

    // Repdigits (all 4 zero-padded digits identical) never converge -- escalate.
    assert_eq!(
        run("kaprekar_stopping_time", &[0]).halt,
        cell80::Halt::Escalate(0xFF06)
    );
    assert_eq!(
        run("kaprekar_stopping_time", &[1111]).halt,
        cell80::Halt::Escalate(0xFF06)
    );
    assert_eq!(
        run("kaprekar_stopping_time", &[9999]).halt,
        cell80::Halt::Escalate(0xFF06)
    );

    // Out of the 4-digit domain entirely.
    assert_eq!(
        run("kaprekar_stopping_time", &[10000]).halt,
        cell80::Halt::Escalate(0xFF06)
    );
}

#[test]
fn series_term_count_hand_computed() {
    // Recovers the missing term count from an arithmetic series' endpoints and sum,
    // the exact inverse of series_sum's count*(first+last)/2 formula.
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // 3,5,7,9,11 (5 terms, first=3, last=11) sums to 35; count = 2*35/(3+11) = 70/14 = 5.
    let (_, cell) = step(
        "series_term_count",
        "SeriesTermCount",
        &[("first", 3), ("last", 11), ("sum", 35)],
    );
    assert_eq!(cell.get("count"), Some(5));

    // 5+5+5=15 (3 terms, first=last=5); count = 2*15/(5+5) = 30/10 = 3.
    let (_, cell) = step(
        "series_term_count",
        "SeriesTermCount",
        &[("first", 5), ("last", 5), ("sum", 15)],
    );
    assert_eq!(cell.get("count"), Some(3));

    // sum=0 with nonzero endpoints -> count=0 (zero terms contribute nothing).
    let (_, cell) = step(
        "series_term_count",
        "SeriesTermCount",
        &[("first", 5), ("last", 7), ("sum", 0)],
    );
    assert_eq!(cell.get("count"), Some(0));

    // first+last==0 (first=last=0) but sum!=0 -> no count can produce a nonzero sum -> out_of_domain.
    let (report, _) = step(
        "series_term_count",
        "SeriesTermCount",
        &[("first", 0), ("last", 0), ("sum", 5)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // first=1, last=2, sum=4: endpoint_sum=3, doubled=8, 8 % 3 != 0 -> not evenly divisible -> out_of_domain.
    let (report, _) = step(
        "series_term_count",
        "SeriesTermCount",
        &[("first", 1), ("last", 2), ("sum", 4)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // first=1, last=1, sum=3_000_000_000: endpoint_sum=2, doubled=6_000_000_000 overflows u32::MAX -> needs_wider_math.
    let (report, _) = step(
        "series_term_count",
        "SeriesTermCount",
        &[("first", 1), ("last", 1), ("sum", 3_000_000_000)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn arithmetic_common_diff_recovers_step_sequences_slice() {
    // arithmetic_common_diff is the third solvable unknown in start + step*(n-1) = term:
    // given start, the 1-indexed term position n, and that term's value, recover step.
    fn step(fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("arithmetic_common_diff"),
            "ArithmeticCommonDiff",
            None,
        )
        .unwrap_or_else(|e| panic!("bind arithmetic_common_diff: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // Same sequence as arithmetic_nth_u32/arithmetic_term_index tests: 3,5,7,9,11
    // (start=3, n=5, term=11). gap = 11-3 = 8, nm1 = 5-1 = 4, step = 8/4 = 2.
    let (_, cell) = step(&[("start", 3), ("n", 5), ("term", 11)]);
    assert_eq!(cell.get("step"), Some(2));

    // Larger non-trivial case: start=100, n=17, term=500.
    // gap = 500-100 = 400, nm1 = 17-1 = 16, step = 400/16 = 25.
    let (_, cell) = step(&[("start", 100), ("n", 17), ("term", 500)]);
    assert_eq!(cell.get("step"), Some(25));

    // n == 1: only the first term is pinned down; term == start -> step reported as 0
    // (unconstrained by a single point, so the canonical value is reported, not a
    // divide-by-zero halt).
    let (_, cell) = step(&[("start", 3), ("n", 1), ("term", 3)]);
    assert_eq!(cell.get("step"), Some(0));

    // term < start can never happen reading forward from start -> out_of_domain.
    let (report, _) = step(&[("start", 10), ("n", 3), ("term", 5)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // gap not an exact multiple of (n-1): start=3, n=3, term=4 -> gap=1, nm1=2, 1%2 != 0.
    let (report, _) = step(&[("start", 3), ("n", 3), ("term", 4)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // n == 1 but term != start is impossible (the first term IS start) -> out_of_domain.
    let (report, _) = step(&[("start", 5), ("n", 1), ("term", 6)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
}

#[test]
fn horner_eval_cubic_slice() {
    // Evaluates a*x^3 + b*x^2 + c*x + d via Horner's method, checked at every
    // multiply-add step; the last case forces an overflow to confirm it escalates
    // (0xFF05, needs_wider_math) instead of silently wrapping.
    fn step(fields: &[(&str, u64)]) -> (StateCell, cell80::Report) {
        let mut cell = StateCell::bind(&cell_src("horner_eval_cubic"), "HornerCubic", None)
            .unwrap_or_else(|e| panic!("bind horner_eval_cubic: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (cell, report)
    }

    // 2x^3 + 3x^2 + 4x + 5 at x=10 -> 2000 + 300 + 40 + 5 = 2345.
    let (cell, _) = step(&[("a", 2), ("b", 3), ("c", 4), ("d", 5), ("x", 10)]);
    assert_eq!(cell.get("result"), Some(2345));

    // x^3 at x=3 (a=1, b=c=d=0) -> 27.
    let (cell, _) = step(&[("a", 1), ("b", 0), ("c", 0), ("d", 0), ("x", 3)]);
    assert_eq!(cell.get("result"), Some(27));

    // x=0 collapses the polynomial to its constant term d.
    let (cell, _) = step(&[("a", 1), ("b", 2), ("c", 3), ("d", 4), ("x", 0)]);
    assert_eq!(cell.get("result"), Some(4));

    // Overflow at the very first multiply (a*x = 1_000_000 * 5000 = 5e9 > u32::MAX)
    // must escalate rather than wrap.
    let (_, report) = step(&[("a", 1_000_000), ("b", 0), ("c", 0), ("d", 0), ("x", 5000)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn arithmetic_first_term_hand_computed_cases() {
    fn cell_src() -> String {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("cells/sequences/arithmetic_first_term.rs");
        std::fs::read_to_string(path).unwrap()
    }
    fn verify(fields: &[(&str, u64)]) -> (u16, cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(), "ArithmeticFirstTerm", None).unwrap();
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        let result = report.result;
        (result, report, cell)
    }

    // Case 1: normal recovery — sequence 3,5,7,9,11 (step=2), 5th term is 11.
    // start = term - step*(n-1) = 11 - 2*4 = 3.
    let (_, _, cell) = verify(&[("step", 2), ("n", 5), ("term", 11)]);
    assert_eq!(cell.get("start"), Some(3));

    // Case 2: n=1 branch — the "first term" case is pinned directly: start = term.
    let (_, _, cell) = verify(&[("step", 2), ("n", 1), ("term", 3)]);
    assert_eq!(cell.get("start"), Some(3));

    // Case 3: step=0 — a constant sequence, every term equals start regardless of n.
    // start = 7 - 0*4 = 7.
    let (_, _, cell) = verify(&[("step", 0), ("n", 5), ("term", 7)]);
    assert_eq!(cell.get("start"), Some(7));

    // Case 4: n==0 is out of domain (no such term index) — escalates 0xFF06.
    let (_, report, _) = verify(&[("step", 2), ("n", 0), ("term", 5)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // Case 5: offset exceeds term — step*(n-1) = 5*2 = 10 > term(7), so no valid
    // nonnegative start exists in u32 domain. Escalates 0xFF06.
    let (_, report, _) = verify(&[("step", 5), ("n", 3), ("term", 7)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // Case 6: overflow — step*(n-1) = 4_000_000_000 * 2 overflows u32, so
    // mul_checked_u32 must halt with 0xFF05 (needs_wider_math) rather than wrap.
    let (_, report, _) = verify(&[("step", 4_000_000_000), ("n", 3), ("term", 100)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn geometric_first_term_hand_checked() {
    // Local bind/set/run helper, same shape as this file's other step() closures.
    fn step(fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("geometric_first_term"),
            "GeometricFirstTerm",
            None,
        )
        .unwrap_or_else(|e| panic!("bind geometric_first_term: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // geometric_first_term: inverse of geometric_nth_checked_u32. Sequence 2,6,18,54
    // (start=2, ratio=3) -- the 4th term is 54, so recovering start from (ratio=3, n=4,
    // term=54) should give back 2.
    let (report, cell) = step(&[("ratio", 3), ("n", 4), ("term", 54)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("start"), Some(2));

    // n=1 means the term IS the first term, regardless of ratio (divisor = ratio^0 = 1).
    let (report, cell) = step(&[("ratio", 5), ("n", 1), ("term", 7)]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("start"), Some(7));

    // n == 0 is out of domain -- same guard geometric_nth_checked_u32 uses.
    let (report, _) = step(&[("ratio", 2), ("n", 0), ("term", 100)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // ratio=2, n=3 -> divisor = ratio^(n-1) = 4; term=5 is not an exact multiple of 4,
    // so there's no integer start that fits -- out of domain rather than truncating.
    let (report, _) = step(&[("ratio", 2), ("n", 3), ("term", 5)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // ratio=100_000, n=3 -> building the divisor (1*100_000, then *100_000 again =
    // 10_000_000_000) overflows u32 (max ~4.29e9) -> needs_wider_math, same escalation
    // geometric_nth_checked_u32 raises when a forward term overflows.
    let (report, _) = step(&[("ratio", 100_000), ("n", 3), ("term", 1)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn series_other_endpoint_hand_computed() {
    // Recovers the missing arithmetic-series endpoint from term count, sum, and the known
    // endpoint: other = 2*sum/count - known. Symmetric inverse of series_sum/series_term_count
    // that neither of those two cells provides (they don't recover a single endpoint).
    fn step(fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("series_other_endpoint"),
            "SeriesOtherEndpoint",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // 3,5,7,9,11 (5 terms, first=3, last=11, sum=35). avg_pair = 2*35/5 = 14.
    // known=first=3 -> other should recover last=11.
    let (_, cell) = step(&[("known", 3), ("count", 5), ("sum", 35)]);
    assert_eq!(cell.get("other"), Some(11));

    // Same series, known=last=11 -> other should recover first=3 (symmetric either direction).
    let (_, cell) = step(&[("known", 11), ("count", 5), ("sum", 35)]);
    assert_eq!(cell.get("other"), Some(3));

    // 5+5+5=15 (3 terms, first=last=5). avg_pair = 2*15/3 = 10. known=5 -> other=5.
    let (_, cell) = step(&[("known", 5), ("count", 3), ("sum", 15)]);
    assert_eq!(cell.get("other"), Some(5));

    // count==0 -> out_of_domain (0xFF06): no series has zero terms with a nonzero endpoint.
    let (report, _) = step(&[("known", 5), ("count", 0), ("sum", 0)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // count=3, sum=4: doubled=8, 8 % 3 != 0 -> not evenly divisible -> out_of_domain.
    let (report, _) = step(&[("known", 1), ("count", 3), ("sum", 4)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // count=5, sum=35: avg_pair=14, known=20 > 14 -> other would go negative -> out_of_domain.
    let (report, _) = step(&[("known", 20), ("count", 5), ("sum", 35)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // sum=3_000_000_000: doubled=6_000_000_000 overflows u32::MAX -> needs_wider_math (0xFF05).
    let (report, _) = step(&[("known", 0), ("count", 2), ("sum", 3_000_000_000)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn horner_eval_quartic_slice() {
    // Evaluates a*x^4 + b*x^3 + c*x^2 + d*x + e via Horner's method, checked at every
    // multiply-add step; the last case forces an overflow to confirm it escalates
    // (0xFF05, needs_wider_math) instead of silently wrapping.
    fn step(fields: &[(&str, u64)]) -> (StateCell, cell80::Report) {
        let mut cell = StateCell::bind(&cell_src("horner_eval_quartic"), "HornerQuartic", None)
            .unwrap_or_else(|e| panic!("bind horner_eval_quartic: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (cell, report)
    }

    // 2x^4 + 3x^3 + 4x^2 + 5x + 6 at x=10 -> 20000 + 3000 + 400 + 50 + 6 = 23456.
    let (cell, _) = step(&[("a", 2), ("b", 3), ("c", 4), ("d", 5), ("e", 6), ("x", 10)]);
    assert_eq!(cell.get("result"), Some(23456));

    // x^4 at x=3 (a=1, b=c=d=e=0) -> 81.
    let (cell, _) = step(&[("a", 1), ("b", 0), ("c", 0), ("d", 0), ("e", 0), ("x", 3)]);
    assert_eq!(cell.get("result"), Some(81));

    // x=0 collapses the polynomial to its constant term e.
    let (cell, _) = step(&[("a", 1), ("b", 2), ("c", 3), ("d", 4), ("e", 5), ("x", 0)]);
    assert_eq!(cell.get("result"), Some(5));

    // Overflow at the very first multiply (a*x = 1_000_000 * 5000 = 5e9 > u32::MAX)
    // must escalate rather than wrap.
    let (_, report) = step(&[
        ("a", 1_000_000),
        ("b", 0),
        ("c", 0),
        ("d", 0),
        ("e", 0),
        ("x", 5000),
    ]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // All-ones coefficients at x=20 -> 160000 + 8000 + 400 + 20 + 1 = 168421.
    let (cell, _) = step(&[("a", 1), ("b", 1), ("c", 1), ("d", 1), ("e", 1), ("x", 20)]);
    assert_eq!(cell.get("result"), Some(168421));
}

#[test]
fn geometric_series_term_count_matches_hand_computed_values() {
    // Verifies the inverse of geometric_series_sum: given (a, r, target_sum), recover n.
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // a=2, r=3, target_sum=80 -> 2,6,18,54 sums to 80 at n=4.
    let (report, cell) = step(
        "geometric_series_term_count",
        "GeometricSeriesTermCount",
        &[("a", 2), ("r", 3), ("target_sum", 80)],
    );
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("n"), Some(4));

    // r == 1 special case: a=5, target_sum=35 -> series sum after n terms is 5*n, so n=7.
    let (report, cell) = step(
        "geometric_series_term_count",
        "GeometricSeriesTermCount",
        &[("a", 5), ("r", 1), ("target_sum", 35)],
    );
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("n"), Some(7));

    // a == 0 special case: every term is 0, so target_sum=0 is trivially met at n=0.
    let (report, cell) = step(
        "geometric_series_term_count",
        "GeometricSeriesTermCount",
        &[("a", 0), ("r", 3), ("target_sum", 0)],
    );
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("n"), Some(0));

    // Overshoot without ever matching: a=1, r=2 -> partial sums 1,3,7,15,... never hit 10,
    // so this must escalate out_of_domain rather than return a wrong nearest n.
    let (report, _cell) = step(
        "geometric_series_term_count",
        "GeometricSeriesTermCount",
        &[("a", 1), ("r", 2), ("target_sum", 10)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // Fixed-point stall: a=5, r=0 -> partial sums are 5,5,5,... (only the first term is
    // nonzero), so a target_sum of 12 can never be reached -- must escalate out_of_domain.
    let (report, _cell) = step(
        "geometric_series_term_count",
        "GeometricSeriesTermCount",
        &[("a", 5), ("r", 0), ("target_sum", 12)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
}

#[test]
fn collatz_next_hand_computed() {
    // collatz_next: one raw Collatz step (3n+1 if odd, n/2 if even), distinct from
    // collatz_stopping_time/collatz_max_value which only ever return a trajectory
    // summary -- this exposes the bare single-step transform.
    fn run(id: &str, args: &[u16]) -> cell80::Report {
        let mut r =
            cell80::Runner::compile(&cell_src(id)).unwrap_or_else(|e| panic!("compile {id}: {e}"));
        r.run(None, args, DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run {id}: {e}"))
    }

    // n=1 (odd) -> 3*1+1 = 4.
    assert_eq!(run("collatz_next", &[1]).result, 4);
    // n=4 (even) -> 4/2 = 2.
    assert_eq!(run("collatz_next", &[4]).result, 2);
    // n=7 (odd) -> 3*7+1 = 22.
    assert_eq!(run("collatz_next", &[7]).result, 22);
    // n=21843 (odd, just below the overflow boundary) -> 3*21843+1 = 65530, fits u16.
    assert_eq!(run("collatz_next", &[21843]).result, 65530);

    // n=0 is out of domain -- escalate.
    assert_eq!(
        run("collatz_next", &[0]).halt,
        cell80::Halt::Escalate(0xFF06)
    );

    // n=21845 (odd) -> 3*21845+1 = 65536, one past u16::MAX -- needs_wider_math escalation.
    assert_eq!(
        run("collatz_next", &[21845]).halt,
        cell80::Halt::Escalate(0xFF05)
    );
}
