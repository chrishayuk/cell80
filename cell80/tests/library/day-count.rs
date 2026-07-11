//! Host-oracle tests for the day-count pack (`cell80/cells/day-count/*.rs`) — mechanically
//! generated from each cell's own proposed test cases (Finance80 batch, verify->admit
//! loop) rather than hand-transcribed: every `test_cases` entry from the authoring
//! output becomes one comparison inside its cell's `#[test]` function, run against the
//! real compiled cell via `StateCell::bind`/`set`/`run`/`get` (see
//! `cell80/tests/library/common.rs` for the shared `cell_src` helper). f32 fields ride
//! raw bit patterns (`to_bits`/`from_bits`, the physics/softfloat packs' own convention)
//! and compare with a small epsilon tolerance rather than bit-exactness, since these
//! expected values are hand-derived arithmetic, not a host-rustc oracle.
//
// Mechanically generated scaffolds: single-type cells degenerate to `match name
// { _ => .. }` and every case table shares one tuple shape — style lints the
// generator would re-trip next wave are allowed rather than hand-patched.
#![allow(clippy::match_single_binding, clippy::type_complexity)]

#[test]
fn date_add_months_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("year", 2024.0),
                ("month", 1.0),
                ("day", 31.0),
                ("months", 1.0),
                ("direction", 0.0),
            ],
            "new_year",
            2024.0,
        ),
        (
            &[
                ("year", 2024.0),
                ("month", 1.0),
                ("day", 31.0),
                ("months", 1.0),
                ("direction", 0.0),
            ],
            "new_month",
            2.0,
        ),
        (
            &[
                ("year", 2024.0),
                ("month", 1.0),
                ("day", 31.0),
                ("months", 1.0),
                ("direction", 0.0),
            ],
            "new_day",
            29.0,
        ),
        (
            &[
                ("year", 2023.0),
                ("month", 1.0),
                ("day", 31.0),
                ("months", 1.0),
                ("direction", 0.0),
            ],
            "new_day",
            28.0,
        ),
        (
            &[
                ("year", 2024.0),
                ("month", 1.0),
                ("day", 15.0),
                ("months", 2.0),
                ("direction", 1.0),
            ],
            "new_year",
            2023.0,
        ),
        (
            &[
                ("year", 2024.0),
                ("month", 1.0),
                ("day", 15.0),
                ("months", 2.0),
                ("direction", 1.0),
            ],
            "new_month",
            11.0,
        ),
        (
            &[
                ("year", 2024.0),
                ("month", 1.0),
                ("day", 15.0),
                ("months", 2.0),
                ("direction", 1.0),
            ],
            "new_day",
            15.0,
        ),
        (
            &[
                ("year", 2023.0),
                ("month", 1.0),
                ("day", 31.0),
                ("months", 2.0),
                ("direction", 0.0),
            ],
            "new_day",
            31.0,
        ),
        (
            &[
                ("year", 2024.0),
                ("month", 3.0),
                ("day", 31.0),
                ("months", 1.0),
                ("direction", 1.0),
            ],
            "new_day",
            29.0,
        ),
        (
            &[
                ("year", 2025.0),
                ("month", 6.0),
                ("day", 15.0),
                ("months", 4.0),
                ("direction", 0.0),
            ],
            "new_month",
            10.0,
        ),
        (
            &[
                ("year", 2022.0),
                ("month", 5.0),
                ("day", 20.0),
                ("months", 0.0),
                ("direction", 0.0),
            ],
            "new_month",
            5.0,
        ),
        (
            &[
                ("year", 1900.0),
                ("month", 1.0),
                ("day", 30.0),
                ("months", 1.0),
                ("direction", 0.0),
            ],
            "new_day",
            28.0,
        ),
        (
            &[
                ("year", 2000.0),
                ("month", 1.0),
                ("day", 30.0),
                ("months", 1.0),
                ("direction", 0.0),
            ],
            "new_day",
            29.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("date_add_months"),
            "DateAddMonths",
            None,
        )
        .unwrap_or_else(|e| panic!("bind date_add_months: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run date_add_months case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "date_add_months case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "__none__");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "date_add_months case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "date_add_months case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn day_count_30_360_eu_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("y1", 2024.0),
                ("m1", 1.0),
                ("d1", 15.0),
                ("y2", 2024.0),
                ("m2", 3.0),
                ("d2", 20.0),
            ],
            "days_mag",
            65.0,
        ),
        (
            &[
                ("y1", 2024.0),
                ("m1", 1.0),
                ("d1", 31.0),
                ("y2", 2024.0),
                ("m2", 2.0),
                ("d2", 1.0),
            ],
            "days_mag",
            1.0,
        ),
        (
            &[
                ("y1", 2024.0),
                ("m1", 2.0),
                ("d1", 1.0),
                ("y2", 2024.0),
                ("m2", 3.0),
                ("d2", 31.0),
            ],
            "days_mag",
            59.0,
        ),
        (
            &[
                ("y1", 2023.0),
                ("m1", 11.0),
                ("d1", 30.0),
                ("y2", 2024.0),
                ("m2", 2.0),
                ("d2", 15.0),
            ],
            "days_mag",
            75.0,
        ),
        (
            &[
                ("y1", 2024.0),
                ("m1", 6.0),
                ("d1", 10.0),
                ("y2", 2024.0),
                ("m2", 1.0),
                ("d2", 5.0),
            ],
            "days_mag",
            155.0,
        ),
        (
            &[
                ("y1", 2024.0),
                ("m1", 6.0),
                ("d1", 10.0),
                ("y2", 2024.0),
                ("m2", 1.0),
                ("d2", 5.0),
            ],
            "days_neg",
            1.0,
        ),
        (
            &[
                ("y1", 2024.0),
                ("m1", 5.0),
                ("d1", 17.0),
                ("y2", 2024.0),
                ("m2", 5.0),
                ("d2", 17.0),
            ],
            "days_mag",
            0.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("day_count_30_360_eu"),
            "DayCount30360Eu",
            None,
        )
        .unwrap_or_else(|e| panic!("bind day_count_30_360_eu: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run day_count_30_360_eu case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "day_count_30_360_eu case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "__none__");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!((got - expected_value).abs() < tol, "day_count_30_360_eu case {i}: field {expected_field} got {got} want {expected_value}");
        } else {
            assert_eq!(got.round() as i64, *expected_value as i64, "day_count_30_360_eu case {i}: field {expected_field} got {got} want {expected_value}");
        }
    }
}

#[test]
fn day_count_act_act_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("y1", 2021.0),
                ("m1", 1.0),
                ("d1", 1.0),
                ("y2", 2022.0),
                ("m2", 1.0),
                ("d2", 1.0),
            ],
            "num",
            365.0,
        ),
        (
            &[
                ("y1", 2021.0),
                ("m1", 1.0),
                ("d1", 1.0),
                ("y2", 2022.0),
                ("m2", 1.0),
                ("d2", 1.0),
            ],
            "den",
            365.0,
        ),
        (
            &[
                ("y1", 2020.0),
                ("m1", 1.0),
                ("d1", 1.0),
                ("y2", 2021.0),
                ("m2", 1.0),
                ("d2", 1.0),
            ],
            "num",
            366.0,
        ),
        (
            &[
                ("y1", 2020.0),
                ("m1", 1.0),
                ("d1", 1.0),
                ("y2", 2021.0),
                ("m2", 1.0),
                ("d2", 1.0),
            ],
            "den",
            366.0,
        ),
        (
            &[
                ("y1", 2020.0),
                ("m1", 2.0),
                ("d1", 1.0),
                ("y2", 2020.0),
                ("m2", 3.0),
                ("d2", 1.0),
            ],
            "num",
            29.0,
        ),
        (
            &[
                ("y1", 2020.0),
                ("m1", 2.0),
                ("d1", 1.0),
                ("y2", 2020.0),
                ("m2", 3.0),
                ("d2", 1.0),
            ],
            "den",
            366.0,
        ),
        (
            &[
                ("y1", 2020.0),
                ("m1", 3.0),
                ("d1", 1.0),
                ("y2", 2021.0),
                ("m2", 3.0),
                ("d2", 1.0),
            ],
            "num",
            365.0,
        ),
        (
            &[
                ("y1", 2020.0),
                ("m1", 3.0),
                ("d1", 1.0),
                ("y2", 2021.0),
                ("m2", 3.0),
                ("d2", 1.0),
            ],
            "den",
            365.0,
        ),
        (
            &[
                ("y1", 2023.0),
                ("m1", 6.0),
                ("d1", 15.0),
                ("y2", 2023.0),
                ("m2", 6.0),
                ("d2", 15.0),
            ],
            "num",
            0.0,
        ),
        (
            &[
                ("y1", 2023.0),
                ("m1", 6.0),
                ("d1", 15.0),
                ("y2", 2023.0),
                ("m2", 6.0),
                ("d2", 15.0),
            ],
            "den",
            365.0,
        ),
        (
            &[
                ("y1", 2022.0),
                ("m1", 1.0),
                ("d1", 1.0),
                ("y2", 2021.0),
                ("m2", 1.0),
                ("d2", 1.0),
            ],
            "num",
            365.0,
        ),
        (
            &[
                ("y1", 2022.0),
                ("m1", 1.0),
                ("d1", 1.0),
                ("y2", 2021.0),
                ("m2", 1.0),
                ("d2", 1.0),
            ],
            "den",
            365.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("day_count_act_act"),
            "DayCountActAct",
            None,
        )
        .unwrap_or_else(|e| panic!("bind day_count_act_act: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run day_count_act_act case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "day_count_act_act case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "__none__");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!((got - expected_value).abs() < tol, "day_count_act_act case {i}: field {expected_field} got {got} want {expected_value}");
        } else {
            assert_eq!(got.round() as i64, *expected_value as i64, "day_count_act_act case {i}: field {expected_field} got {got} want {expected_value}");
        }
    }
}

#[test]
fn day_count_act_360_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "year_fraction" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "year_fraction" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("y1", 2024.0),
                ("m1", 6.0),
                ("d1", 10.0),
                ("y2", 2024.0),
                ("m2", 6.0),
                ("d2", 10.0),
            ],
            "year_fraction",
            0.0,
        ),
        (
            &[
                ("y1", 2023.0),
                ("m1", 1.0),
                ("d1", 1.0),
                ("y2", 2023.0),
                ("m2", 4.0),
                ("d2", 1.0),
            ],
            "year_fraction",
            0.25,
        ),
        (
            &[
                ("y1", 2023.0),
                ("m1", 1.0),
                ("d1", 1.0),
                ("y2", 2023.0),
                ("m2", 6.0),
                ("d2", 30.0),
            ],
            "year_fraction",
            0.5,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("day_count_act_360"),
            "DayCountAct360",
            None,
        )
        .unwrap_or_else(|e| panic!("bind day_count_act_360: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run day_count_act_360 case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "day_count_act_360 case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "year_fraction");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!((got - expected_value).abs() < tol, "day_count_act_360 case {i}: field {expected_field} got {got} want {expected_value}");
        } else {
            assert_eq!(got.round() as i64, *expected_value as i64, "day_count_act_360 case {i}: field {expected_field} got {got} want {expected_value}");
        }
    }
}

#[test]
fn day_count_act_365_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "year_fraction" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "year_fraction" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("y1", 2024.0),
                ("m1", 6.0),
                ("d1", 15.0),
                ("y2", 2024.0),
                ("m2", 6.0),
                ("d2", 15.0),
            ],
            "year_fraction",
            0.0,
        ),
        (
            &[
                ("y1", 2023.0),
                ("m1", 1.0),
                ("d1", 1.0),
                ("y2", 2024.0),
                ("m2", 1.0),
                ("d2", 1.0),
            ],
            "year_fraction",
            1.0,
        ),
        (
            &[
                ("y1", 2024.0),
                ("m1", 1.0),
                ("d1", 1.0),
                ("y2", 2025.0),
                ("m2", 1.0),
                ("d2", 1.0),
            ],
            "year_fraction",
            1.002739667892456,
        ),
        (
            &[
                ("y1", 2024.0),
                ("m1", 1.0),
                ("d1", 1.0),
                ("y2", 2024.0),
                ("m2", 7.0),
                ("d2", 1.0),
            ],
            "days",
            182.0,
        ),
        (
            &[
                ("y1", 2024.0),
                ("m1", 1.0),
                ("d1", 1.0),
                ("y2", 2024.0),
                ("m2", 7.0),
                ("d2", 1.0),
            ],
            "year_fraction",
            0.4986301362514496,
        ),
        (
            &[
                ("y1", 2025.0),
                ("m1", 3.0),
                ("d1", 11.0),
                ("y2", 2025.0),
                ("m2", 3.0),
                ("d2", 1.0),
            ],
            "days",
            10.0,
        ),
        (
            &[
                ("y1", 2020.0),
                ("m1", 2.0),
                ("d1", 28.0),
                ("y2", 2020.0),
                ("m2", 3.0),
                ("d2", 1.0),
            ],
            "days",
            2.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("day_count_act_365"),
            "DayCountAct365",
            None,
        )
        .unwrap_or_else(|e| panic!("bind day_count_act_365: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run day_count_act_365 case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "day_count_act_365 case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "year_fraction");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!((got - expected_value).abs() < tol, "day_count_act_365 case {i}: field {expected_field} got {got} want {expected_value}");
        } else {
            assert_eq!(got.round() as i64, *expected_value as i64, "day_count_act_365 case {i}: field {expected_field} got {got} want {expected_value}");
        }
    }
}
