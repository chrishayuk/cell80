//! Host-oracle tests for the running-stats pack (`cell80/cells/running-stats/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::{cell_src, run_cell};
use cell80::{StateCell, DEFAULT_CYCLES};

#[test]
fn running_stats_state_cells_match_defined_behaviour() {
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let result = cell.run(DEFAULT_CYCLES).unwrap().result;
        (result, cell)
    }

    // running_min_max_step: self-initializes on the first call (`seen` starts at 0).
    let (mut min, mut max, mut seen) = (0u64, 0u64, 0u64);
    for (value, expect_range) in [(10u64, 0u64), (3, 7), (7, 7), (20, 17), (1, 19)] {
        let (range, cell) = step(
            "running_min_max_step",
            "RunningMinMax",
            &[("value", value), ("min", min), ("max", max), ("seen", seen)],
        );
        assert_eq!(range as u64, expect_range);
        min = cell.get("min").unwrap();
        max = cell.get("max").unwrap();
        seen = cell.get("seen").unwrap();
    }
    assert_eq!((min, max), (1, 20));

    // streak_step: counts consecutive nonzero inputs, resets hard at a 0.
    let mut streak = 0u64;
    for (input, expect) in [(1u64, 1u64), (1, 2), (1, 3), (0, 0), (1, 1)] {
        let (out, cell) = step(
            "streak_step",
            "Streak",
            &[("input", input), ("streak", streak)],
        );
        assert_eq!(out as u64, expect);
        streak = cell.get("streak").unwrap();
    }

    // accumulate_step: running sum + count, saturating; compose with safe_div for a mean.
    let (mut sum, mut count) = (0u64, 0u64);
    for value in [10u64, 20, 30] {
        let (out, cell) = step(
            "accumulate_step",
            "Accumulate",
            &[("value", value), ("sum", sum), ("count", count)],
        );
        sum = cell.get("sum").unwrap();
        count = cell.get("count").unwrap();
        assert_eq!(out as u64, sum);
    }
    assert_eq!((sum, count), (60, 3));
    assert_eq!(run_cell("safe_div", &[sum as u16, count as u16]), 20); // the composed mean
    let (saturated, _) = step(
        "accumulate_step",
        "Accumulate",
        &[("value", 100), ("sum", 65_500), ("count", 5)],
    );
    assert_eq!(saturated, 65535);
    // Running-statistics state cells (wave 3), each driven over a short stream: set fields
    // by name, run, feed the updated state back as the next call's input.
}

#[test]
fn wave4_agentic_runtime_reflexes_running_stats_slice() {
    // zscore_q8: 0.25 above the mean with stddev 1.0 -> z = 0.25 (64 in Q8.8); symmetric
    // below the mean; stddev <= 0 -> 0 (the safe_div convention).
    assert_eq!(run_cell("zscore_q8", &[64, 0, 256]), 64);
    assert_eq!(run_cell("zscore_q8", &[65472, 0, 256]), 65472); // -64 as i16 bits -> -64
    assert_eq!(run_cell("zscore_q8", &[64, 0, 0]), 0);
}

#[test]
fn running_stddev_step_matches_hand_computed_values() {
    // running_stddev_step: sqrt-of-variance sibling of running_variance_step -- same
    // running (count, sum, m2) update per value, then variance = m2/count (guarded) and
    // stddev = floor(sqrt(variance)) via an inlined branch-free bitwise integer sqrt.
    fn step(fields: &[(&str, u64)]) -> (u16, StateCell) {
        let mut cell = StateCell::bind(&cell_src("running_stddev_step"), "RunningStddev", None)
            .unwrap_or_else(|e| panic!("bind: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let result = cell.run(DEFAULT_CYCLES).unwrap().result;
        (result, cell)
    }

    // Stream [10, 20, 30]: hand-derived running variance is 0, 25, then 200/3 (floor 66);
    // floor(sqrt(.)) of those is 0, 5, 8 -- 8*8=64 <= 66 < 81=9*9.
    let (mut count, mut sum, mut m2) = (0u64, 0u64, 0u64);
    let mut got = Vec::new();
    for value in [10u64, 20, 30] {
        let (out, cell) = step(&[("value", value), ("count", count), ("sum", sum), ("m2", m2)]);
        count = cell.get("count").unwrap();
        sum = cell.get("sum").unwrap();
        m2 = cell.get("m2").unwrap();
        got.push(out);
    }
    assert_eq!(got, vec![0, 5, 8]);
    assert_eq!((count, sum, m2), (3, 60, 200));

    // Stream [10, 4, 4]: exercises the negative-deviation branch (value below the running
    // mean) and a perfect-square variance (9 -> stddev 3 exactly) along the way.
    let (mut count, mut sum, mut m2) = (0u64, 0u64, 0u64);
    let mut got = Vec::new();
    for value in [10u64, 4, 4] {
        let (out, cell) = step(&[("value", value), ("count", count), ("sum", sum), ("m2", m2)]);
        count = cell.get("count").unwrap();
        sum = cell.get("sum").unwrap();
        m2 = cell.get("m2").unwrap();
        got.push(out);
    }
    assert_eq!(got, vec![0, 3, 2]); // variances 0, 9, 8 -> floor(sqrt(.)) = 0, 3, 2

    // Constant stream never accrues variance, so stddev stays 0 throughout.
    let (mut count, mut sum, mut m2) = (0u64, 0u64, 0u64);
    for value in [5u64, 5, 5] {
        let (out, cell) = step(&[("value", value), ("count", count), ("sum", sum), ("m2", m2)]);
        count = cell.get("count").unwrap();
        sum = cell.get("sum").unwrap();
        m2 = cell.get("m2").unwrap();
        assert_eq!(out, 0);
    }
}

#[test]
fn streak_best_step_tracks_longest_streak_alongside_current() {
    // streak_best_step: increments/resets exactly like streak_step, but also remembers
    // the longest streak ever seen in `best`, updating it only when the current streak
    // strictly exceeds it (a tie does not re-trigger the update). Returns `best`.
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let result = cell.run(DEFAULT_CYCLES).unwrap().result;
        (result, cell)
    }

    let (mut streak, mut best) = (0u64, 0u64);
    // inputs: 1,1,0,1,1,1 -> streak dips to 0 then climbs past the prior best of 2 to 3.
    for (input, expect_best) in [(1u64, 1u64), (1, 2), (0, 2), (1, 2), (1, 2), (1, 3)] {
        let (out, cell) = step(
            "streak_best_step",
            "StreakBest",
            &[("input", input), ("streak", streak), ("best", best)],
        );
        assert_eq!(out as u64, expect_best);
        streak = cell.get("streak").unwrap();
        best = cell.get("best").unwrap();
    }
    assert_eq!((streak, best), (3, 3));
}

#[test]
fn running_min_max_step_u32_matches_defined_behaviour() {
    // Wide u32 sibling of running_min_max_step: same self-initializing (`seen`) min/max
    // tracker, but over u32-range values and storing range (max-min) in its own field
    // rather than only returning it -- the round-trip convention running_variance_step's
    // m2/sum/count already use. run() always reports the 1u16 success flag; callers read
    // range/min/max back as fields.
    fn step(fields: &[(&str, u64)]) -> (u16, StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("running_min_max_step_u32"),
            "RunningMinMaxU32",
            None,
        )
        .unwrap_or_else(|e| panic!("bind running_min_max_step_u32: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let result = cell.run(DEFAULT_CYCLES).unwrap().result;
        (result, cell)
    }

    // Stream of values above u16::MAX to prove genuine u32 widening (not just u16 values
    // stored in wider fields): 100000, 30000, 70000, 200000, 10000.
    let (mut min, mut max, mut seen) = (0u64, 0u64, 0u64);
    for (value, expect_range) in [
        (100_000u64, 0u64),
        (30_000, 70_000),
        (70_000, 70_000),
        (200_000, 170_000),
        (10_000, 190_000),
    ] {
        let (result, cell) = step(&[("value", value), ("min", min), ("max", max), ("seen", seen)]);
        assert_eq!(result, 1u16);
        assert_eq!(
            cell.get("range").unwrap(),
            expect_range,
            "range mismatch for value={value}"
        );
        min = cell.get("min").unwrap();
        max = cell.get("max").unwrap();
        seen = cell.get("seen").unwrap();
    }
    assert_eq!((min, max), (10_000, 200_000));

    // Values that exceed u16::MAX outright: 4_000_000_000 then 100 -> range = 3_999_999_900,
    // only representable because min/max/range are u32 fields.
    let (r1, c1) = step(&[
        ("value", 4_000_000_000u64),
        ("min", 0),
        ("max", 0),
        ("seen", 0),
    ]);
    assert_eq!(r1, 1u16);
    assert_eq!(c1.get("range").unwrap(), 0);
    let (min1, max1, seen1) = (
        c1.get("min").unwrap(),
        c1.get("max").unwrap(),
        c1.get("seen").unwrap(),
    );
    let (r2, c2) = step(&[
        ("value", 100u64),
        ("min", min1),
        ("max", max1),
        ("seen", seen1),
    ]);
    assert_eq!(r2, 1u16);
    assert_eq!(c2.get("min").unwrap(), 100);
    assert_eq!(c2.get("max").unwrap(), 4_000_000_000);
    assert_eq!(c2.get("range").unwrap(), 3_999_999_900);
}
