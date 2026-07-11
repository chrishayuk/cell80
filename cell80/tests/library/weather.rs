//! Host-oracle tests for the weather pack (`cell80/cells/weather/*.rs`) — mechanically
//! generated from each cell's own proposed test cases (the first weather-pack batch:
//! `heat_index_f32`, `wind_chill_f32`, `dew_point_approx_f32`, `gust_factor_f32`,
//! `temperature_trend_step` — verify->admit loop; a sixth proposed cell, `rain_rate_f32`,
//! was backed out by the admission gate as a behavioural duplicate of `gust_factor_f32` —
//! both are literally `a / b` with a domain guard on the denominator, so no distinct
//! behaviour survives the rename) rather than
//! hand-transcribed: every `test_cases` entry from the authoring output becomes one
//! comparison, run against the real compiled cell via `StateCell::bind`/`set`/`run`/`get`
//! (see `cell80/tests/library/common.rs` for the shared `cell_src` helper, and
//! `physics.rs` for the `run_f32`/`halt_of`/`get_f32` helper shapes this file reuses,
//! since every weather cell so far is f32-typed exactly like the physics pack). f32
//! fields ride raw bit patterns (`to_bits`/`from_bits`) and compare with a small
//! relative-tolerance epsilon rather than bit-exactness, since these expected values are
//! hand-derived arithmetic (independently re-verified against the compiled cell here, not
//! just trusted from the authoring notes), matching the convention already established in
//! `excel-mathstat.rs`/`numerical-primitives.rs`. `wind_chill_f32` composes with the
//! already-shipped `nth_root_f32` for its V^0.16 term: it takes the already-computed 25th
//! root as a plain input field (`v_pow4_25th_root`) rather than attempting a fractional-
//! power loop itself, so its test cases supply that field directly instead of running
//! `nth_root_f32` first. `gust_factor_f32`'s domain-guard case (a non-positive
//! `mean_wind_speed`) is checked as a typed escalation (`Halt::Escalate(0xFF06)`,
//! `physics.rs`'s own convention for a halt-shaped test case) rather than a field
//! comparison.
//
// Mechanically generated scaffolds: a flat per-cell case table. Style lints the
// generator would re-trip next wave are allowed rather than hand-patched.
#![allow(clippy::excessive_precision, clippy::approx_constant)]

use crate::common::cell_src;
use cell80::{Halt, StateCell, DEFAULT_CYCLES};

fn run_f32(id: &str, state: &str, fields: &[(&str, f32)]) -> StateCell {
    let mut cell =
        StateCell::bind(&cell_src(id), state, None).unwrap_or_else(|e| panic!("{id}: {e}"));
    for (name, v) in fields {
        cell.set(name, v.to_bits() as u64)
            .unwrap_or_else(|e| panic!("{id}.{name}: {e}"));
    }
    let r = cell
        .run(DEFAULT_CYCLES)
        .unwrap_or_else(|e| panic!("{id}: {e}"));
    assert_eq!(r.halt, Halt::Returned, "{id}");
    cell
}

fn halt_of(id: &str, state: &str, fields: &[(&str, f32)]) -> Halt {
    let mut cell =
        StateCell::bind(&cell_src(id), state, None).unwrap_or_else(|e| panic!("{id}: {e}"));
    for (name, v) in fields {
        cell.set(name, v.to_bits() as u64).unwrap();
    }
    cell.run(DEFAULT_CYCLES).unwrap().halt
}

fn get_f32(cell: &StateCell, field: &str) -> f32 {
    f32::from_bits(cell.get(field).expect(field) as u32)
}

fn f32_tol(got: f32, want: f32) -> bool {
    (got - want).abs() < (want.abs() * 1e-3_f32).max(1e-3_f32)
}

#[test]
fn heat_index_f32_matches_test_cases() {
    let cases: &[((f32, f32), f32)] = &[
        ((100.0, 55.0), 123.638214),
        ((90.0, 50.0), 94.597015),
        ((80.0, 40.0), 79.929413),
        ((110.0, 60.0), 171.244919),
    ];
    for (i, ((t, rh), want)) in cases.iter().enumerate() {
        let cell = run_f32("heat_index_f32", "HeatIndexF32", &[("t", *t), ("rh", *rh)]);
        let got = get_f32(&cell, "hi");
        assert!(
            f32_tol(got, *want),
            "heat_index_f32 case {i}: got {got} want {want}"
        );
    }
}

#[test]
fn wind_chill_f32_matches_test_cases() {
    let cases: &[((f32, f32, f32), f32)] = &[
        ((5.0, 15.0, 1.5423203706741333), -12.993741035461426),
        ((-10.0, 25.0, 1.6736721992492676), -37.46372985839844),
        ((50.0, 3.0, 1.1921733617782593), 49.677513122558594),
        ((0.0, 15.0, 1.5423203706741333), -19.397953033447266),
    ];
    for (i, ((t, v, root), want)) in cases.iter().enumerate() {
        let cell = run_f32(
            "wind_chill_f32",
            "WindChillF32",
            &[("t", *t), ("v", *v), ("v_pow4_25th_root", *root)],
        );
        let got = get_f32(&cell, "wc");
        assert!(
            f32_tol(got, *want),
            "wind_chill_f32 case {i}: got {got} want {want}"
        );
    }
}

#[test]
fn dew_point_approx_f32_matches_test_cases() {
    let cases: &[((f32, f32), f32)] = &[
        ((20.0, 50.0), 10.0),
        ((30.0, 80.0), 26.0),
        ((-5.0, 90.0), -7.0),
        ((15.0, 100.0), 15.0),
        ((25.0, 0.0), 5.0),
        ((18.0, 73.0), 12.600000381469727),
    ];
    for (i, ((temp_c, rh_pct), want)) in cases.iter().enumerate() {
        let cell = run_f32(
            "dew_point_approx_f32",
            "DewPointApproxF32",
            &[("temp_c", *temp_c), ("rh_pct", *rh_pct)],
        );
        let got = get_f32(&cell, "dew_point_c");
        assert!(
            f32_tol(got, *want),
            "dew_point_approx_f32 case {i}: got {got} want {want}"
        );
    }
}

#[test]
fn gust_factor_f32_matches_test_cases() {
    let cases: &[((f32, f32), f32)] = &[
        ((15.0, 10.0), 1.5),
        ((22.0, 20.0), 1.1),
        ((8.0, 8.0), 1.0),
        ((9.0, 10.0), 0.9),
    ];
    for (i, ((peak, mean), want)) in cases.iter().enumerate() {
        let cell = run_f32(
            "gust_factor_f32",
            "GustFactor",
            &[("peak_gust_speed", *peak), ("mean_wind_speed", *mean)],
        );
        let got = get_f32(&cell, "gust_factor");
        assert!(
            f32_tol(got, *want),
            "gust_factor_f32 case {i}: got {got} want {want}"
        );
    }
}

/// The 5th proposed test case (`peak_gust_speed=10, mean_wind_speed=0`) exercises the
/// domain guard, not a field value — the authoring notes flagged the schema mismatch
/// (`expected_field: "halt"`) explicitly; checked here as a typed escalation instead,
/// `physics.rs`'s own convention (`Halt::Escalate(0xFF06)` = `out_of_domain`).
#[test]
fn gust_factor_f32_escalates_on_non_positive_mean_wind() {
    assert_eq!(
        halt_of(
            "gust_factor_f32",
            "GustFactor",
            &[("peak_gust_speed", 10.0), ("mean_wind_speed", 0.0)],
        ),
        Halt::Escalate(0xFF06),
        "zero mean_wind_speed must escalate out_of_domain, not divide"
    );
}

#[test]
fn temperature_trend_step_matches_test_cases() {
    let cases: &[((f32, f32, f32), u16)] = &[
        ((25.0, 20.0, 2.0), 2),
        ((15.0, 20.0, 2.0), 0),
        ((21.5, 20.0, 2.0), 1),
        ((22.0, 20.0, 2.0), 1),
        ((18.0, 20.0, 2.0), 1),
        ((99.3, 98.6, 0.5), 2),
        ((-8.0, -5.0, 1.0), 0),
        ((20.0, 20.0, 0.0), 1),
        ((20.0001, 20.0, 0.0), 2),
    ];
    for (i, ((current, previous, threshold), want)) in cases.iter().enumerate() {
        let cell = run_f32(
            "temperature_trend_step",
            "TemperatureTrendStep",
            &[
                ("current_reading", *current),
                ("previous_reading", *previous),
                ("threshold", *threshold),
            ],
        );
        let got = cell.get("trend").expect("trend") as u16;
        assert_eq!(
            got, *want,
            "temperature_trend_step case {i}: got {got} want {want}"
        );
    }
}
