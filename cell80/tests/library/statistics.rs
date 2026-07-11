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

#[test]
fn linear_regression_intercept_matches_hand_computed_ols_fits() {
    // Checks linear_regression_intercept against four hand-computed OLS fits and
    // the two documented halt paths, using the exact five precomputed sums
    // (n, sum_x, sum_y, sum_xy, sum_x2) linear_regression_slope also consumes.
    fn step(fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("linear_regression_intercept"),
            "LinearRegressionIntercept",
            None,
        )
        .unwrap_or_else(|e| panic!("bind linear_regression_intercept: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // y=2x over x=[1,2,3] passes through the origin -- n=3, sum_x=6, sum_y=12, sum_xy=28, sum_x2=14.
    // b = (12*14 - 6*28)/(3*14-36) = (168-168)/6 = 0/6.
    let (_, cell) = step(&[
        ("n", 3),
        ("sum_x", 6),
        ("sum_y", 12),
        ("sum_xy", 28),
        ("sum_x2", 14),
    ]);
    assert_eq!(
        (cell.get("num_mag"), cell.get("num_neg"), cell.get("den")),
        (Some(0), Some(0), Some(6))
    );

    // y=2x+1 over x=[1,2,3,4] -- n=4, sum_x=10, sum_y=24, sum_xy=70, sum_x2=30.
    // b = (24*30 - 10*70)/(4*30-100) = (720-700)/20 = 20/20 = 1.
    let (_, cell) = step(&[
        ("n", 4),
        ("sum_x", 10),
        ("sum_y", 24),
        ("sum_xy", 70),
        ("sum_x2", 30),
    ]);
    assert_eq!(
        (cell.get("num_mag"), cell.get("num_neg"), cell.get("den")),
        (Some(20), Some(0), Some(20))
    );

    // y=2x-3 over x=[1..5] gives y=[-1,1,3,5,7] -- n=5, sum_x=15, sum_y=15, sum_xy=65, sum_x2=55.
    // b = (15*55 - 15*65)/(5*55-225) = (825-975)/50 = -150/50 = -3.
    let (_, cell) = step(&[
        ("n", 5),
        ("sum_x", 15),
        ("sum_y", 15),
        ("sum_xy", 65),
        ("sum_x2", 55),
    ]);
    assert_eq!(
        (cell.get("num_mag"), cell.get("num_neg"), cell.get("den")),
        (Some(150), Some(1), Some(50))
    );

    // Vertical line, x=[3,3,3] -- den = 3*27 - 81 = 0, undefined intercept.
    let (report, _) = step(&[
        ("n", 3),
        ("sum_x", 9),
        ("sum_y", 12),
        ("sum_xy", 36),
        ("sum_x2", 27),
    ]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // n=0 halts immediately, same as linear_regression_slope's own guard.
    let (report, _) = step(&[
        ("n", 0),
        ("sum_x", 0),
        ("sum_y", 0),
        ("sum_xy", 0),
        ("sum_x2", 0),
    ]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
}

#[test]
fn variance_from_sums_matches_hand_computed_expectations() {
    // variance_from_sums: population variance from precomputed sums (n, sum_x, sum_x2),
    // var = (n*sum_x2 - sum_x^2)/n^2 as an exact fraction num/den. Every expected
    // fraction below was hand-computed against the definition before running.
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // x = [1,2,3] -> n=3, sum_x=6, sum_x2=1+4+9=14.
    // var = (3*14 - 6^2)/3^2 = (42-36)/9 = 6/9.
    let (_, cell) = step(
        "variance_from_sums",
        "VarianceFromSums",
        &[("n", 3), ("sum_x", 6), ("sum_x2", 14)],
    );
    assert_eq!((cell.get("num"), cell.get("den")), (Some(6), Some(9)));

    // x = [2,2,2] -> no spread -> n=3, sum_x=6, sum_x2=12. var = (36-36)/9 = 0/9.
    let (_, cell) = step(
        "variance_from_sums",
        "VarianceFromSums",
        &[("n", 3), ("sum_x", 6), ("sum_x2", 12)],
    );
    assert_eq!((cell.get("num"), cell.get("den")), (Some(0), Some(9)));

    // x = [1,2,3,4,5] -> n=5, sum_x=15, sum_x2=55. var = (275-225)/25 = 50/25 (=2).
    let (_, cell) = step(
        "variance_from_sums",
        "VarianceFromSums",
        &[("n", 5), ("sum_x", 15), ("sum_x2", 55)],
    );
    assert_eq!((cell.get("num"), cell.get("den")), (Some(50), Some(25)));

    // n == 0 -> escalate out_of_domain.
    let (report, _) = step(
        "variance_from_sums",
        "VarianceFromSums",
        &[("n", 0), ("sum_x", 0), ("sum_x2", 0)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // Inconsistent sums (n=1, sum_x=5, sum_x2=1) -- n*sum_x2 < sum_x^2 is
    // mathematically impossible for a real dataset, so this signals corrupted
    // input and must escalate needs_wider_math rather than silently underflow.
    let (report, _) = step(
        "variance_from_sums",
        "VarianceFromSums",
        &[("n", 1), ("sum_x", 5), ("sum_x2", 1)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn std_dev_from_sums_matches_hand_computed_expectations() {
    // std_dev_from_sums: population standard deviation from precomputed sums
    // (n, sum_x, sum_x2), stddev = floor(sqrt((n*sum_x2 - sum_x^2)/n^2)) -- the
    // sqrt-taking completion of variance_from_sums's own num/den fraction. Every
    // expected value below was hand-computed against the definition before running.
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // x = [1,2,3,4,5] -> n=5, sum_x=15, sum_x2=55.
    // variance = (5*55 - 15^2)/5^2 = (275-225)/25 = 50/25 = 2 exactly.
    // stddev = floor(sqrt(2)) = 1.
    let (_, cell) = step(
        "std_dev_from_sums",
        "StdDevFromSums",
        &[("n", 5), ("sum_x", 15), ("sum_x2", 55)],
    );
    assert_eq!(cell.get("stddev"), Some(1));

    // x = [2,4,4,4,5,5,7,9] -> n=8, sum_x=40, sum_x2=232.
    // variance = (8*232 - 40^2)/8^2 = (1856-1600)/64 = 256/64 = 4 exactly.
    // stddev = floor(sqrt(4)) = 2.
    let (_, cell) = step(
        "std_dev_from_sums",
        "StdDevFromSums",
        &[("n", 8), ("sum_x", 40), ("sum_x2", 232)],
    );
    assert_eq!(cell.get("stddev"), Some(2));

    // x = [1,2,3] -> n=3, sum_x=6, sum_x2=14.
    // variance = (3*14 - 6^2)/3^2 = 6/9 -> truncates to 0 under integer division
    // (matches variance_from_sums's own num=6, den=9 case) -> stddev = floor(sqrt(0)) = 0.
    let (_, cell) = step(
        "std_dev_from_sums",
        "StdDevFromSums",
        &[("n", 3), ("sum_x", 6), ("sum_x2", 14)],
    );
    assert_eq!(cell.get("stddev"), Some(0));

    // n == 0 -> escalate out_of_domain, same guard as variance_from_sums.
    let (report, _) = step(
        "std_dev_from_sums",
        "StdDevFromSums",
        &[("n", 0), ("sum_x", 0), ("sum_x2", 0)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // Inconsistent sums (n=1, sum_x=5, sum_x2=1): n*sum_x2 (=1) < sum_x^2 (=25) is
    // impossible for a real dataset -> escalate needs_wider_math rather than silently
    // underflow, same as variance_from_sums's own corrupted-input case.
    let (report, _) = step(
        "std_dev_from_sums",
        "StdDevFromSums",
        &[("n", 1), ("sum_x", 5), ("sum_x2", 1)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn sample_variance_from_sums_matches_hand_computed_expectations() {
    // sample_variance_from_sums: unbiased (Bessel-corrected) sample variance from
    // precomputed sums (n, sum_x, sum_x2), var = (n*sum_x2 - sum_x^2)/(n*(n-1)) as
    // an exact fraction num/den. Every expected fraction below was hand-computed
    // against the definition before running.
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // x = [1,2,3] -> n=3, sum_x=6, sum_x2=14.
    // num = 3*14 - 6^2 = 42-36 = 6; den = 3*2 = 6; sample var = 6/6 = 1.
    let (_, cell) = step(
        "sample_variance_from_sums",
        "SampleVarianceFromSums",
        &[("n", 3), ("sum_x", 6), ("sum_x2", 14)],
    );
    assert_eq!((cell.get("num"), cell.get("den")), (Some(6), Some(6)));

    // x = [2,2,2] -> no spread -> n=3, sum_x=6, sum_x2=12. num=0, den=6.
    let (_, cell) = step(
        "sample_variance_from_sums",
        "SampleVarianceFromSums",
        &[("n", 3), ("sum_x", 6), ("sum_x2", 12)],
    );
    assert_eq!((cell.get("num"), cell.get("den")), (Some(0), Some(6)));

    // x = [1,2,3,4,5] -> n=5, sum_x=15, sum_x2=55.
    // num = 5*55 - 225 = 50; den = 5*4 = 20; sample var = 50/20 = 2.5.
    let (_, cell) = step(
        "sample_variance_from_sums",
        "SampleVarianceFromSums",
        &[("n", 5), ("sum_x", 15), ("sum_x2", 55)],
    );
    assert_eq!((cell.get("num"), cell.get("den")), (Some(50), Some(20)));

    // n == 1 -> sample variance undefined with fewer than two observations.
    let (report, _) = step(
        "sample_variance_from_sums",
        "SampleVarianceFromSums",
        &[("n", 1), ("sum_x", 5), ("sum_x2", 25)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // n == 0 -> escalate out_of_domain.
    let (report, _) = step(
        "sample_variance_from_sums",
        "SampleVarianceFromSums",
        &[("n", 0), ("sum_x", 0), ("sum_x2", 0)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // Inconsistent sums (n=2, sum_x=10, sum_x2=1) -- n*sum_x2 < sum_x^2 is
    // mathematically impossible for a real dataset, so this signals corrupted
    // input and must escalate needs_wider_math rather than silently underflow.
    let (report, _) = step(
        "sample_variance_from_sums",
        "SampleVarianceFromSums",
        &[("n", 2), ("sum_x", 10), ("sum_x2", 1)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn sample_covariance_from_sums_matches_hand_computed() {
    // Same host-oracle pattern as the pack's other precomputed-sums cells: bind the
    // state cell, set fields, run, and check the exact signed-fraction outputs.
    fn verify(fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("sample_covariance_from_sums"),
            "SampleCovarianceFromSums",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // x=[1,2,3], y=[2,4,6] (y=2x, perfectly correlated). n=3, sum_x=6, sum_y=12, sum_xy=28.
    // cov = (3*28 - 6*12) / (3*2) = (84-72)/6 = 12/6 = 2 (vs. population's 12/9 = 4/3 --
    // the n/(n-1) Bessel correction scales it up by 3/2).
    let (_, c) = verify(&[("n", 3), ("sum_x", 6), ("sum_y", 12), ("sum_xy", 28)]);
    assert_eq!(
        (c.get("num_mag"), c.get("num_neg"), c.get("den")),
        (Some(12), Some(0), Some(6))
    );

    // Inversely related: x=[1,2,3], y=[6,4,2] -- sum_xy=1*6+2*4+3*2=20.
    // cov = (3*20 - 6*12)/6 = (60-72)/6 => negative, so num_neg flips to 1.
    let (_, c) = verify(&[("n", 3), ("sum_x", 6), ("sum_y", 12), ("sum_xy", 20)]);
    assert_eq!(
        (c.get("num_mag"), c.get("num_neg"), c.get("den")),
        (Some(12), Some(1), Some(6))
    );

    // Minimal valid n=2: x=[1,2], y=[1,2] -- sum_x=3, sum_y=3, sum_xy=5.
    // cov = (2*5 - 3*3) / (2*1) = 1/2.
    let (_, c) = verify(&[("n", 2), ("sum_x", 3), ("sum_y", 3), ("sum_xy", 5)]);
    assert_eq!(
        (c.get("num_mag"), c.get("num_neg"), c.get("den")),
        (Some(1), Some(0), Some(2))
    );

    // n < 2 is undefined for a sample covariance (Bessel's n-1 denominator) --
    // escalates out_of_domain regardless of the other sums.
    let (report, _) = verify(&[("n", 1), ("sum_x", 5), ("sum_y", 5), ("sum_xy", 25)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
    let (report, _) = verify(&[("n", 0), ("sum_x", 0), ("sum_y", 0), ("sum_xy", 0)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
}
