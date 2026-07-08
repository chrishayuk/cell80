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
