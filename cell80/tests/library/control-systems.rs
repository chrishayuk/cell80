//! Host-oracle tests for the control-systems pack (`cell80/cells/control-systems/*.rs`) —
//! mechanically generated (state cells) plus hand-written (free functions) from each cell's
//! own proposed test cases (the 22-cell excel-datetime/control-systems/numerical-primitives
//! batch, verify->admit loop). State cells run through `StateCell::bind`/`set`/`run`/`get`
//! (see `cell80/tests/library/common.rs`'s shared `cell_src` helper); the 3 free functions
//! (`slew_rate_limiter_step`, `deadband`, `bang_bang_controller`) run through `run_cell`'s
//! raw-register interface instead, matching `cell80/tests/library/signed-deltas.rs`'s own
//! convention for i16 params/returns — passed/read as their two's-complement `u16` bit
//! pattern (`-5` <-> `65531`). f32 fields ride raw bit patterns (`to_bits`/`from_bits`, the
//! physics/softfloat packs' own convention) and compare with a small epsilon tolerance
//! rather than bit-exactness, since these expected values are hand-derived arithmetic, not
//! a host-rustc oracle.
//
// Mechanically generated scaffolds: single-type cells degenerate to `match name
// { _ => .. }` and every case table shares one tuple shape — style lints the
// generator would re-trip next wave are allowed rather than hand-patched.
#![allow(clippy::match_single_binding, clippy::type_complexity)]

#[test]
fn pid_step_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "dt" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "error" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "integral" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "kd" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "ki" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "kp" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "output" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "prev_error" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "dt" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "error" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "integral" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "kd" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "ki" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "kp" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "output" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "prev_error" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("error", 4.0),
                ("dt", 1.0),
                ("kp", 2.0),
                ("ki", 0.5),
                ("kd", 0.25),
                ("integral", 0.0),
                ("prev_error", 3.0),
            ],
            "output",
            10.25,
        ),
        (
            &[
                ("error", 4.0),
                ("dt", 1.0),
                ("kp", 2.0),
                ("ki", 0.5),
                ("kd", 0.25),
                ("integral", 0.0),
                ("prev_error", 3.0),
            ],
            "integral",
            4.0,
        ),
        (
            &[
                ("error", 4.0),
                ("dt", 1.0),
                ("kp", 2.0),
                ("ki", 0.5),
                ("kd", 0.25),
                ("integral", 0.0),
                ("prev_error", 3.0),
            ],
            "prev_error",
            4.0,
        ),
        (
            &[
                ("error", 2.0),
                ("dt", 0.5),
                ("kp", 0.0),
                ("ki", 3.0),
                ("kd", 0.0),
                ("integral", 1.0),
                ("prev_error", 2.0),
            ],
            "output",
            6.0,
        ),
        (
            &[
                ("error", 1.0),
                ("dt", 2.0),
                ("kp", 0.0),
                ("ki", 0.0),
                ("kd", 4.0),
                ("integral", 0.0),
                ("prev_error", 5.0),
            ],
            "output",
            -8.0,
        ),
        (
            &[
                ("error", 10.0),
                ("dt", 1.0),
                ("kp", 1.0),
                ("ki", 1.0),
                ("kd", 1.0),
                ("integral", 0.0),
                ("prev_error", 10.0),
            ],
            "output",
            20.0,
        ),
        (
            &[
                ("error", 8.0),
                ("dt", 1.0),
                ("kp", 1.0),
                ("ki", 1.0),
                ("kd", 1.0),
                ("integral", 10.0),
                ("prev_error", 10.0),
            ],
            "output",
            24.0,
        ),
        (
            &[
                ("error", 8.0),
                ("dt", 1.0),
                ("kp", 1.0),
                ("ki", 1.0),
                ("kd", 1.0),
                ("integral", 10.0),
                ("prev_error", 10.0),
            ],
            "integral",
            18.0,
        ),
        (
            &[
                ("error", -3.0),
                ("dt", 1.0),
                ("kp", 2.0),
                ("ki", 1.0),
                ("kd", 1.0),
                ("integral", 5.0),
                ("prev_error", -1.0),
            ],
            "output",
            -6.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("pid_step"),
            "PidStep",
            None,
        )
        .unwrap_or_else(|e| panic!("bind pid_step: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run pid_step case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "pid_step case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "dt" | "error" | "integral" | "kd" | "ki" | "kp" | "output" | "prev_error");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "pid_step case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "pid_step case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn pid_step_antiwindup_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "dt" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "integral" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "integral_out" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "kd" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "ki" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "kp" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "measurement" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "out_max" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "out_min" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "output" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "prev_error" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "prev_error_out" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "setpoint" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "dt" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "integral" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "integral_out" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "kd" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "ki" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "kp" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "measurement" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "out_max" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "out_min" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "output" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "prev_error" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "prev_error_out" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "setpoint" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("setpoint", 10.0),
                ("measurement", 4.0),
                ("kp", 2.0),
                ("ki", 0.5),
                ("kd", 0.25),
                ("dt", 1.0),
                ("integral", 0.0),
                ("prev_error", 2.0),
                ("out_min", -100.0),
                ("out_max", 100.0),
            ],
            "output",
            16.0,
        ),
        (
            &[
                ("setpoint", 10.0),
                ("measurement", 4.0),
                ("kp", 2.0),
                ("ki", 0.5),
                ("kd", 0.25),
                ("dt", 1.0),
                ("integral", 0.0),
                ("prev_error", 2.0),
                ("out_min", -100.0),
                ("out_max", 100.0),
            ],
            "integral_out",
            6.0,
        ),
        (
            &[
                ("setpoint", 10.0),
                ("measurement", 4.0),
                ("kp", 2.0),
                ("ki", 0.5),
                ("kd", 0.25),
                ("dt", 1.0),
                ("integral", 0.0),
                ("prev_error", 2.0),
                ("out_min", -100.0),
                ("out_max", 100.0),
            ],
            "prev_error_out",
            6.0,
        ),
        (
            &[
                ("setpoint", 100.0),
                ("measurement", 0.0),
                ("kp", 2.0),
                ("ki", 1.0),
                ("kd", 0.5),
                ("dt", 1.0),
                ("integral", 10.0),
                ("prev_error", 50.0),
                ("out_min", 0.0),
                ("out_max", 50.0),
            ],
            "output",
            50.0,
        ),
        (
            &[
                ("setpoint", 100.0),
                ("measurement", 0.0),
                ("kp", 2.0),
                ("ki", 1.0),
                ("kd", 0.5),
                ("dt", 1.0),
                ("integral", 10.0),
                ("prev_error", 50.0),
                ("out_min", 0.0),
                ("out_max", 50.0),
            ],
            "integral_out",
            10.0,
        ),
        (
            &[
                ("setpoint", 100.0),
                ("measurement", 0.0),
                ("kp", 2.0),
                ("ki", 1.0),
                ("kd", 0.5),
                ("dt", 1.0),
                ("integral", 10.0),
                ("prev_error", 50.0),
                ("out_min", 0.0),
                ("out_max", 50.0),
            ],
            "prev_error_out",
            100.0,
        ),
        (
            &[
                ("setpoint", 0.0),
                ("measurement", 100.0),
                ("kp", 2.0),
                ("ki", 1.0),
                ("kd", 0.5),
                ("dt", 1.0),
                ("integral", -10.0),
                ("prev_error", -50.0),
                ("out_min", -50.0),
                ("out_max", 0.0),
            ],
            "output",
            -50.0,
        ),
        (
            &[
                ("setpoint", 0.0),
                ("measurement", 100.0),
                ("kp", 2.0),
                ("ki", 1.0),
                ("kd", 0.5),
                ("dt", 1.0),
                ("integral", -10.0),
                ("prev_error", -50.0),
                ("out_min", -50.0),
                ("out_max", 0.0),
            ],
            "integral_out",
            -10.0,
        ),
        (
            &[
                ("setpoint", 50.0),
                ("measurement", 0.0),
                ("kp", 1.0),
                ("ki", 0.0),
                ("kd", 0.0),
                ("dt", 1.0),
                ("integral", 0.0),
                ("prev_error", 50.0),
                ("out_min", -100.0),
                ("out_max", 50.0),
            ],
            "integral_out",
            50.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("pid_step_antiwindup"),
            "PidStepAntiwindup",
            None,
        )
        .unwrap_or_else(|e| panic!("bind pid_step_antiwindup: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run pid_step_antiwindup case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "pid_step_antiwindup case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "dt" | "integral" | "integral_out" | "kd" | "ki" | "kp" | "measurement" | "out_max" | "out_min" | "output" | "prev_error" | "prev_error_out" | "setpoint");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "pid_step_antiwindup case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "pid_step_antiwindup case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn slew_rate_limiter_step_matches_test_cases() {
    use crate::common::run_cell;
    let cases: &[(u16, u16, u16, u16)] = &[
        (100, 150, 20, 120),
        (100, 150, 100, 150),
        (200, 50, 30, 170),
        (500, 500, 10, 500),
        (500, 800, 0, 500),
        (0, 65535, 65535, 65535),
        (65535, 0, 1, 65534),
    ];
    for (i, (current, target, max_delta_per_step, expected)) in cases.iter().enumerate() {
        let got = run_cell(
            "slew_rate_limiter_step",
            &[*current, *target, *max_delta_per_step],
        );
        assert_eq!(
            got, *expected,
            "slew_rate_limiter_step case {i}: got {got} want {expected}"
        );
    }
}

#[test]
fn deadband_matches_test_cases() {
    use crate::common::run_cell;
    // value/center are i16 (signed); run_cell's raw-register interface takes/returns u16
    // bit patterns (the signed-deltas pack's own convention — `-5` <-> `65531`).
    let cases: &[(i16, i16, u16, i16)] = &[
        (100, 100, 5, 0),
        (103, 100, 5, 0),
        (105, 100, 5, 0),
        (106, 100, 5, 6),
        (94, 100, 5, -6),
        (-50, -60, 5, 10),
        (7, 5, 0, 2),
        (5, 5, 0, 0),
    ];
    for (i, (value, center, band_width, expected)) in cases.iter().enumerate() {
        let got =
            run_cell("deadband", &[*value as u16, *center as u16, *band_width]) as i16;
        assert_eq!(got, *expected, "deadband case {i}: got {got} want {expected}");
    }
}

#[test]
fn bang_bang_controller_matches_test_cases() {
    use crate::common::run_cell;
    let cases: &[(u16, u16, u16, i16)] = &[
        (90, 100, 5, 1),
        (110, 100, 5, -1),
        (100, 100, 5, 0),
        (95, 100, 5, 0),
        (105, 100, 5, 0),
        (0, 3, 10, 0),
        (65535, 65530, 1000, 0),
        (49, 50, 0, 1),
        (51, 50, 0, -1),
    ];
    for (i, (value, setpoint, deadband, expected)) in cases.iter().enumerate() {
        let got = run_cell("bang_bang_controller", &[*value, *setpoint, *deadband]) as i16;
        assert_eq!(
            got, *expected,
            "bang_bang_controller case {i}: got {got} want {expected}"
        );
    }
}
