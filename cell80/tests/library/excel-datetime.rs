//! Host-oracle tests for the excel-datetime pack (`cell80/cells/excel-datetime/*.rs`) —
//! mechanically generated from each cell's own proposed test cases (the 22-cell
//! excel-datetime/control-systems/numerical-primitives batch, verify->admit loop) rather
//! than hand-transcribed: every `test_cases` entry from the authoring output becomes one
//! comparison inside its cell's `#[test]` function, run against the real compiled cell via
//! `StateCell::bind`/`set`/`run`/`get` (see `cell80/tests/library/common.rs` for the shared
//! `cell_src` helper). f32 fields ride raw bit patterns (`to_bits`/`from_bits`, the
//! physics/softfloat packs' own convention) and compare with a small epsilon tolerance
//! rather than bit-exactness, since these expected values are hand-derived arithmetic, not
//! a host-rustc oracle.
//
// Mechanically generated scaffolds: single-type cells degenerate to `match name
// { _ => .. }` and every case table shares one tuple shape — style lints the
// generator would re-trip next wave are allowed rather than hand-patched.
#![allow(clippy::match_single_binding, clippy::type_complexity)]

#[test]
fn excel_eomonth_matches_test_cases() {
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
                ("year", 2026.0),
                ("month", 1.0),
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
                ("months", 1.0),
                ("direction", 0.0),
            ],
            "new_day",
            29.0,
        ),
        (
            &[
                ("year", 2026.0),
                ("month", 1.0),
                ("months", 1.0),
                ("direction", 1.0),
            ],
            "new_day",
            31.0,
        ),
        (
            &[
                ("year", 2026.0),
                ("month", 1.0),
                ("months", 1.0),
                ("direction", 1.0),
            ],
            "new_year",
            2025.0,
        ),
        (
            &[
                ("year", 2026.0),
                ("month", 1.0),
                ("months", 1.0),
                ("direction", 1.0),
            ],
            "new_month",
            12.0,
        ),
        (
            &[
                ("year", 2099.0),
                ("month", 12.0),
                ("months", 2.0),
                ("direction", 0.0),
            ],
            "new_day",
            28.0,
        ),
        (
            &[
                ("year", 2099.0),
                ("month", 12.0),
                ("months", 2.0),
                ("direction", 0.0),
            ],
            "new_year",
            2100.0,
        ),
        (
            &[
                ("year", 2026.0),
                ("month", 4.0),
                ("months", 0.0),
                ("direction", 0.0),
            ],
            "new_day",
            30.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_eomonth"),
            "ExcelEomonth",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_eomonth: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_eomonth case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_eomonth case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "__none__");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_eomonth case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_eomonth case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_days360_matches_test_cases() {
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
                ("y1", 2020.0),
                ("m1", 1.0),
                ("d1", 15.0),
                ("y2", 2020.0),
                ("m2", 3.0),
                ("d2", 1.0),
                ("method", 0.0),
            ],
            "days_mag",
            46.0,
        ),
        (
            &[
                ("y1", 2021.0),
                ("m1", 1.0),
                ("d1", 31.0),
                ("y2", 2021.0),
                ("m2", 3.0),
                ("d2", 31.0),
                ("method", 0.0),
            ],
            "days_mag",
            60.0,
        ),
        (
            &[
                ("y1", 2021.0),
                ("m1", 1.0),
                ("d1", 15.0),
                ("y2", 2021.0),
                ("m2", 1.0),
                ("d2", 31.0),
                ("method", 0.0),
            ],
            "days_mag",
            16.0,
        ),
        (
            &[
                ("y1", 2021.0),
                ("m1", 1.0),
                ("d1", 15.0),
                ("y2", 2021.0),
                ("m2", 1.0),
                ("d2", 31.0),
                ("method", 1.0),
            ],
            "days_mag",
            15.0,
        ),
        (
            &[
                ("y1", 2022.0),
                ("m1", 1.0),
                ("d1", 1.0),
                ("y2", 2021.0),
                ("m2", 1.0),
                ("d2", 1.0),
                ("method", 1.0),
            ],
            "days_mag",
            360.0,
        ),
        (
            &[
                ("y1", 2022.0),
                ("m1", 1.0),
                ("d1", 1.0),
                ("y2", 2021.0),
                ("m2", 1.0),
                ("d2", 1.0),
                ("method", 1.0),
            ],
            "days_neg",
            1.0,
        ),
        (
            &[
                ("y1", 0.0),
                ("m1", 1.0),
                ("d1", 1.0),
                ("y2", 65535.0),
                ("m2", 12.0),
                ("d2", 30.0),
                ("method", 1.0),
            ],
            "days_mag",
            23592959.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_days360"),
            "ExcelDays360",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_days360: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_days360 case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_days360 case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "__none__");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_days360 case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_days360 case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_days_matches_test_cases() {
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
                ("y_end", 2024.0),
                ("m_end", 1.0),
                ("d_end", 31.0),
                ("y_start", 2024.0),
                ("m_start", 1.0),
                ("d_start", 1.0),
            ],
            "days_mag",
            30.0,
        ),
        (
            &[
                ("y_end", 2024.0),
                ("m_end", 1.0),
                ("d_end", 31.0),
                ("y_start", 2024.0),
                ("m_start", 1.0),
                ("d_start", 1.0),
            ],
            "days_neg",
            0.0,
        ),
        (
            &[
                ("y_end", 2024.0),
                ("m_end", 6.0),
                ("d_end", 1.0),
                ("y_start", 2024.0),
                ("m_start", 6.0),
                ("d_start", 15.0),
            ],
            "days_mag",
            14.0,
        ),
        (
            &[
                ("y_end", 2024.0),
                ("m_end", 6.0),
                ("d_end", 1.0),
                ("y_start", 2024.0),
                ("m_start", 6.0),
                ("d_start", 15.0),
            ],
            "days_neg",
            1.0,
        ),
        (
            &[
                ("y_end", 2024.0),
                ("m_end", 1.0),
                ("d_end", 5.0),
                ("y_start", 2023.0),
                ("m_start", 12.0),
                ("d_start", 25.0),
            ],
            "days_mag",
            11.0,
        ),
        (
            &[
                ("y_end", 2024.0),
                ("m_end", 1.0),
                ("d_end", 5.0),
                ("y_start", 2023.0),
                ("m_start", 12.0),
                ("d_start", 25.0),
            ],
            "days_neg",
            0.0,
        ),
        (
            &[
                ("y_end", 2024.0),
                ("m_end", 3.0),
                ("d_end", 1.0),
                ("y_start", 2024.0),
                ("m_start", 2.0),
                ("d_start", 28.0),
            ],
            "days_mag",
            2.0,
        ),
        (
            &[
                ("y_end", 2024.0),
                ("m_end", 3.0),
                ("d_end", 1.0),
                ("y_start", 2024.0),
                ("m_start", 2.0),
                ("d_start", 28.0),
            ],
            "days_neg",
            0.0,
        ),
        (
            &[
                ("y_end", 2024.0),
                ("m_end", 7.0),
                ("d_end", 4.0),
                ("y_start", 2024.0),
                ("m_start", 7.0),
                ("d_start", 4.0),
            ],
            "days_mag",
            0.0,
        ),
        (
            &[
                ("y_end", 2024.0),
                ("m_end", 7.0),
                ("d_end", 4.0),
                ("y_start", 2024.0),
                ("m_start", 7.0),
                ("d_start", 4.0),
            ],
            "days_neg",
            0.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell =
            cell80::StateCell::bind(&crate::common::cell_src("excel_days"), "ExcelDays", None)
                .unwrap_or_else(|e| panic!("bind excel_days: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_days case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_days case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "__none__");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_days case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_days case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_datedif_matches_test_cases() {
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
                ("y_start", 2020.0),
                ("m_start", 1.0),
                ("d_start", 15.0),
                ("y_end", 2023.0),
                ("m_end", 3.0),
                ("d_end", 10.0),
                ("unit", 0.0),
            ],
            "result",
            3.0,
        ),
        (
            &[
                ("y_start", 2020.0),
                ("m_start", 1.0),
                ("d_start", 15.0),
                ("y_end", 2023.0),
                ("m_end", 3.0),
                ("d_end", 10.0),
                ("unit", 1.0),
            ],
            "result",
            37.0,
        ),
        (
            &[
                ("y_start", 2020.0),
                ("m_start", 1.0),
                ("d_start", 15.0),
                ("y_end", 2023.0),
                ("m_end", 3.0),
                ("d_end", 10.0),
                ("unit", 2.0),
            ],
            "result",
            1150.0,
        ),
        (
            &[
                ("y_start", 2020.0),
                ("m_start", 1.0),
                ("d_start", 15.0),
                ("y_end", 2023.0),
                ("m_end", 3.0),
                ("d_end", 10.0),
                ("unit", 3.0),
            ],
            "result",
            23.0,
        ),
        (
            &[
                ("y_start", 2020.0),
                ("m_start", 1.0),
                ("d_start", 15.0),
                ("y_end", 2023.0),
                ("m_end", 3.0),
                ("d_end", 10.0),
                ("unit", 4.0),
            ],
            "result",
            1.0,
        ),
        (
            &[
                ("y_start", 2020.0),
                ("m_start", 1.0),
                ("d_start", 15.0),
                ("y_end", 2023.0),
                ("m_end", 3.0),
                ("d_end", 10.0),
                ("unit", 5.0),
            ],
            "result",
            54.0,
        ),
        (
            &[
                ("y_start", 2021.0),
                ("m_start", 6.0),
                ("d_start", 10.0),
                ("y_end", 2022.0),
                ("m_end", 5.0),
                ("d_end", 20.0),
                ("unit", 0.0),
            ],
            "result",
            0.0,
        ),
        (
            &[
                ("y_start", 2021.0),
                ("m_start", 6.0),
                ("d_start", 10.0),
                ("y_end", 2022.0),
                ("m_end", 5.0),
                ("d_end", 20.0),
                ("unit", 1.0),
            ],
            "result",
            11.0,
        ),
        (
            &[
                ("y_start", 2021.0),
                ("m_start", 6.0),
                ("d_start", 10.0),
                ("y_end", 2022.0),
                ("m_end", 5.0),
                ("d_end", 20.0),
                ("unit", 2.0),
            ],
            "result",
            344.0,
        ),
        (
            &[
                ("y_start", 2021.0),
                ("m_start", 6.0),
                ("d_start", 10.0),
                ("y_end", 2022.0),
                ("m_end", 5.0),
                ("d_end", 20.0),
                ("unit", 3.0),
            ],
            "result",
            10.0,
        ),
        (
            &[
                ("y_start", 2021.0),
                ("m_start", 6.0),
                ("d_start", 10.0),
                ("y_end", 2022.0),
                ("m_end", 5.0),
                ("d_end", 20.0),
                ("unit", 4.0),
            ],
            "result",
            11.0,
        ),
        (
            &[
                ("y_start", 2021.0),
                ("m_start", 6.0),
                ("d_start", 10.0),
                ("y_end", 2022.0),
                ("m_end", 5.0),
                ("d_end", 20.0),
                ("unit", 5.0),
            ],
            "result",
            344.0,
        ),
        (
            &[
                ("y_start", 2020.0),
                ("m_start", 2.0),
                ("d_start", 29.0),
                ("y_end", 2023.0),
                ("m_end", 3.0),
                ("d_end", 1.0),
                ("unit", 0.0),
            ],
            "result",
            3.0,
        ),
        (
            &[
                ("y_start", 2020.0),
                ("m_start", 2.0),
                ("d_start", 29.0),
                ("y_end", 2023.0),
                ("m_end", 3.0),
                ("d_end", 1.0),
                ("unit", 1.0),
            ],
            "result",
            36.0,
        ),
        (
            &[
                ("y_start", 2020.0),
                ("m_start", 2.0),
                ("d_start", 29.0),
                ("y_end", 2023.0),
                ("m_end", 3.0),
                ("d_end", 1.0),
                ("unit", 2.0),
            ],
            "result",
            1096.0,
        ),
        (
            &[
                ("y_start", 2020.0),
                ("m_start", 2.0),
                ("d_start", 29.0),
                ("y_end", 2023.0),
                ("m_end", 3.0),
                ("d_end", 1.0),
                ("unit", 3.0),
            ],
            "result",
            1.0,
        ),
        (
            &[
                ("y_start", 2020.0),
                ("m_start", 2.0),
                ("d_start", 29.0),
                ("y_end", 2023.0),
                ("m_end", 3.0),
                ("d_end", 1.0),
                ("unit", 4.0),
            ],
            "result",
            0.0,
        ),
        (
            &[
                ("y_start", 2020.0),
                ("m_start", 2.0),
                ("d_start", 29.0),
                ("y_end", 2023.0),
                ("m_end", 3.0),
                ("d_end", 1.0),
                ("unit", 5.0),
            ],
            "result",
            1.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_datedif"),
            "ExcelDatedif",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_datedif: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_datedif case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_datedif case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "__none__");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_datedif case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_datedif case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_weekday_matches_test_cases() {
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
                ("year", 2000.0),
                ("month", 1.0),
                ("day", 1.0),
                ("return_type", 1.0),
            ],
            "weekday",
            7.0,
        ),
        (
            &[
                ("year", 2000.0),
                ("month", 1.0),
                ("day", 1.0),
                ("return_type", 2.0),
            ],
            "weekday",
            6.0,
        ),
        (
            &[
                ("year", 2000.0),
                ("month", 1.0),
                ("day", 1.0),
                ("return_type", 3.0),
            ],
            "weekday",
            5.0,
        ),
        (
            &[
                ("year", 1776.0),
                ("month", 7.0),
                ("day", 4.0),
                ("return_type", 1.0),
            ],
            "weekday",
            5.0,
        ),
        (
            &[
                ("year", 1776.0),
                ("month", 7.0),
                ("day", 4.0),
                ("return_type", 2.0),
            ],
            "weekday",
            4.0,
        ),
        (
            &[
                ("year", 1776.0),
                ("month", 7.0),
                ("day", 4.0),
                ("return_type", 3.0),
            ],
            "weekday",
            3.0,
        ),
        (
            &[
                ("year", 2023.0),
                ("month", 1.0),
                ("day", 1.0),
                ("return_type", 1.0),
            ],
            "weekday",
            1.0,
        ),
        (
            &[
                ("year", 2023.0),
                ("month", 1.0),
                ("day", 1.0),
                ("return_type", 2.0),
            ],
            "weekday",
            7.0,
        ),
        (
            &[
                ("year", 2023.0),
                ("month", 1.0),
                ("day", 1.0),
                ("return_type", 3.0),
            ],
            "weekday",
            6.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_weekday"),
            "ExcelWeekday",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_weekday: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_weekday case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_weekday case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "__none__");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_weekday case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_weekday case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_weeknum_matches_test_cases() {
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
                ("year", 2026.0),
                ("month", 1.0),
                ("day", 1.0),
                ("return_type", 1.0),
            ],
            "week",
            1.0,
        ),
        (
            &[
                ("year", 2026.0),
                ("month", 1.0),
                ("day", 4.0),
                ("return_type", 1.0),
            ],
            "week",
            2.0,
        ),
        (
            &[
                ("year", 2026.0),
                ("month", 1.0),
                ("day", 5.0),
                ("return_type", 2.0),
            ],
            "week",
            2.0,
        ),
        (
            &[
                ("year", 2026.0),
                ("month", 1.0),
                ("day", 4.0),
                ("return_type", 2.0),
            ],
            "week",
            1.0,
        ),
        (
            &[
                ("year", 2025.0),
                ("month", 12.0),
                ("day", 31.0),
                ("return_type", 1.0),
            ],
            "week",
            53.0,
        ),
        (
            &[
                ("year", 2024.0),
                ("month", 2.0),
                ("day", 29.0),
                ("return_type", 1.0),
            ],
            "week",
            9.0,
        ),
        (
            &[
                ("year", 2024.0),
                ("month", 7.0),
                ("day", 4.0),
                ("return_type", 2.0),
            ],
            "week",
            27.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_weeknum"),
            "ExcelWeeknum",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_weeknum: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_weeknum case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_weeknum case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "__none__");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_weeknum case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_weeknum case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_isoweeknum_matches_test_cases() {
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
            &[("year", 2026.0), ("month", 7.0), ("day", 11.0)],
            "week",
            28.0,
        ),
        (
            &[("year", 2005.0), ("month", 1.0), ("day", 1.0)],
            "week",
            53.0,
        ),
        (
            &[("year", 2018.0), ("month", 12.0), ("day", 31.0)],
            "week",
            1.0,
        ),
        (
            &[("year", 2026.0), ("month", 1.0), ("day", 1.0)],
            "week",
            1.0,
        ),
        (
            &[("year", 2020.0), ("month", 12.0), ("day", 31.0)],
            "week",
            53.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_isoweeknum"),
            "ExcelIsoweeknum",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_isoweeknum: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_isoweeknum case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_isoweeknum case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "__none__");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_isoweeknum case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_isoweeknum case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_yearfrac_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "year_frac" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "year_frac" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("y_start", 2026.0),
                ("m_start", 1.0),
                ("d_start", 1.0),
                ("y_end", 2026.0),
                ("m_end", 4.0),
                ("d_end", 1.0),
                ("basis", 2.0),
            ],
            "year_frac",
            0.25,
        ),
        (
            &[
                ("y_start", 2026.0),
                ("m_start", 1.0),
                ("d_start", 1.0),
                ("y_end", 2026.0),
                ("m_end", 4.0),
                ("d_end", 1.0),
                ("basis", 3.0),
            ],
            "year_frac",
            0.2465753425,
        ),
        (
            &[
                ("y_start", 2024.0),
                ("m_start", 2.0),
                ("d_start", 1.0),
                ("y_end", 2024.0),
                ("m_end", 3.0),
                ("d_end", 1.0),
                ("basis", 1.0),
            ],
            "year_frac",
            0.0792349727,
        ),
        (
            &[
                ("y_start", 2025.0),
                ("m_start", 2.0),
                ("d_start", 1.0),
                ("y_end", 2025.0),
                ("m_end", 3.0),
                ("d_end", 1.0),
                ("basis", 1.0),
            ],
            "year_frac",
            0.0767123288,
        ),
        (
            &[
                ("y_start", 2026.0),
                ("m_start", 1.0),
                ("d_start", 15.0),
                ("y_end", 2026.0),
                ("m_end", 1.0),
                ("d_end", 31.0),
                ("basis", 0.0),
            ],
            "year_frac",
            0.0444444444,
        ),
        (
            &[
                ("y_start", 2026.0),
                ("m_start", 1.0),
                ("d_start", 15.0),
                ("y_end", 2026.0),
                ("m_end", 1.0),
                ("d_end", 31.0),
                ("basis", 4.0),
            ],
            "year_frac",
            0.0416666667,
        ),
        (
            &[
                ("y_start", 2026.0),
                ("m_start", 1.0),
                ("d_start", 31.0),
                ("y_end", 2026.0),
                ("m_end", 3.0),
                ("d_end", 31.0),
                ("basis", 0.0),
            ],
            "year_frac",
            0.1666666667,
        ),
        (
            &[
                ("y_start", 2026.0),
                ("m_start", 4.0),
                ("d_start", 1.0),
                ("y_end", 2026.0),
                ("m_end", 1.0),
                ("d_end", 1.0),
                ("basis", 2.0),
            ],
            "year_frac",
            0.25,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_yearfrac"),
            "ExcelYearfrac",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_yearfrac: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_yearfrac case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_yearfrac case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "year_frac");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_yearfrac case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_yearfrac case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_networkdays_matches_test_cases() {
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
                ("y_start", 2024.0),
                ("m_start", 1.0),
                ("d_start", 1.0),
                ("y_end", 2024.0),
                ("m_end", 1.0),
                ("d_end", 7.0),
            ],
            "workdays_mag",
            5.0,
        ),
        (
            &[
                ("y_start", 2024.0),
                ("m_start", 1.0),
                ("d_start", 1.0),
                ("y_end", 2024.0),
                ("m_end", 1.0),
                ("d_end", 7.0),
            ],
            "workdays_neg",
            0.0,
        ),
        (
            &[
                ("y_start", 2024.0),
                ("m_start", 1.0),
                ("d_start", 7.0),
                ("y_end", 2024.0),
                ("m_end", 1.0),
                ("d_end", 1.0),
            ],
            "workdays_mag",
            5.0,
        ),
        (
            &[
                ("y_start", 2024.0),
                ("m_start", 1.0),
                ("d_start", 7.0),
                ("y_end", 2024.0),
                ("m_end", 1.0),
                ("d_end", 1.0),
            ],
            "workdays_neg",
            1.0,
        ),
        (
            &[
                ("y_start", 2024.0),
                ("m_start", 3.0),
                ("d_start", 13.0),
                ("y_end", 2024.0),
                ("m_end", 3.0),
                ("d_end", 13.0),
            ],
            "workdays_mag",
            1.0,
        ),
        (
            &[
                ("y_start", 2024.0),
                ("m_start", 2.0),
                ("d_start", 28.0),
                ("y_end", 2024.0),
                ("m_end", 3.0),
                ("d_end", 1.0),
            ],
            "workdays_mag",
            3.0,
        ),
        (
            &[
                ("y_start", 2024.0),
                ("m_start", 1.0),
                ("d_start", 6.0),
                ("y_end", 2024.0),
                ("m_end", 1.0),
                ("d_end", 7.0),
            ],
            "workdays_mag",
            0.0,
        ),
        (
            &[
                ("y_start", 2024.0),
                ("m_start", 1.0),
                ("d_start", 1.0),
                ("y_end", 2024.0),
                ("m_end", 1.0),
                ("d_end", 14.0),
            ],
            "workdays_mag",
            10.0,
        ),
        (
            &[
                ("y_start", 2024.0),
                ("m_start", 1.0),
                ("d_start", 6.0),
                ("y_end", 2024.0),
                ("m_end", 1.0),
                ("d_end", 6.0),
            ],
            "workdays_mag",
            0.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_networkdays"),
            "ExcelNetworkdays",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_networkdays: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_networkdays case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_networkdays case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "__none__");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_networkdays case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_networkdays case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_networkdays_intl_matches_test_cases() {
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
                ("d1", 1.0),
                ("y2", 2024.0),
                ("m2", 1.0),
                ("d2", 7.0),
                ("weekend_mask", 3.0),
            ],
            "workdays_mag",
            5.0,
        ),
        (
            &[
                ("y1", 2024.0),
                ("m1", 1.0),
                ("d1", 1.0),
                ("y2", 2024.0),
                ("m2", 1.0),
                ("d2", 5.0),
                ("weekend_mask", 65.0),
            ],
            "workdays_mag",
            4.0,
        ),
        (
            &[
                ("y1", 2024.0),
                ("m1", 1.0),
                ("d1", 5.0),
                ("y2", 2024.0),
                ("m2", 1.0),
                ("d2", 1.0),
                ("weekend_mask", 3.0),
            ],
            "workdays_neg",
            1.0,
        ),
        (
            &[
                ("y1", 2024.0),
                ("m1", 1.0),
                ("d1", 5.0),
                ("y2", 2024.0),
                ("m2", 1.0),
                ("d2", 1.0),
                ("weekend_mask", 3.0),
            ],
            "workdays_mag",
            5.0,
        ),
        (
            &[
                ("y1", 2024.0),
                ("m1", 1.0),
                ("d1", 6.0),
                ("y2", 2024.0),
                ("m2", 1.0),
                ("d2", 6.0),
                ("weekend_mask", 3.0),
            ],
            "workdays_mag",
            0.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_networkdays_intl"),
            "ExcelNetworkdaysIntl",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_networkdays_intl: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_networkdays_intl case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_networkdays_intl case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "__none__");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_networkdays_intl case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_networkdays_intl case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_workday_matches_test_cases() {
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
                ("year", 2026.0),
                ("month", 7.0),
                ("day", 13.0),
                ("num_days", 3.0),
                ("direction", 0.0),
            ],
            "new_day",
            16.0,
        ),
        (
            &[
                ("year", 2026.0),
                ("month", 7.0),
                ("day", 16.0),
                ("num_days", 3.0),
                ("direction", 0.0),
            ],
            "new_day",
            21.0,
        ),
        (
            &[
                ("year", 2026.0),
                ("month", 7.0),
                ("day", 20.0),
                ("num_days", 2.0),
                ("direction", 1.0),
            ],
            "new_day",
            16.0,
        ),
        (
            &[
                ("year", 2026.0),
                ("month", 7.0),
                ("day", 11.0),
                ("num_days", 0.0),
                ("direction", 0.0),
            ],
            "new_day",
            11.0,
        ),
        (
            &[
                ("year", 2026.0),
                ("month", 7.0),
                ("day", 30.0),
                ("num_days", 3.0),
                ("direction", 0.0),
            ],
            "new_month",
            8.0,
        ),
        (
            &[
                ("year", 2028.0),
                ("month", 3.0),
                ("day", 2.0),
                ("num_days", 2.0),
                ("direction", 1.0),
            ],
            "new_day",
            29.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_workday"),
            "ExcelWorkday",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_workday: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_workday case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_workday case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "__none__");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_workday case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_workday case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_workday_intl_matches_test_cases() {
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
                ("year", 2026.0),
                ("month", 7.0),
                ("day", 11.0),
                ("num_days", 0.0),
                ("direction", 0.0),
                ("weekend_mask", 3.0),
            ],
            "new_day",
            11.0,
        ),
        (
            &[
                ("year", 2026.0),
                ("month", 7.0),
                ("day", 13.0),
                ("num_days", 3.0),
                ("direction", 0.0),
                ("weekend_mask", 3.0),
            ],
            "new_day",
            16.0,
        ),
        (
            &[
                ("year", 2026.0),
                ("month", 7.0),
                ("day", 16.0),
                ("num_days", 3.0),
                ("direction", 0.0),
                ("weekend_mask", 65.0),
            ],
            "new_day",
            21.0,
        ),
        (
            &[
                ("year", 2026.0),
                ("month", 7.0),
                ("day", 1.0),
                ("num_days", 5.0),
                ("direction", 1.0),
                ("weekend_mask", 3.0),
            ],
            "new_day",
            24.0,
        ),
        (
            &[
                ("year", 2026.0),
                ("month", 7.0),
                ("day", 1.0),
                ("num_days", 5.0),
                ("direction", 1.0),
                ("weekend_mask", 3.0),
            ],
            "new_month",
            6.0,
        ),
        (
            &[
                ("year", 2026.0),
                ("month", 12.0),
                ("day", 30.0),
                ("num_days", 3.0),
                ("direction", 0.0),
                ("weekend_mask", 3.0),
            ],
            "new_year",
            2027.0,
        ),
        (
            &[
                ("year", 2026.0),
                ("month", 12.0),
                ("day", 30.0),
                ("num_days", 3.0),
                ("direction", 0.0),
                ("weekend_mask", 3.0),
            ],
            "new_month",
            1.0,
        ),
        (
            &[
                ("year", 2026.0),
                ("month", 12.0),
                ("day", 30.0),
                ("num_days", 3.0),
                ("direction", 0.0),
                ("weekend_mask", 3.0),
            ],
            "new_day",
            4.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_workday_intl"),
            "ExcelWorkdayIntl",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_workday_intl: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_workday_intl case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_workday_intl case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "__none__");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_workday_intl case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_workday_intl case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}
