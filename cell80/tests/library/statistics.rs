//! Host-oracle tests for the statistics pack (`cell80/cells/statistics/*.rs`). Mirrors the
//! cells' own pack-directory structure; see `cell80/tests/library/common.rs` for the
//! shared `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::cell_src;
use cell80::{StateCell, DEFAULT_CYCLES};

#[test]
fn wave13_statistics_from_precomputed_sums_match_defined_behaviour() {
    // Wave 13 (docs/math-server-map.md's statistics.descriptive category, the
    // "given precomputed sums" slice -- raw-dataset aggregation stays upstream).
    // Every expected fraction was cross-checked against an independent Python
    // reference implementation (including a fractions.Fraction sanity check)
    // before transcription.
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // covariance: x=[1,2,3], y=[2,4,6] (y=2x) -- n=3, sum_x=6, sum_y=12, sum_xy=28.
    // cov = (3*28 - 6*12)/9 = (84-72)/9 = 12/9 = 4/3.
    let (_, cell) = step(
        "covariance",
        "Covariance",
        &[("n", 3), ("sum_x", 6), ("sum_y", 12), ("sum_xy", 28)],
    );
    assert_eq!(
        (cell.get("num_mag"), cell.get("num_neg"), cell.get("den")),
        (Some(12), Some(0), Some(9))
    );

    // Inversely related: x=[1,2,3], y=[6,4,2] -- sum_x=6, sum_y=12, sum_xy=1*6+2*4+3*2=20.
    // cov = (3*20 - 6*12)/9 = (60-72)/9 = -12/9.
    let (_, cell) = step(
        "covariance",
        "Covariance",
        &[("n", 3), ("sum_x", 6), ("sum_y", 12), ("sum_xy", 20)],
    );
    assert_eq!(
        (cell.get("num_mag"), cell.get("num_neg"), cell.get("den")),
        (Some(12), Some(1), Some(9))
    );

    let (report, _) = step(
        "covariance",
        "Covariance",
        &[("n", 0), ("sum_x", 0), ("sum_y", 0), ("sum_xy", 0)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // linear_regression_slope: y=2x over x=[1,2,3] -- sum_x2=1+4+9=14.
    // slope = (3*28 - 6*12)/(3*14 - 36) = 12/6 = 2/1.
    let (_, cell) = step(
        "linear_regression_slope",
        "LinearRegressionSlope",
        &[
            ("n", 3),
            ("sum_x", 6),
            ("sum_y", 12),
            ("sum_xy", 28),
            ("sum_x2", 14),
        ],
    );
    assert_eq!(
        (cell.get("num_mag"), cell.get("num_neg"), cell.get("den")),
        (Some(12), Some(0), Some(6))
    );

    // y=2x+1 over x=[1,2,3,4]: sum_x=10, sum_y=24, sum_xy=70, sum_x2=30.
    // slope = (4*70 - 10*24)/(4*30 - 100) = (280-240)/(120-100) = 40/20 = 2/1.
    let (_, cell) = step(
        "linear_regression_slope",
        "LinearRegressionSlope",
        &[
            ("n", 4),
            ("sum_x", 10),
            ("sum_y", 24),
            ("sum_xy", 70),
            ("sum_x2", 30),
        ],
    );
    assert_eq!(
        (cell.get("num_mag"), cell.get("num_neg"), cell.get("den")),
        (Some(40), Some(0), Some(20))
    );

    // Vertical line: every x identical (x=[3,3,3]) -- denominator vanishes.
    let (report, _) = step(
        "linear_regression_slope",
        "LinearRegressionSlope",
        &[
            ("n", 3),
            ("sum_x", 9),
            ("sum_y", 12),
            ("sum_xy", 36),
            ("sum_x2", 27),
        ],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
}

#[test]
fn wave14_q8_statistics_match_defined_behaviour() {
    // Wave 14: correlation and effect_size_r, the last two candidates from
    // docs/math-server-map.md's original 77-candidate list. Both compute an integer
    // square root at a 256x-scaled precision (sqrt first, divide last, rather than
    // dividing by a truncated integer sqrt directly) -- verified against an
    // independent Python reference over thousands of random cases, checking both
    // exactness of known perfect-correlation cases and worst-case Q8.8 error on
    // noisy/realistic data, before transcribing any row here.
    fn i16_bits(v: i16) -> u64 {
        (v as u16) as u64
    }
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // effect_size_r: r = t / sqrt(t^2 + df), bounded to [-1, 1].
    let (_, cell) = step(
        "effect_size_r",
        "EffectSizeR",
        &[("t", i16_bits(2)), ("df", 8)],
    );
    assert_eq!((cell.get("r_mag"), cell.get("r_neg")), (Some(148), Some(0))); // true r*256 = 147.8

    let (_, cell) = step(
        "effect_size_r",
        "EffectSizeR",
        &[("t", i16_bits(-3)), ("df", 16)],
    );
    assert_eq!((cell.get("r_mag"), cell.get("r_neg")), (Some(153), Some(1))); // true |r|*256 = 153.6

    let (_, cell) = step(
        "effect_size_r",
        "EffectSizeR",
        &[("t", i16_bits(0)), ("df", 10)],
    );
    assert_eq!((cell.get("r_mag"), cell.get("r_neg")), (Some(0), Some(0)));

    let (_, cell) = step(
        "effect_size_r",
        "EffectSizeR",
        &[("t", i16_bits(50)), ("df", 0)],
    );
    assert_eq!((cell.get("r_mag"), cell.get("r_neg")), (Some(256), Some(0))); // df=0 -> r = t/|t| = 1 exactly

    // t at i16::MAX with df=0: t^2 fits u32, but the 256x-scaled magnitude doesn't.
    let (report, _) = step(
        "effect_size_r",
        "EffectSizeR",
        &[("t", i16_bits(i16::MAX)), ("df", 0)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // correlation: x=[1,2,3], y=[2,4,6] (y=2x exactly) -> r=1.
    let (_, cell) = step(
        "correlation",
        "Correlation",
        &[
            ("n", 3),
            ("sum_x", 6),
            ("sum_y", 12),
            ("sum_xy", 28),
            ("sum_x2", 14),
            ("sum_y2", 56),
        ],
    );
    assert_eq!((cell.get("r_mag"), cell.get("r_neg")), (Some(256), Some(0)));

    // x=[1,2,3], y=[6,4,2] (y=-2x+8, perfectly inversely related) -> r=-1.
    let (_, cell) = step(
        "correlation",
        "Correlation",
        &[
            ("n", 3),
            ("sum_x", 6),
            ("sum_y", 12),
            ("sum_xy", 20),
            ("sum_x2", 14),
            ("sum_y2", 56),
        ],
    );
    assert_eq!((cell.get("r_mag"), cell.get("r_neg")), (Some(256), Some(1)));

    // x=[1,2,3,4,5], y=[2,1,4,3,5] (noisy) -> true r*256 = 204.8.
    let (_, cell) = step(
        "correlation",
        "Correlation",
        &[
            ("n", 5),
            ("sum_x", 15),
            ("sum_y", 15),
            ("sum_xy", 53),
            ("sum_x2", 55),
            ("sum_y2", 55),
        ],
    );
    assert_eq!((cell.get("r_mag"), cell.get("r_neg")), (Some(204), Some(0)));

    // Zero variance in x (x=[3,3,3]) -> correlation undefined.
    let (report, _) = step(
        "correlation",
        "Correlation",
        &[
            ("n", 3),
            ("sum_x", 9),
            ("sum_y", 6),
            ("sum_xy", 18),
            ("sum_x2", 27),
            ("sum_y2", 14),
        ],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    let (report, _) = step(
        "correlation",
        "Correlation",
        &[
            ("n", 0),
            ("sum_x", 0),
            ("sum_y", 0),
            ("sum_xy", 0),
            ("sum_x2", 0),
            ("sum_y2", 0),
        ],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
}
