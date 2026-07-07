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
