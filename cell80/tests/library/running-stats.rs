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
fn sliding_window_cells_match_defined_behaviour() {
    // The first true sliding-window cells (`.cell` v11 array-state surface): state
    // persistence is host re-feed as everywhere in this file, the window array now
    // riding set_array/get_array by name. Each cell is checked against an exact
    // integer mirror oracle (same truncation, same walk) computed in host Rust.

    // simple_moving_average — the experiment's verified 10-step expectations
    // (experiments/sliding-window-state-cells-findings.md), now through the named
    // surface instead of hand-computed raw address triples.
    let mut sma = StateCell::bind(
        &cell_src("simple_moving_average"),
        "SimpleMovingAverage",
        None,
    )
    .unwrap_or_else(|e| panic!("bind sma: {e}"));
    #[rustfmt::skip]
    let expect = [
        (10u64, 10u64), (20, 15), (30, 20), (40, 25), (50, 30),
        (60, 35), (70, 40), (80, 45), (90, 55), (100, 65),
    ];
    for (value, want) in expect {
        let (window, head, count, sum) = (
            sma.get_array("window").unwrap(),
            sma.get("head").unwrap(),
            sma.get("count").unwrap(),
            sma.get("sum").unwrap(),
        );
        sma.set("value", value).unwrap();
        sma.set_array("window", &window).unwrap();
        sma.set("head", head).unwrap();
        sma.set("count", count).unwrap();
        sma.set("sum", sum).unwrap();
        let out = sma.run(DEFAULT_CYCLES).unwrap().result;
        assert_eq!(out as u64, want, "sma(value={value})");
    }

    // A shared harness for the three ring-walk cells: feed a stream, re-feeding the
    // full state (window included) each call, returning the per-step results.
    fn drive(id: &str, strct: &str, scalars: &[&str], stream: &[u64]) -> Vec<u64> {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        let mut outs = Vec::new();
        for &value in stream {
            let window = cell.get_array("window").unwrap();
            let prior: Vec<(String, u64)> = scalars
                .iter()
                .map(|f| (f.to_string(), cell.get(f).unwrap()))
                .collect();
            cell.set("value", value).unwrap();
            cell.set_array("window", &window).unwrap();
            for (f, v) in &prior {
                cell.set(f, *v).unwrap();
            }
            let result = cell.run(DEFAULT_CYCLES).unwrap().result as u64;
            // The observable: the return for wma/std; the wide `var` state field
            // for rolling_variance (which returns 1, its running sibling's shape).
            outs.push(match id {
                "rolling_variance" => cell.get("var").unwrap(),
                _ => result,
            });
        }
        outs
    }

    // weighted_moving_average — mirror oracle: linear weights 1..count over the
    // window oldest→newest, integer division.
    let stream = [10u64, 20, 30, 40, 50, 60, 70, 80, 90, 100];
    let got = drive(
        "weighted_moving_average",
        "WeightedMovingAverage",
        &["head", "count"],
        &stream,
    );
    let mut ring: Vec<u64> = Vec::new();
    for (i, &v) in stream.iter().enumerate() {
        ring.push(v);
        if ring.len() > 8 {
            ring.remove(0);
        }
        let num: u64 = ring
            .iter()
            .enumerate()
            .map(|(j, &x)| (j as u64 + 1) * x)
            .sum();
        let den: u64 = (1..=ring.len() as u64).sum();
        assert_eq!(got[i], num / den, "wma step {i}");
    }

    // rolling_variance — mirror oracle: truncated mean, squared-deviation walk,
    // truncated divide. An old outlier must age out (the windowed-vs-cumulative
    // distinction that makes this a different cell from running_variance_step).
    let stream = [
        100u64, 100, 100, 5000, 100, 100, 100, 100, 100, 100, 100, 100,
    ];
    let got = drive(
        "rolling_variance",
        "RollingVariance",
        &["head", "count", "sum"],
        &stream,
    );
    let mut ring: Vec<u64> = Vec::new();
    for (i, &v) in stream.iter().enumerate() {
        ring.push(v);
        if ring.len() > 8 {
            ring.remove(0);
        }
        let mean = ring.iter().sum::<u64>() / ring.len() as u64;
        let ssd: u64 = ring.iter().map(|&x| x.abs_diff(mean).pow(2)).sum();
        assert_eq!(got[i], ssd / ring.len() as u64, "var step {i}");
    }
    // The outlier left the window at step 11 (index 11): variance back to zero —
    // the cumulative sibling can never do this.
    assert_eq!(got[11], 0, "outlier must age out of the window");

    // rolling_std — floor(sqrt(rolling variance)).
    let got = drive(
        "rolling_std",
        "RollingStd",
        &["head", "count", "sum"],
        &stream,
    );
    let mut ring: Vec<u64> = Vec::new();
    for (i, &v) in stream.iter().enumerate() {
        ring.push(v);
        if ring.len() > 8 {
            ring.remove(0);
        }
        let mean = ring.iter().sum::<u64>() / ring.len() as u64;
        let ssd: u64 = ring.iter().map(|&x| x.abs_diff(mean).pow(2)).sum();
        assert_eq!(got[i], (ssd / ring.len() as u64).isqrt(), "std step {i}");
    }
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

#[test]
fn accumulate_step_u32_matches_defined_behaviour() {
    // Wide/checked sibling of accumulate_step: same running sum+count over a stream, but
    // u32-domain and escalating (halt 0xFF05) on sum overflow instead of saturating at 65535.
    fn step(fields: &[(&str, u64)]) -> (u16, cell80::Halt, StateCell) {
        let mut cell = StateCell::bind(&cell_src("accumulate_step_u32"), "AccumulateU32", None)
            .unwrap_or_else(|e| panic!("bind accumulate_step_u32: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report.result, report.halt, cell)
    }

    // Stream of u32-range values (well above u16::MAX) accumulating exactly, one call per value.
    let (mut sum, mut count) = (0u64, 0u64);
    for (value, expect_sum, expect_count) in [
        (100_000u64, 100_000u64, 1u64),
        (200_000, 300_000, 2),
        (50, 300_050, 3),
    ] {
        let (result, halt, cell) = step(&[("value", value), ("sum", sum), ("count", count)]);
        assert_eq!(result, 1u16);
        assert_eq!(halt, cell80::Halt::Returned);
        sum = cell.get("sum").unwrap();
        count = cell.get("count").unwrap();
        assert_eq!(sum, expect_sum, "sum mismatch for value={value}");
        assert_eq!(count, expect_count);
    }

    // sum at u32::MAX, value=0 -> no overflow, sum unchanged, count still increments.
    let u32_max: u64 = 0xFFFF_FFFF;
    let (result, halt, cell) = step(&[("value", 0u64), ("sum", u32_max), ("count", 9)]);
    assert_eq!(result, 1u16);
    assert_eq!(halt, cell80::Halt::Returned);
    assert_eq!(cell.get("sum").unwrap(), u32_max);
    assert_eq!(cell.get("count").unwrap(), 10);

    // sum at u32::MAX, value=1 -> genuine overflow -> escalate (halt 0xFF05), never a silent wrap.
    let (_result, halt, _cell) = step(&[("value", 1u64), ("sum", u32_max), ("count", 10)]);
    assert_eq!(halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn running_covariance_step_matches_defined_behaviour() {
    // running_covariance_step: bivariate-stream counterpart of running_variance_step --
    // accumulates count, sum_x, sum_y, sum_xy one (x,y) pair per call (checked/escalating
    // on u32 overflow, matching covariance's own downstream sum consumption).
    fn step(fields: &[(&str, u64)]) -> (u16, cell80::Report, StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("running_covariance_step"),
            "RunningCovariance",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        let result = report.result;
        (result, report, cell)
    }

    // Stream of (x,y) pairs (2,3), (4,5), (6,1). Hand-computed:
    //  call1: xy=6;  sum_x=2,  sum_y=3,  sum_xy=6,  count=1
    //  call2: xy=20; sum_x=6,  sum_y=8,  sum_xy=26, count=2
    //  call3: xy=6;  sum_x=12, sum_y=9,  sum_xy=32, count=3
    let (mut count, mut sum_x, mut sum_y, mut sum_xy) = (0u64, 0u64, 0u64, 0u64);
    let mut results = Vec::new();
    for (x, y) in [(2u64, 3u64), (4, 5), (6, 1)] {
        let (out, report, cell) = step(&[
            ("x", x),
            ("y", y),
            ("count", count),
            ("sum_x", sum_x),
            ("sum_y", sum_y),
            ("sum_xy", sum_xy),
        ]);
        assert_eq!(report.halt, cell80::Halt::Returned);
        count = cell.get("count").unwrap();
        sum_x = cell.get("sum_x").unwrap();
        sum_y = cell.get("sum_y").unwrap();
        sum_xy = cell.get("sum_xy").unwrap();
        results.push(out);
    }
    assert_eq!(results, vec![1, 1, 1]);
    assert_eq!((count, sum_x, sum_y, sum_xy), (3, 12, 9, 32));

    // sum_xy overflow escalation: x=y=65535 -> xy = 65535*65535 = 4294836225 (fits u32,
    // max 4294967295). Seed sum_xy with 4294836225 already accumulated so this call's
    // add_checked_u32(sum_xy, xy) needs 4294836225 + 4294836225 = 8589672450, which
    // overflows u32 -> halt 0xFF05 (needs_wider_math).
    let (_, report, _) = step(&[
        ("x", 65535),
        ("y", 65535),
        ("count", 1),
        ("sum_x", 65535),
        ("sum_y", 65535),
        ("sum_xy", 4294836225),
    ]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // sum_x overflow escalation: seeded sum_x = u32::MAX - 5 = 4294967290; adding x=10
    // needs 4294967300, which overflows u32 -> halt 0xFF05.
    let (_, report, _) = step(&[
        ("x", 10),
        ("y", 0),
        ("count", 0),
        ("sum_x", (u32::MAX - 5) as u64),
        ("sum_y", 0),
        ("sum_xy", 0),
    ]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn running_sample_stddev_step_matches_hand_computed_values() {
    // running_sample_stddev_step: Bessel-corrected (n-1 denominator) sibling of
    // running_stddev_step -- identical running (count, sum, m2) update per value, but
    // variance = m2/(count-1) instead of m2/count, guarded to 0 while count < 2.
    fn step(fields: &[(&str, u64)]) -> (u16, StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("running_sample_stddev_step"),
            "RunningSampleStddev",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let result = cell.run(DEFAULT_CYCLES).unwrap().result;
        (result, cell)
    }

    // Stream [10, 20, 30]: m2 trajectory 0, 50, 200 (identical to running_stddev_step's
    // m2 -- only the divisor differs). Sample variance = m2/(count-1): n/a, 50/1=50,
    // 200/2=100. floor(sqrt(.)): 0, 7 (7*7=49<=50<64), 10 (exact).
    let (mut count, mut sum, mut m2) = (0u64, 0u64, 0u64);
    let mut got = Vec::new();
    for value in [10u64, 20, 30] {
        let (out, cell) = step(&[("value", value), ("count", count), ("sum", sum), ("m2", m2)]);
        count = cell.get("count").unwrap();
        sum = cell.get("sum").unwrap();
        m2 = cell.get("m2").unwrap();
        got.push(out);
    }
    assert_eq!(got, vec![0, 7, 10]);
    assert_eq!((count, sum, m2), (3, 60, 200));

    // Discriminating case vs. the population sibling running_stddev_step: stream [10, 20].
    // After 2 values m2=50. Population stddev = floor(sqrt(50/2)) = floor(sqrt(25)) = 5,
    // but sample stddev = floor(sqrt(50/1)) = floor(sqrt(50)) = 7 -- proves the (count-1)
    // denominator is actually wired in, not a no-op alias of the population version.
    let (out1, cell1) = step(&[("value", 10u64), ("count", 0), ("sum", 0), ("m2", 0)]);
    assert_eq!(out1, 0); // count becomes 1, still < 2 -> guarded to 0, not a divide
    let (count1, sum1, m2_1) = (
        cell1.get("count").unwrap(),
        cell1.get("sum").unwrap(),
        cell1.get("m2").unwrap(),
    );
    let (out2, _cell2) = step(&[
        ("value", 20u64),
        ("count", count1),
        ("sum", sum1),
        ("m2", m2_1),
    ]);
    assert_eq!(out2, 7);
}

#[test]
fn running_min_max_step_i16_matches_hand_computed_values() {
    // running_min_max_step_i16: signed sibling of running_min_max_step -- same seen/min/max
    // update logic, over i16 values, returning range = max - min via sign-magnitude (the
    // abs_diff_i16 shape), since max=i16::MAX and min=i16::MIN would overflow i16 by one
    // before a native subtraction could produce the range.
    fn i16_bits(v: i16) -> u64 {
        (v as u16) as u64
    }
    fn step(fields: &[(&str, u64)]) -> (u16, StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("running_min_max_step_i16"),
            "RunningMinMaxI16",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let result = cell.run(DEFAULT_CYCLES).unwrap().result;
        (result, cell)
    }

    // Stream 10, -5, 20, i16::MIN, i16::MAX -- hand-computed range after each call:
    // 0, 15, 25, 32788, 65535 (the last is the extreme case: i16::MAX - i16::MIN = 65535,
    // exactly u16::MAX, confirming the range always fits in u16).
    let (mut min, mut max, mut seen) = (i16_bits(0), i16_bits(0), 0u64);
    let mut got = Vec::new();
    for value in [10i16, -5, 20, i16::MIN, i16::MAX] {
        let (range, cell) = step(&[
            ("value", i16_bits(value)),
            ("min", min),
            ("max", max),
            ("seen", seen),
        ]);
        min = cell.get("min").unwrap();
        max = cell.get("max").unwrap();
        seen = cell.get("seen").unwrap();
        got.push(range);
    }
    assert_eq!(got, vec![0u16, 15, 25, 32788, 65535]);
    assert_eq!((min as u16 as i16, max as u16 as i16), (i16::MIN, i16::MAX));

    // All-negative stream -10, -20, -3 -- exercises the same-sign (both negative) branch
    // of the sign-magnitude range subtract, distinct from the opposite-sign add above.
    let (mut min2, mut max2, mut seen2) = (i16_bits(0), i16_bits(0), 0u64);
    let mut got2 = Vec::new();
    for value in [-10i16, -20, -3] {
        let (range, cell) = step(&[
            ("value", i16_bits(value)),
            ("min", min2),
            ("max", max2),
            ("seen", seen2),
        ]);
        min2 = cell.get("min").unwrap();
        max2 = cell.get("max").unwrap();
        seen2 = cell.get("seen").unwrap();
        got2.push(range);
    }
    assert_eq!(got2, vec![0u16, 10, 17]);
    assert_eq!((min2 as u16 as i16, max2 as u16 as i16), (-20, -3));
}

#[test]
fn accumulate_step_i16_matches_hand_computed_values() {
    // accumulate_step_i16: signed sibling of accumulate_step/accumulate_step_u32 -- running
    // sum tracked as a sign-magnitude pair (sum_mag, sum_neg) plus count, over a stream of
    // i16 values. Same-sign combines via add_checked_u32 of magnitudes; opposite-sign
    // combines via subtracting the smaller magnitude from the larger, with the winner's
    // sign carried through (forced nonnegative on exact cancellation).
    fn i16_bits(v: i16) -> u64 {
        (v as u16) as u64
    }
    fn step(fields: &[(&str, u64)]) -> (u16, cell80::Halt, StateCell) {
        let mut cell = StateCell::bind(&cell_src("accumulate_step_i16"), "AccumulateI16", None)
            .unwrap_or_else(|e| panic!("bind: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report.result, report.halt, cell)
    }

    // Mixed-sign stream 10, -5, 20 -> running sum 10, 5, 25 (all nonnegative along the way).
    let (mut sum_mag, mut sum_neg, mut count) = (0u64, 0u64, 0u64);
    let mut mags = Vec::new();
    for value in [10i16, -5, 20] {
        let (result, halt, cell) = step(&[
            ("value", i16_bits(value)),
            ("sum_mag", sum_mag),
            ("sum_neg", sum_neg),
            ("count", count),
        ]);
        assert_eq!(result, 1u16);
        assert_eq!(halt, cell80::Halt::Returned);
        sum_mag = cell.get("sum_mag").unwrap();
        sum_neg = cell.get("sum_neg").unwrap();
        count = cell.get("count").unwrap();
        mags.push((sum_mag, sum_neg));
    }
    assert_eq!(mags, vec![(10, 0), (5, 0), (25, 0)]);
    assert_eq!(count, 3);

    // Negative-heavy stream -10, -20, 5 -> running sum -10, -30, -25 (sum_neg stays 1
    // through the same-sign accumulation, then the opposite-sign step keeps the sign since
    // the negative side's magnitude (30) still exceeds the positive nudge (5)).
    let (mut sum_mag2, mut sum_neg2, mut count2) = (0u64, 0u64, 0u64);
    for value in [-10i16, -20, 5] {
        let (_, halt, cell) = step(&[
            ("value", i16_bits(value)),
            ("sum_mag", sum_mag2),
            ("sum_neg", sum_neg2),
            ("count", count2),
        ]);
        assert_eq!(halt, cell80::Halt::Returned);
        sum_mag2 = cell.get("sum_mag").unwrap();
        sum_neg2 = cell.get("sum_neg").unwrap();
        count2 = cell.get("count").unwrap();
    }
    assert_eq!((sum_mag2, sum_neg2, count2), (25, 1, 3));

    // Exact cancellation 7 + (-7) = 0 must force sum_neg back to 0, not leave a "negative
    // zero" -- the same convention linear_solve_1var/lerp_i16 enforce.
    let (_, h1, c1) = step(&[
        ("value", i16_bits(7)),
        ("sum_mag", 0),
        ("sum_neg", 0),
        ("count", 0),
    ]);
    assert_eq!(h1, cell80::Halt::Returned);
    let (sm1, sn1, cnt1) = (
        c1.get("sum_mag").unwrap(),
        c1.get("sum_neg").unwrap(),
        c1.get("count").unwrap(),
    );
    let (_, h2, c2) = step(&[
        ("value", i16_bits(-7)),
        ("sum_mag", sm1),
        ("sum_neg", sn1),
        ("count", cnt1),
    ]);
    assert_eq!(h2, cell80::Halt::Returned);
    assert_eq!(
        (
            c2.get("sum_mag").unwrap(),
            c2.get("sum_neg").unwrap(),
            c2.get("count").unwrap()
        ),
        (0, 0, 2)
    );

    // i16::MIN (-32768) is the classic negation edge case for a hand-rolled abs(); the
    // wrapping_sub-based i16_mag helper must report magnitude 32768, not overflow.
    let (r_min, h_min, c_min) = step(&[
        ("value", i16_bits(i16::MIN)),
        ("sum_mag", 0),
        ("sum_neg", 0),
        ("count", 0),
    ]);
    assert_eq!(r_min, 1u16);
    assert_eq!(h_min, cell80::Halt::Returned);
    assert_eq!(
        (
            c_min.get("sum_mag").unwrap(),
            c_min.get("sum_neg").unwrap(),
            c_min.get("count").unwrap()
        ),
        (32768, 1, 1)
    );

    // Magnitude overflow escalates rather than silently wrapping: sum_mag seeded at
    // u32::MAX, a same-sign add of 1 -> halt 0xFF05 (needs_wider_math).
    let (_, halt_overflow, _) = step(&[
        ("value", i16_bits(1)),
        ("sum_mag", u32::MAX as u64),
        ("sum_neg", 0),
        ("count", 5),
    ]);
    assert_eq!(halt_overflow, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn running_correlation_sums_step_matches_hand_computed_values() {
    // running_correlation_sums_step: widens running_covariance_step's four-sum stream
    // (n, sum_x, sum_y, sum_xy) with sum_x2/sum_y2 so statistics/correlation and
    // linear_regression_slope/intercept can be fed from an online stream, one (x,y) pair
    // per call, checked/escalating on u32 overflow the same way running_covariance_step does.
    fn step(fields: &[(&str, u64)]) -> (u16, cell80::Report, StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("running_correlation_sums_step"),
            "RunningCorrelationSums",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        let result = report.result;
        (result, report, cell)
    }

    // Stream of (x,y) pairs (2,3), (4,5), (6,1). Hand-computed:
    //  call1: xy=6,  x2=4,  y2=9;  sum_x=2,  sum_y=3, sum_xy=6,  sum_x2=4,  sum_y2=9,  count=1
    //  call2: xy=20, x2=16, y2=25; sum_x=6,  sum_y=8, sum_xy=26, sum_x2=20, sum_y2=34, count=2
    //  call3: xy=6,  x2=36, y2=1;  sum_x=12, sum_y=9, sum_xy=32, sum_x2=56, sum_y2=35, count=3
    let (mut count, mut sum_x, mut sum_y, mut sum_xy, mut sum_x2, mut sum_y2) =
        (0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
    let mut results = Vec::new();
    for (x, y) in [(2u64, 3u64), (4, 5), (6, 1)] {
        let (out, report, cell) = step(&[
            ("x", x),
            ("y", y),
            ("count", count),
            ("sum_x", sum_x),
            ("sum_y", sum_y),
            ("sum_xy", sum_xy),
            ("sum_x2", sum_x2),
            ("sum_y2", sum_y2),
        ]);
        assert_eq!(report.halt, cell80::Halt::Returned);
        count = cell.get("count").unwrap();
        sum_x = cell.get("sum_x").unwrap();
        sum_y = cell.get("sum_y").unwrap();
        sum_xy = cell.get("sum_xy").unwrap();
        sum_x2 = cell.get("sum_x2").unwrap();
        sum_y2 = cell.get("sum_y2").unwrap();
        results.push(out);
    }
    assert_eq!(results, vec![1, 1, 1]);
    assert_eq!(
        (count, sum_x, sum_y, sum_xy, sum_x2, sum_y2),
        (3, 12, 9, 32, 56, 35)
    );

    // sum_x2 overflow escalation: x=65535 -> x2 = 65535*65535 = 4294836225 (fits u32, max
    // 4294967295). Seed sum_x2 with 4294836225 already accumulated so this call's
    // add_checked_u32(sum_x2, x2) needs 4294836225 + 4294836225 = 8589672450, which
    // overflows u32 -> halt 0xFF05 (needs_wider_math).
    let (_, report, _) = step(&[
        ("x", 65535),
        ("y", 0),
        ("count", 1),
        ("sum_x", 0),
        ("sum_y", 0),
        ("sum_xy", 0),
        ("sum_x2", 4294836225),
        ("sum_y2", 0),
    ]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // sum_y2 overflow escalation: mirror of the above with x/y swapped, proving the y2
    // accumulation is independently guarded (not just an alias of sum_x2's check).
    let (_, report, _) = step(&[
        ("x", 0),
        ("y", 65535),
        ("count", 1),
        ("sum_x", 0),
        ("sum_y", 0),
        ("sum_xy", 0),
        ("sum_x2", 0),
        ("sum_y2", 4294836225),
    ]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // sum_xy overflow escalation, matching running_covariance_step's own sibling case:
    // x=y=65535 -> xy = 4294836225; seeded sum_xy = 4294836225 -> overflow -> halt 0xFF05.
    let (_, report, _) = step(&[
        ("x", 65535),
        ("y", 65535),
        ("count", 1),
        ("sum_x", 0),
        ("sum_y", 0),
        ("sum_xy", 4294836225),
        ("sum_x2", 0),
        ("sum_y2", 0),
    ]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn running_mad_step_matches_hand_computed_values() {
    // running_mad_step: MAD sibling of running_variance_step/running_stddev_step -- same
    // (count, sum) running update per streamed value, but accumulates sum_abs_dev +=
    // |x_i - running_mean_at_time_i| (plain absolute value, no squared-product machinery).
    fn step(fields: &[(&str, u64)]) -> (u16, cell80::Halt, StateCell) {
        let mut cell = StateCell::bind(&cell_src("running_mad_step"), "RunningMad", None)
            .unwrap_or_else(|e| panic!("bind: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report.result, report.halt, cell)
    }

    // Stream [10, 20, 30]. Hand-computed:
    //  call1: new_sum=10, new_count=1, mean=10, abs_dev=0,  sum_abs_dev=0
    //  call2: new_sum=30, new_count=2, mean=15, abs_dev=5,  sum_abs_dev=5
    //  call3: new_sum=60, new_count=3, mean=20, abs_dev=10, sum_abs_dev=15
    let (mut sum, mut count, mut sum_abs_dev) = (0u64, 0u64, 0u64);
    let mut results = Vec::new();
    for value in [10u64, 20, 30] {
        let (out, halt, cell) = step(&[
            ("value", value),
            ("sum", sum),
            ("count", count),
            ("sum_abs_dev", sum_abs_dev),
        ]);
        assert_eq!(halt, cell80::Halt::Returned);
        sum = cell.get("sum").unwrap();
        count = cell.get("count").unwrap();
        sum_abs_dev = cell.get("sum_abs_dev").unwrap();
        results.push(out);
    }
    assert_eq!(results, vec![1, 1, 1]);
    assert_eq!((sum, count, sum_abs_dev), (60, 3, 15));

    // Constant stream [5, 5, 5]: the running mean always equals the value itself, so
    // sum_abs_dev must stay 0 throughout every call.
    let (mut sum2, mut count2, mut sum_abs_dev2) = (0u64, 0u64, 0u64);
    for value in [5u64, 5, 5] {
        let (_out, halt, cell) = step(&[
            ("value", value),
            ("sum", sum2),
            ("count", count2),
            ("sum_abs_dev", sum_abs_dev2),
        ]);
        assert_eq!(halt, cell80::Halt::Returned);
        sum2 = cell.get("sum").unwrap();
        count2 = cell.get("count").unwrap();
        sum_abs_dev2 = cell.get("sum_abs_dev").unwrap();
        assert_eq!(sum_abs_dev2, 0);
    }

    // Escalation on sum_abs_dev overflow specifically (the running sum itself does not
    // overflow): value=100, sum=0, count=1 -> new_sum=100, new_count=2, mean=50,
    // abs_dev=|100-50|=50. sum_abs_dev seeded at u32::MAX-5=4294967290; +50=4294967340,
    // which exceeds u32::MAX -> halt 0xFF05 (needs_wider_math).
    let (_result, halt, _cell) = step(&[
        ("value", 100u64),
        ("sum", 0u64),
        ("count", 1u64),
        ("sum_abs_dev", (u32::MAX - 5) as u64),
    ]);
    assert_eq!(halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn rising_streak_step_counts_strictly_increasing_runs() {
    // rising_streak_step: increments while the new value is strictly greater than the
    // immediately preceding one, resets to 0 otherwise (and self-initializes on the
    // first call via `seen`, matching running_min_max_step's first-call convention).
    fn step(fields: &[(&str, u64)]) -> (u16, StateCell) {
        let mut cell = StateCell::bind(&cell_src("rising_streak_step"), "RisingStreak", None)
            .unwrap_or_else(|e| panic!("bind: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let result = cell.run(DEFAULT_CYCLES).unwrap().result;
        (result, cell)
    }

    // Stream: 5, 8, 3, 3, 10, 20, 1
    //  5  -> first call (seen=0)      -> streak resets to 0
    //  8  -> 8 > 5                    -> streak = 1
    //  3  -> 3 > 8 is false           -> streak resets to 0
    //  3  -> 3 > 3 is false (strict)  -> streak stays 0 (plateau does not extend the streak)
    //  10 -> 10 > 3                   -> streak = 1
    //  20 -> 20 > 10                  -> streak = 2
    //  1  -> 1 > 20 is false          -> streak resets to 0
    let (mut prev, mut streak, mut seen) = (0u64, 0u64, 0u64);
    for (value, expect) in [
        (5u64, 0u64),
        (8, 1),
        (3, 0),
        (3, 0),
        (10, 1),
        (20, 2),
        (1, 0),
    ] {
        let (out, cell) = step(&[
            ("value", value),
            ("prev", prev),
            ("streak", streak),
            ("seen", seen),
        ]);
        assert_eq!(
            out as u64, expect,
            "value={value} expected={expect} got={out}"
        );
        prev = cell.get("prev").unwrap();
        streak = cell.get("streak").unwrap();
        seen = cell.get("seen").unwrap();
    }
}

// falling_streak_step: counts consecutive strictly-decreasing values, self-initializing on the
// first call (`seen` starts at 0) the same way running_min_max_step does — the strictly-decreasing
// direct complement of rising_streak_step. Sequence 10, 8, 8, 3, 1 exercises: first-call init
// (streak stays 0, no comparison made yet), a genuine decrease (streak grows), a tie (resets to 0,
// since a tie is not a *strict* decrease), then a fresh decreasing run building back up.
#[test]
fn falling_streak_step_counts_strictly_decreasing_runs() {
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let result = cell.run(DEFAULT_CYCLES).unwrap().result;
        (result, cell)
    }

    let (mut prev, mut streak, mut seen) = (0u64, 0u64, 0u64);
    for (value, expect_streak) in [(10u64, 0u64), (8, 1), (8, 0), (3, 1), (1, 2)] {
        let (out, cell) = step(
            "falling_streak_step",
            "FallingStreak",
            &[
                ("value", value),
                ("prev", prev),
                ("streak", streak),
                ("seen", seen),
            ],
        );
        assert_eq!(out as u64, expect_streak);
        prev = cell.get("prev").unwrap();
        streak = cell.get("streak").unwrap();
        seen = cell.get("seen").unwrap();
    }
    assert_eq!((prev, seen), (1, 1));
}

#[test]
fn running_covariance_step_i16_matches_hand_computed_values() {
    // running_covariance_step_i16: signed sibling of running_covariance_step -- accumulates
    // count, sum_x, sum_y, sum_xy one signed (x,y) pair per call, each sum tracked as a
    // (magnitude, sign) pair since x/y can now go negative.
    fn i16_bits(v: i16) -> u64 {
        (v as u16) as u64
    }
    fn step(fields: &[(&str, u64)]) -> (u16, cell80::Report, StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("running_covariance_step_i16"),
            "RunningCovarianceI16",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        let result = report.result;
        (result, report, cell)
    }
    // Decode a (mag, neg) field pair back to a signed i64 for readable assertions.
    let signed = |mag: u64, neg: u64| if neg == 1 { -(mag as i64) } else { mag as i64 };

    // Stream of signed pairs (2,3), (-4,5), (6,-1). Hand-computed running sums (plain
    // signed arithmetic, cross-checked against the sign-magnitude update rules):
    //   call1: xy=6;   sum_x=2,  sum_y=3, sum_xy=6,   count=1
    //   call2: xy=-20; sum_x=-2, sum_y=8, sum_xy=-14, count=2
    //   call3: xy=-6;  sum_x=4,  sum_y=7, sum_xy=-20, count=3
    let (mut count, mut sxm, mut sxn, mut sym, mut syn, mut sxym, mut sxyn) =
        (0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
    let mut results = Vec::new();
    for (x, y) in [(2i16, 3i16), (-4, 5), (6, -1)] {
        let (out, report, cell) = step(&[
            ("x", i16_bits(x)),
            ("y", i16_bits(y)),
            ("count", count),
            ("sum_x_mag", sxm),
            ("sum_x_neg", sxn),
            ("sum_y_mag", sym),
            ("sum_y_neg", syn),
            ("sum_xy_mag", sxym),
            ("sum_xy_neg", sxyn),
        ]);
        assert_eq!(report.halt, cell80::Halt::Returned);
        count = cell.get("count").unwrap();
        sxm = cell.get("sum_x_mag").unwrap();
        sxn = cell.get("sum_x_neg").unwrap();
        sym = cell.get("sum_y_mag").unwrap();
        syn = cell.get("sum_y_neg").unwrap();
        sxym = cell.get("sum_xy_mag").unwrap();
        sxyn = cell.get("sum_xy_neg").unwrap();
        results.push(out);
    }
    assert_eq!(results, vec![1, 1, 1]);
    assert_eq!(count, 3);
    assert_eq!(signed(sxm, sxn), 4);
    assert_eq!(signed(sym, syn), 7);
    assert_eq!(signed(sxym, sxyn), -20);

    // Opposite-sign tie forces neg back to 0 rather than carrying the previous sign
    // forward on a zero result: sum_x=5 (pos) + x=-5 (neg) -> mag 0, neg must be 0.
    let (_, report, cell) = step(&[
        ("x", i16_bits(-5)),
        ("y", i16_bits(0)),
        ("count", 1),
        ("sum_x_mag", 5),
        ("sum_x_neg", 0),
        ("sum_y_mag", 0),
        ("sum_y_neg", 0),
        ("sum_xy_mag", 0),
        ("sum_xy_neg", 0),
    ]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("sum_x_mag").unwrap(), 0);
    assert_eq!(cell.get("sum_x_neg").unwrap(), 0); // a zero result is never "negative"

    // sum_x_mag overflow escalation: seed sum_x_mag = u32::MAX - 5 (positive), add x=10
    // (positive, same sign) -> add_checked_u32 overflows u32 -> halt 0xFF05.
    let (_, report, _) = step(&[
        ("x", i16_bits(10)),
        ("y", i16_bits(0)),
        ("count", 0),
        ("sum_x_mag", (u32::MAX - 5) as u64),
        ("sum_x_neg", 0),
        ("sum_y_mag", 0),
        ("sum_y_neg", 0),
        ("sum_xy_mag", 0),
        ("sum_xy_neg", 0),
    ]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}
