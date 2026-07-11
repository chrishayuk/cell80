//! Host-oracle tests for the excel-financial pack (`cell80/cells/excel-financial/*.rs`) — mechanically
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
fn excel_fv_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "fv" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "pmt" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "pv" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "rate" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "fv" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "pmt" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "pv" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "rate" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("rate", 0.0),
                ("nper", 5.0),
                ("pmt", -200.0),
                ("pv", -1000.0),
                ("due", 0.0),
            ],
            "fv",
            2000.0,
        ),
        (
            &[
                ("rate", 1.0),
                ("nper", 3.0),
                ("pmt", 0.0),
                ("pv", -10.0),
                ("due", 0.0),
            ],
            "fv",
            80.0,
        ),
        (
            &[
                ("rate", 1.0),
                ("nper", 2.0),
                ("pmt", -5.0),
                ("pv", 0.0),
                ("due", 1.0),
            ],
            "fv",
            30.0,
        ),
        (
            &[
                ("rate", 0.5),
                ("nper", 4.0),
                ("pmt", -20.0),
                ("pv", -50.0),
                ("due", 0.0),
            ],
            "fv",
            415.625,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell =
            cell80::StateCell::bind(&crate::common::cell_src("excel_fv"), "ExcelFv", None)
                .unwrap_or_else(|e| panic!("bind excel_fv: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_fv case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_fv case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "fv" | "pmt" | "pv" | "rate");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_fv case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_fv case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_pv_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "fv" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "pmt" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "pv" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "rate" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "fv" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "pmt" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "pv" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "rate" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("rate", 0.0),
                ("nper", 5.0),
                ("pmt", -100.0),
                ("fv", 0.0),
                ("pmt_type", 0.0),
            ],
            "pv",
            500.0,
        ),
        (
            &[
                ("rate", 1.0),
                ("nper", 3.0),
                ("pmt", 0.0),
                ("fv", 800.0),
                ("pmt_type", 0.0),
            ],
            "pv",
            -100.0,
        ),
        (
            &[
                ("rate", 1.0),
                ("nper", 2.0),
                ("pmt", -40.0),
                ("fv", 0.0),
                ("pmt_type", 1.0),
            ],
            "pv",
            60.0,
        ),
        (
            &[
                ("rate", 0.1),
                ("nper", 0.0),
                ("pmt", -50.0),
                ("fv", 200.0),
                ("pmt_type", 0.0),
            ],
            "pv",
            -200.0,
        ),
        (
            &[
                ("rate", -0.5),
                ("nper", 3.0),
                ("pmt", 10.0),
                ("fv", 0.0),
                ("pmt_type", 0.0),
            ],
            "pv",
            -140.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell =
            cell80::StateCell::bind(&crate::common::cell_src("excel_pv"), "ExcelPv", None)
                .unwrap_or_else(|e| panic!("bind excel_pv: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_pv case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_pv case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "fv" | "pmt" | "pv" | "rate");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_pv case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_pv case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_pmt_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "fv" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "pmt" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "pv" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "rate" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "fv" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "pmt" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "pv" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "rate" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("rate", 0.5),
                ("nper", 1.0),
                ("pv", 100.0),
                ("fv", 0.0),
                ("typ", 0.0),
            ],
            "pmt",
            -150.0,
        ),
        (
            &[
                ("rate", 0.25),
                ("nper", 1.0),
                ("pv", 200.0),
                ("fv", 50.0),
                ("typ", 1.0),
            ],
            "pmt",
            -240.0,
        ),
        (
            &[
                ("rate", 0.5),
                ("nper", 2.0),
                ("pv", 100.0),
                ("fv", 0.0),
                ("typ", 0.0),
            ],
            "pmt",
            -90.0,
        ),
        (
            &[
                ("rate", 0.0),
                ("nper", 4.0),
                ("pv", 1000.0),
                ("fv", 0.0),
                ("typ", 0.0),
            ],
            "pmt",
            -250.0,
        ),
        (
            &[
                ("rate", 0.5),
                ("nper", 1.0),
                ("pv", -100.0),
                ("fv", 0.0),
                ("typ", 0.0),
            ],
            "pmt",
            150.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(&crate::common::cell_src("excel_pmt"), "Pmt", None)
            .unwrap_or_else(|e| panic!("bind excel_pmt: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_pmt case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_pmt case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "fv" | "pmt" | "pv" | "rate");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_pmt case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_pmt case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_ispmt_matches_test_cases() {
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
                ("pv_mag", 1000000.0),
                ("pv_negative", 0.0),
                ("rate_bps", 1000.0),
                ("per", 1.0),
                ("nper", 12.0),
            ],
            "result_mag",
            91666.0,
        ),
        (
            &[
                ("pv_mag", 1000000.0),
                ("pv_negative", 0.0),
                ("rate_bps", 1000.0),
                ("per", 1.0),
                ("nper", 12.0),
            ],
            "result_negative",
            1.0,
        ),
        (
            &[
                ("pv_mag", 500000.0),
                ("pv_negative", 1.0),
                ("rate_bps", 833.0),
                ("per", 24.0),
                ("nper", 24.0),
            ],
            "result_mag",
            0.0,
        ),
        (
            &[
                ("pv_mag", 500000.0),
                ("pv_negative", 1.0),
                ("rate_bps", 833.0),
                ("per", 24.0),
                ("nper", 24.0),
            ],
            "result_negative",
            0.0,
        ),
        (
            &[
                ("pv_mag", 2000000.0),
                ("pv_negative", 1.0),
                ("rate_bps", 500.0),
                ("per", 3.0),
                ("nper", 10.0),
            ],
            "result_mag",
            70000.0,
        ),
        (
            &[
                ("pv_mag", 2000000.0),
                ("pv_negative", 1.0),
                ("rate_bps", 500.0),
                ("per", 3.0),
                ("nper", 10.0),
            ],
            "result_negative",
            0.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell =
            cell80::StateCell::bind(&crate::common::cell_src("excel_ispmt"), "ExcelIspmt", None)
                .unwrap_or_else(|e| panic!("bind excel_ispmt: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_ispmt case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_ispmt case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "__none__");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_ispmt case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_ispmt case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_effect_matches_test_cases() {
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
            &[("nominal_rate_bps", 1200.0), ("npery", 12.0)],
            "effective_rate_bps",
            1266.0,
        ),
        (
            &[("nominal_rate_bps", 500.0), ("npery", 1.0)],
            "effective_rate_bps",
            500.0,
        ),
        (
            &[("nominal_rate_bps", 1000.0), ("npery", 4.0)],
            "effective_rate_bps",
            1037.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_effect"),
            "ExcelEffect",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_effect: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_effect case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_effect case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "__none__");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_effect case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_effect case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_syd_matches_test_cases() {
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
                ("cost", 30000.0),
                ("salvage", 7500.0),
                ("life", 10.0),
                ("per", 1.0),
            ],
            "result",
            4090.0,
        ),
        (
            &[
                ("cost", 30000.0),
                ("salvage", 7500.0),
                ("life", 10.0),
                ("per", 10.0),
            ],
            "result",
            409.0,
        ),
        (
            &[
                ("cost", 10000.0),
                ("salvage", 1000.0),
                ("life", 5.0),
                ("per", 1.0),
            ],
            "result",
            3000.0,
        ),
        (
            &[
                ("cost", 100000.0),
                ("salvage", 10000.0),
                ("life", 5.0),
                ("per", 3.0),
            ],
            "result",
            18000.0,
        ),
        (
            &[
                ("cost", 5000.0),
                ("salvage", 1000.0),
                ("life", 1.0),
                ("per", 1.0),
            ],
            "result",
            4000.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell =
            cell80::StateCell::bind(&crate::common::cell_src("excel_syd"), "ExcelSyd", None)
                .unwrap_or_else(|e| panic!("bind excel_syd: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_syd case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_syd case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "__none__");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_syd case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_syd case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_tbillyield_matches_test_cases() {
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
                ("y1", 2008.0),
                ("m1", 2.0),
                ("d1", 1.0),
                ("y2", 2008.0),
                ("m2", 5.0),
                ("d2", 1.0),
                ("pr_cents", 9845.0),
            ],
            "yield_bps",
            629.0,
        ),
        (
            &[
                ("y1", 2021.0),
                ("m1", 1.0),
                ("d1", 1.0),
                ("y2", 2021.0),
                ("m2", 12.0),
                ("d2", 27.0),
                ("pr_cents", 9500.0),
            ],
            "yield_bps",
            526.0,
        ),
        (
            &[
                ("y1", 2023.0),
                ("m1", 11.0),
                ("d1", 15.0),
                ("y2", 2024.0),
                ("m2", 2.0),
                ("d2", 13.0),
                ("pr_cents", 9950.0),
            ],
            "yield_bps",
            201.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_tbillyield"),
            "ExcelTbillYield",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_tbillyield: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_tbillyield case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_tbillyield case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "__none__");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_tbillyield case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_tbillyield case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_ipmt_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "fv" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "interest_payment" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "pv" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "rate" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "fv" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "interest_payment" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "pv" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "rate" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("rate", 0.1),
                ("per", 1.0),
                ("nper", 3.0),
                ("pv", 1000.0),
                ("fv", 0.0),
                ("pmt_type", 0.0),
            ],
            "interest_payment",
            -100.0,
        ),
        (
            &[
                ("rate", 0.1),
                ("per", 2.0),
                ("nper", 3.0),
                ("pv", 1000.0),
                ("fv", 0.0),
                ("pmt_type", 0.0),
            ],
            "interest_payment",
            -69.78852,
        ),
        (
            &[
                ("rate", 0.08),
                ("per", 1.0),
                ("nper", 5.0),
                ("pv", 2000.0),
                ("fv", 0.0),
                ("pmt_type", 1.0),
            ],
            "interest_payment",
            0.0,
        ),
        (
            &[
                ("rate", 0.1),
                ("per", 2.0),
                ("nper", 3.0),
                ("pv", 1000.0),
                ("fv", 0.0),
                ("pmt_type", 1.0),
            ],
            "interest_payment",
            -63.44411,
        ),
        (
            &[
                ("rate", 0.0),
                ("per", 1.0),
                ("nper", 5.0),
                ("pv", 500.0),
                ("fv", 0.0),
                ("pmt_type", 0.0),
            ],
            "interest_payment",
            0.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell =
            cell80::StateCell::bind(&crate::common::cell_src("excel_ipmt"), "ExcelIpmt", None)
                .unwrap_or_else(|e| panic!("bind excel_ipmt: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_ipmt case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_ipmt case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "fv" | "interest_payment" | "pv" | "rate");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_ipmt case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_ipmt case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_ppmt_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "fv" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "ppmt" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "pv" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "rate" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "fv" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "ppmt" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "pv" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "rate" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("rate", 0.0066666667),
                ("per", 1.0),
                ("nper", 10.0),
                ("pv", 10000.0),
                ("fv", 0.0),
                ("pmt_type", 0.0),
            ],
            "ppmt",
            -970.3654,
        ),
        (
            &[
                ("rate", 0.0066666667),
                ("per", 2.0),
                ("nper", 10.0),
                ("pv", 10000.0),
                ("fv", 0.0),
                ("pmt_type", 0.0),
            ],
            "ppmt",
            -976.8345,
        ),
        (
            &[
                ("rate", 0.0066666667),
                ("per", 1.0),
                ("nper", 10.0),
                ("pv", 10000.0),
                ("fv", 0.0),
                ("pmt_type", 1.0),
            ],
            "ppmt",
            -1030.1643,
        ),
        (
            &[
                ("rate", 0.0),
                ("per", 3.0),
                ("nper", 5.0),
                ("pv", 1000.0),
                ("fv", 0.0),
                ("pmt_type", 0.0),
            ],
            "ppmt",
            -200.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell =
            cell80::StateCell::bind(&crate::common::cell_src("excel_ppmt"), "ExcelPpmt", None)
                .unwrap_or_else(|e| panic!("bind excel_ppmt: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_ppmt case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_ppmt case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "fv" | "ppmt" | "pv" | "rate");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_ppmt case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_ppmt case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_nominal_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "effect_rate" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "nominal_rate" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "effect_rate" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "nominal_rate" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[("effect_rate", 0.1), ("npery", 1.0)],
            "nominal_rate",
            0.100000024,
        ),
        (
            &[("effect_rate", 0.1025), ("npery", 2.0)],
            "nominal_rate",
            0.099999905,
        ),
        (
            &[("effect_rate", 0.10381289), ("npery", 4.0)],
            "nominal_rate",
            0.099999905,
        ),
        (
            &[("effect_rate", 0.12682503), ("npery", 12.0)],
            "nominal_rate",
            0.119999886,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_nominal"),
            "ExcelNominal",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_nominal: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_nominal case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_nominal case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "effect_rate" | "nominal_rate");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_nominal case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_nominal case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_rate_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "fv" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "guess" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "pmt" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "pv" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "rate" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "fv" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "guess" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "pmt" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "pv" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "rate" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("nper", 1.0),
                ("pmt", -1050.0),
                ("pv", 1000.0),
                ("fv", 0.0),
                ("typ", 0.0),
                ("guess", 0.1),
            ],
            "rate",
            0.05,
        ),
        (
            &[
                ("nper", 2.0),
                ("pmt", -72.0),
                ("pv", 110.0),
                ("fv", 0.0),
                ("typ", 0.0),
                ("guess", 0.1),
            ],
            "rate",
            0.2,
        ),
        (
            &[
                ("nper", 2.0),
                ("pmt", -60.0),
                ("pv", 110.0),
                ("fv", 0.0),
                ("typ", 1.0),
                ("guess", 0.1),
            ],
            "rate",
            0.2,
        ),
        (
            &[
                ("nper", 3.0),
                ("pmt", -100.0),
                ("pv", 0.0),
                ("fv", 331.0),
                ("typ", 0.0),
                ("guess", 0.1),
            ],
            "rate",
            0.1,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell =
            cell80::StateCell::bind(&crate::common::cell_src("excel_rate"), "ExcelRate", None)
                .unwrap_or_else(|e| panic!("bind excel_rate: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_rate case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_rate case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "fv" | "guess" | "pmt" | "pv" | "rate");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_rate case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_rate case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_cumipmt_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "cum_interest" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "pv" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "rate" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "cum_interest" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "pv" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "rate" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("rate", 0.1),
                ("nper", 2.0),
                ("pv", 1000.0),
                ("start_period", 1.0),
                ("end_period", 2.0),
                ("pay_type", 0.0),
            ],
            "cum_interest",
            -152.380952,
        ),
        (
            &[
                ("rate", 0.1),
                ("nper", 2.0),
                ("pv", 1000.0),
                ("start_period", 2.0),
                ("end_period", 2.0),
                ("pay_type", 0.0),
            ],
            "cum_interest",
            -52.380952,
        ),
        (
            &[
                ("rate", 0.1),
                ("nper", 2.0),
                ("pv", 1000.0),
                ("start_period", 1.0),
                ("end_period", 2.0),
                ("pay_type", 1.0),
            ],
            "cum_interest",
            -47.619048,
        ),
        (
            &[
                ("rate", 0.1),
                ("nper", 2.0),
                ("pv", 1000.0),
                ("start_period", 1.0),
                ("end_period", 1.0),
                ("pay_type", 1.0),
            ],
            "cum_interest",
            0.0,
        ),
        (
            &[
                ("rate", 0.1),
                ("nper", 3.0),
                ("pv", 1000.0),
                ("start_period", 2.0),
                ("end_period", 3.0),
                ("pay_type", 0.0),
            ],
            "cum_interest",
            -106.344411,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_cumipmt"),
            "ExcelCumipmt",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_cumipmt: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_cumipmt case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_cumipmt case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "cum_interest" | "pv" | "rate");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_cumipmt case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_cumipmt case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_cumprinc_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "cum_principal" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "pv" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "rate" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "cum_principal" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "pv" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "rate" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("rate", 0.1),
                ("nper", 4.0),
                ("pv", 1000.0),
                ("start_period", 1.0),
                ("end_period", 2.0),
                ("pmt_type", 0.0),
            ],
            "cum_principal",
            -452.4886878,
        ),
        (
            &[
                ("rate", 0.1),
                ("nper", 4.0),
                ("pv", 1000.0),
                ("start_period", 1.0),
                ("end_period", 2.0),
                ("pmt_type", 1.0),
            ],
            "cum_principal",
            -502.2624434,
        ),
        (
            &[
                ("rate", 0.1),
                ("nper", 4.0),
                ("pv", 1000.0),
                ("start_period", 1.0),
                ("end_period", 1.0),
                ("pmt_type", 0.0),
            ],
            "cum_principal",
            -215.4708037,
        ),
        (
            &[
                ("rate", 0.1),
                ("nper", 4.0),
                ("pv", 1000.0),
                ("start_period", 1.0),
                ("end_period", 4.0),
                ("pmt_type", 0.0),
            ],
            "cum_principal",
            -1000.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_cumprinc"),
            "ExcelCumprinc",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_cumprinc: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_cumprinc case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_cumprinc case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "cum_principal" | "pv" | "rate");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_cumprinc case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_cumprinc case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_db_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "cost" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "depreciation" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "salvage" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "cost" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "depreciation" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "salvage" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("cost", 800.0),
                ("salvage", 200.0),
                ("life", 1.0),
                ("period", 1.0),
                ("month", 12.0),
            ],
            "depreciation",
            600.0,
        ),
        (
            &[
                ("cost", 800.0),
                ("salvage", 200.0),
                ("life", 1.0),
                ("period", 2.0),
                ("month", 6.0),
            ],
            "depreciation",
            187.5,
        ),
        (
            &[
                ("cost", 1000.0),
                ("salvage", 250.0),
                ("life", 2.0),
                ("period", 2.0),
                ("month", 12.0),
            ],
            "depreciation",
            250.0,
        ),
        (
            &[
                ("cost", 800.0),
                ("salvage", 100.0),
                ("life", 3.0),
                ("period", 3.0),
                ("month", 12.0),
            ],
            "depreciation",
            100.0,
        ),
        (
            &[
                ("cost", 800.0),
                ("salvage", 100.0),
                ("life", 3.0),
                ("period", 4.0),
                ("month", 6.0),
            ],
            "depreciation",
            37.5,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell =
            cell80::StateCell::bind(&crate::common::cell_src("excel_db"), "ExcelDb", None)
                .unwrap_or_else(|e| panic!("bind excel_db: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_db case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_db case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "cost" | "depreciation" | "salvage");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_db case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_db case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_ddb_matches_test_cases() {
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
                ("cost_cents", 240000.0),
                ("salvage_cents", 30000.0),
                ("life", 10.0),
                ("period", 1.0),
                ("factor", 2.0),
            ],
            "depreciation_cents",
            48000.0,
        ),
        (
            &[
                ("cost_cents", 240000.0),
                ("salvage_cents", 30000.0),
                ("life", 10.0),
                ("period", 2.0),
                ("factor", 2.0),
            ],
            "depreciation_cents",
            38400.0,
        ),
        (
            &[
                ("cost_cents", 100000.0),
                ("salvage_cents", 0.0),
                ("life", 1.0),
                ("period", 1.0),
                ("factor", 2.0),
            ],
            "depreciation_cents",
            100000.0,
        ),
        (
            &[
                ("cost_cents", 100000.0),
                ("salvage_cents", 80000.0),
                ("life", 2.0),
                ("period", 2.0),
                ("factor", 2.0),
            ],
            "depreciation_cents",
            0.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell =
            cell80::StateCell::bind(&crate::common::cell_src("excel_ddb"), "ExcelDdb", None)
                .unwrap_or_else(|e| panic!("bind excel_ddb: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_ddb case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_ddb case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "__none__");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_ddb case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_ddb case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_vdb_matches_test_cases() {
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
                ("cost", 240000.0),
                ("salvage", 30000.0),
                ("life", 10.0),
                ("start_permille", 0.0),
                ("end_permille", 875.0),
                ("factor_x100", 200.0),
                ("no_switch", 0.0),
            ],
            "depreciation",
            42000.0,
        ),
        (
            &[
                ("cost", 240000.0),
                ("salvage", 30000.0),
                ("life", 10.0),
                ("start_permille", 0.0),
                ("end_permille", 2000.0),
                ("factor_x100", 200.0),
                ("no_switch", 0.0),
            ],
            "depreciation",
            86400.0,
        ),
        (
            &[
                ("cost", 120000.0),
                ("salvage", 0.0),
                ("life", 4.0),
                ("start_permille", 3000.0),
                ("end_permille", 4000.0),
                ("factor_x100", 200.0),
                ("no_switch", 0.0),
            ],
            "depreciation",
            15000.0,
        ),
        (
            &[
                ("cost", 120000.0),
                ("salvage", 0.0),
                ("life", 4.0),
                ("start_permille", 3000.0),
                ("end_permille", 4000.0),
                ("factor_x100", 200.0),
                ("no_switch", 1.0),
            ],
            "depreciation",
            7500.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell =
            cell80::StateCell::bind(&crate::common::cell_src("excel_vdb"), "ExcelVdb", None)
                .unwrap_or_else(|e| panic!("bind excel_vdb: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_vdb case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_vdb case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "__none__");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_vdb case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_vdb case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_dollarde_matches_test_cases() {
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
                ("whole", 1.0),
                ("frac_digits_value", 2.0),
                ("fraction", 16.0),
            ],
            "num",
            9.0,
        ),
        (
            &[
                ("whole", 1.0),
                ("frac_digits_value", 2.0),
                ("fraction", 16.0),
            ],
            "den",
            8.0,
        ),
        (
            &[
                ("whole", 0.0),
                ("frac_digits_value", 0.0),
                ("fraction", 32.0),
            ],
            "num",
            0.0,
        ),
        (
            &[
                ("whole", 0.0),
                ("frac_digits_value", 0.0),
                ("fraction", 32.0),
            ],
            "den",
            1.0,
        ),
        (
            &[
                ("whole", 2.0),
                ("frac_digits_value", 4.0),
                ("fraction", 8.0),
            ],
            "num",
            5.0,
        ),
        (
            &[
                ("whole", 2.0),
                ("frac_digits_value", 4.0),
                ("fraction", 8.0),
            ],
            "den",
            2.0,
        ),
        (
            &[
                ("whole", 3.0),
                ("frac_digits_value", 64.0),
                ("fraction", 128.0),
            ],
            "num",
            7.0,
        ),
        (
            &[
                ("whole", 3.0),
                ("frac_digits_value", 64.0),
                ("fraction", 128.0),
            ],
            "den",
            2.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_dollarde"),
            "ExcelDollarde",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_dollarde: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_dollarde case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_dollarde case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "__none__");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_dollarde case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_dollarde case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_dollarfr_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "decimal" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "result" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "decimal" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "result" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (&[("decimal", 1.125), ("fraction", 16.0)], "result", 1.02),
        (&[("decimal", 1.125), ("fraction", 8.0)], "result", 1.1),
        (
            &[("decimal", 102.15625), ("fraction", 32.0)],
            "result",
            102.05,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell =
            cell80::StateCell::bind(&crate::common::cell_src("excel_dollarfr"), "DollarFr", None)
                .unwrap_or_else(|e| panic!("bind excel_dollarfr: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_dollarfr case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_dollarfr case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "decimal" | "result");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_dollarfr case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_dollarfr case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_tbilleq_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "bey" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "discount" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "bey" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "discount" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("sy", 2024.0),
                ("sm", 1.0),
                ("sd", 1.0),
                ("my", 2024.0),
                ("mm", 4.0),
                ("md", 2.0),
                ("discount", 0.028),
            ],
            "bey",
            0.0285934912,
        ),
        (
            &[
                ("sy", 2023.0),
                ("sm", 1.0),
                ("sd", 1.0),
                ("my", 2023.0),
                ("mm", 7.0),
                ("md", 2.0),
                ("discount", 0.05),
            ],
            "bey",
            0.05200912,
        ),
        (
            &[
                ("sy", 1990.0),
                ("sm", 6.0),
                ("sd", 7.0),
                ("my", 1991.0),
                ("mm", 6.0),
                ("md", 6.0),
                ("discount", 0.0765),
            ],
            "bey",
            0.08237324412482,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_tbilleq"),
            "ExcelTbilleq",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_tbilleq: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_tbilleq case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_tbilleq case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "bey" | "discount");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_tbilleq case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_tbilleq case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_accrint_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "accrued_interest" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "dsm_over_b" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "par" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "rate" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "accrued_interest" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "dsm_over_b" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "par" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "rate" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("rate", 0.0625),
                ("par", 1024.0),
                ("frequency", 2.0),
                ("dsm_over_b", 0.5),
            ],
            "accrued_interest",
            16.0,
        ),
        (
            &[
                ("rate", 0.125),
                ("par", 2000.0),
                ("frequency", 4.0),
                ("dsm_over_b", 0.75),
            ],
            "accrued_interest",
            46.875,
        ),
        (
            &[
                ("rate", 0.0625),
                ("par", 800.0),
                ("frequency", 1.0),
                ("dsm_over_b", 0.25),
            ],
            "accrued_interest",
            12.5,
        ),
        (
            &[
                ("rate", 0.07),
                ("par", 1000.0),
                ("frequency", 2.0),
                ("dsm_over_b", 0.0),
            ],
            "accrued_interest",
            0.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_accrint"),
            "ExcelAccrint",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_accrint: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_accrint case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_accrint case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(
            *expected_field,
            "accrued_interest" | "dsm_over_b" | "par" | "rate"
        );
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_accrint case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_accrint case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_accrintm_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "accrued_interest" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "par" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "rate" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
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
            "accrued_interest" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "par" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "rate" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "year_frac" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[("par", 1000.0), ("rate", 0.05), ("year_frac", 0.5)],
            "accrued_interest",
            25.0,
        ),
        (
            &[("par", 4000.0), ("rate", 0.05), ("year_frac", 0.25)],
            "accrued_interest",
            50.0,
        ),
        (
            &[("par", 5000.0), ("rate", 0.1), ("year_frac", 0.75)],
            "accrued_interest",
            375.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_accrintm"),
            "ExcelAccrintm",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_accrintm: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_accrintm case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_accrintm case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(
            *expected_field,
            "accrued_interest" | "par" | "rate" | "year_frac"
        );
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_accrintm case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_accrintm case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_coupdaybs_matches_test_cases() {
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
                ("sy", 2024.0),
                ("sm", 3.0),
                ("sd", 31.0),
                ("my", 2024.0),
                ("mm", 8.0),
                ("md", 15.0),
                ("frequency", 2.0),
                ("basis", 0.0),
            ],
            "days_bs",
            46.0,
        ),
        (
            &[
                ("sy", 2024.0),
                ("sm", 3.0),
                ("sd", 31.0),
                ("my", 2024.0),
                ("mm", 8.0),
                ("md", 15.0),
                ("frequency", 2.0),
                ("basis", 4.0),
            ],
            "days_bs",
            45.0,
        ),
        (
            &[
                ("sy", 2023.0),
                ("sm", 4.0),
                ("sd", 10.0),
                ("my", 2023.0),
                ("mm", 6.0),
                ("md", 30.0),
                ("frequency", 1.0),
                ("basis", 2.0),
            ],
            "days_bs",
            284.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_coupdaybs"),
            "ExcelCoupdaybs",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_coupdaybs: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_coupdaybs case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_coupdaybs case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "__none__");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_coupdaybs case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_coupdaybs case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_coupdays_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "days" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "days" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("settle_y", 2026.0),
                ("settle_m", 1.0),
                ("settle_d", 15.0),
                ("mat_y", 2030.0),
                ("mat_m", 6.0),
                ("mat_d", 30.0),
                ("frequency", 2.0),
                ("basis", 0.0),
            ],
            "days",
            180.0,
        ),
        (
            &[
                ("settle_y", 2027.0),
                ("settle_m", 3.0),
                ("settle_d", 1.0),
                ("mat_y", 2028.0),
                ("mat_m", 9.0),
                ("mat_d", 1.0),
                ("frequency", 2.0),
                ("basis", 3.0),
            ],
            "days",
            182.5,
        ),
        (
            &[
                ("settle_y", 2026.0),
                ("settle_m", 3.0),
                ("settle_d", 15.0),
                ("mat_y", 2026.0),
                ("mat_m", 12.0),
                ("mat_d", 31.0),
                ("frequency", 2.0),
                ("basis", 1.0),
            ],
            "days",
            181.0,
        ),
        (
            &[
                ("settle_y", 2025.0),
                ("settle_m", 1.0),
                ("settle_d", 1.0),
                ("mat_y", 2026.0),
                ("mat_m", 1.0),
                ("mat_d", 1.0),
                ("frequency", 4.0),
                ("basis", 2.0),
            ],
            "days",
            90.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_coupdays"),
            "ExcelCoupdays",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_coupdays: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_coupdays case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_coupdays case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "days");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_coupdays case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_coupdays case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_coupnum_matches_test_cases() {
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
                ("sy", 2008.0),
                ("sm", 2.0),
                ("sd", 1.0),
                ("my", 2011.0),
                ("mm", 1.0),
                ("md", 1.0),
                ("frequency", 2.0),
            ],
            "num_coupons",
            6.0,
        ),
        (
            &[
                ("sy", 2020.0),
                ("sm", 6.0),
                ("sd", 15.0),
                ("my", 2025.0),
                ("mm", 6.0),
                ("md", 15.0),
                ("frequency", 1.0),
            ],
            "num_coupons",
            5.0,
        ),
        (
            &[
                ("sy", 2019.0),
                ("sm", 1.0),
                ("sd", 10.0),
                ("my", 2020.0),
                ("mm", 1.0),
                ("md", 10.0),
                ("frequency", 4.0),
            ],
            "num_coupons",
            4.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_coupnum"),
            "ExcelCoupnum",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_coupnum: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_coupnum case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_coupnum case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "__none__");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_coupnum case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_coupnum case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_couppcd_matches_test_cases() {
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
                ("sy", 2024.0),
                ("sm", 3.0),
                ("sd", 10.0),
                ("my", 2025.0),
                ("mm", 1.0),
                ("md", 15.0),
                ("frequency", 2.0),
            ],
            "pcd_y",
            2024.0,
        ),
        (
            &[
                ("sy", 2024.0),
                ("sm", 3.0),
                ("sd", 10.0),
                ("my", 2025.0),
                ("mm", 1.0),
                ("md", 15.0),
                ("frequency", 2.0),
            ],
            "pcd_m",
            1.0,
        ),
        (
            &[
                ("sy", 2024.0),
                ("sm", 3.0),
                ("sd", 10.0),
                ("my", 2025.0),
                ("mm", 1.0),
                ("md", 15.0),
                ("frequency", 2.0),
            ],
            "pcd_d",
            15.0,
        ),
        (
            &[
                ("sy", 2024.0),
                ("sm", 3.0),
                ("sd", 15.0),
                ("my", 2024.0),
                ("mm", 5.0),
                ("md", 31.0),
                ("frequency", 4.0),
            ],
            "pcd_m",
            2.0,
        ),
        (
            &[
                ("sy", 2024.0),
                ("sm", 3.0),
                ("sd", 15.0),
                ("my", 2024.0),
                ("mm", 5.0),
                ("md", 31.0),
                ("frequency", 4.0),
            ],
            "pcd_d",
            29.0,
        ),
        (
            &[
                ("sy", 2023.0),
                ("sm", 8.0),
                ("sd", 1.0),
                ("my", 2024.0),
                ("mm", 5.0),
                ("md", 31.0),
                ("frequency", 4.0),
            ],
            "pcd_y",
            2023.0,
        ),
        (
            &[
                ("sy", 2023.0),
                ("sm", 8.0),
                ("sd", 1.0),
                ("my", 2024.0),
                ("mm", 5.0),
                ("md", 31.0),
                ("frequency", 4.0),
            ],
            "pcd_m",
            5.0,
        ),
        (
            &[
                ("sy", 2023.0),
                ("sm", 8.0),
                ("sd", 1.0),
                ("my", 2024.0),
                ("mm", 5.0),
                ("md", 31.0),
                ("frequency", 4.0),
            ],
            "pcd_d",
            29.0,
        ),
        (
            &[
                ("sy", 2028.0),
                ("sm", 1.0),
                ("sd", 1.0),
                ("my", 2030.0),
                ("mm", 6.0),
                ("md", 30.0),
                ("frequency", 1.0),
            ],
            "pcd_y",
            2027.0,
        ),
        (
            &[
                ("sy", 2028.0),
                ("sm", 1.0),
                ("sd", 1.0),
                ("my", 2030.0),
                ("mm", 6.0),
                ("md", 30.0),
                ("frequency", 1.0),
            ],
            "pcd_m",
            6.0,
        ),
        (
            &[
                ("sy", 2026.0),
                ("sm", 3.0),
                ("sd", 30.0),
                ("my", 2026.0),
                ("mm", 9.0),
                ("md", 30.0),
                ("frequency", 2.0),
            ],
            "pcd_m",
            3.0,
        ),
        (
            &[
                ("sy", 2026.0),
                ("sm", 3.0),
                ("sd", 30.0),
                ("my", 2026.0),
                ("mm", 9.0),
                ("md", 30.0),
                ("frequency", 2.0),
            ],
            "pcd_d",
            30.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_couppcd"),
            "ExcelCouppcd",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_couppcd: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_couppcd case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_couppcd case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "__none__");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_couppcd case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_couppcd case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_amordegrc_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "cost" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "depreciation" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "dsm_over_b" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "rate" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "salvage" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "cost" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "depreciation" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "dsm_over_b" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "rate" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "salvage" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("cost", 1000.0),
                ("salvage", 100.0),
                ("dsm_over_b", 0.3),
                ("period", 0.0),
                ("rate", 0.5),
            ],
            "depreciation",
            150.0,
        ),
        (
            &[
                ("cost", 2000.0),
                ("salvage", 100.0),
                ("dsm_over_b", 0.5),
                ("period", 2.0),
                ("rate", 0.25),
            ],
            "depreciation",
            381.0,
        ),
        (
            &[
                ("cost", 3000.0),
                ("salvage", 200.0),
                ("dsm_over_b", 0.4),
                ("period", 1.0),
                ("rate", 0.18),
            ],
            "depreciation",
            924.0,
        ),
        (
            &[
                ("cost", 2000.0),
                ("salvage", 200.0),
                ("dsm_over_b", 0.5),
                ("period", 0.0),
                ("rate", 0.1),
            ],
            "depreciation",
            250.0,
        ),
        (
            &[
                ("cost", 1000.0),
                ("salvage", 500.0),
                ("dsm_over_b", 1.0),
                ("period", 1.0),
                ("rate", 0.4),
            ],
            "depreciation",
            300.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_amordegrc"),
            "ExcelAmordegrc",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_amordegrc: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_amordegrc case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_amordegrc case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(
            *expected_field,
            "cost" | "depreciation" | "dsm_over_b" | "rate" | "salvage"
        );
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_amordegrc case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_amordegrc case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_amorlinc_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "cost" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "depreciation" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "dp_fp_over_b" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "rate" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "salvage" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "cost" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "depreciation" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "dp_fp_over_b" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "rate" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "salvage" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("cost", 5000.0),
                ("salvage", 500.0),
                ("rate", 0.25),
                ("dp_fp_over_b", 0.75),
                ("period", 0.0),
            ],
            "depreciation",
            937.5,
        ),
        (
            &[
                ("cost", 5000.0),
                ("salvage", 500.0),
                ("rate", 0.25),
                ("dp_fp_over_b", 0.75),
                ("period", 1.0),
            ],
            "depreciation",
            1250.0,
        ),
        (
            &[
                ("cost", 5000.0),
                ("salvage", 500.0),
                ("rate", 0.25),
                ("dp_fp_over_b", 0.75),
                ("period", 2.0),
            ],
            "depreciation",
            1250.0,
        ),
        (
            &[
                ("cost", 5000.0),
                ("salvage", 500.0),
                ("rate", 0.25),
                ("dp_fp_over_b", 0.75),
                ("period", 3.0),
            ],
            "depreciation",
            1062.5,
        ),
        (
            &[
                ("cost", 5000.0),
                ("salvage", 500.0),
                ("rate", 0.25),
                ("dp_fp_over_b", 0.75),
                ("period", 4.0),
            ],
            "depreciation",
            0.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_amorlinc"),
            "ExcelAmorlinc",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_amorlinc: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_amorlinc case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_amorlinc case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(
            *expected_field,
            "cost" | "depreciation" | "dp_fp_over_b" | "rate" | "salvage"
        );
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_amorlinc case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_amorlinc case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_disc_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "disc" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "dsm_over_b" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "pr" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "redemption" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "disc" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "dsm_over_b" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "pr" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "redemption" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[("pr", 95.0), ("redemption", 100.0), ("dsm_over_b", 0.5)],
            "disc",
            0.1,
        ),
        (
            &[("pr", 90.0), ("redemption", 100.0), ("dsm_over_b", 0.25)],
            "disc",
            0.4,
        ),
        (
            &[("pr", 98.0), ("redemption", 100.0), ("dsm_over_b", 1.0)],
            "disc",
            0.02,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell =
            cell80::StateCell::bind(&crate::common::cell_src("excel_disc"), "ExcelDisc", None)
                .unwrap_or_else(|e| panic!("bind excel_disc: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_disc case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_disc case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "disc" | "dsm_over_b" | "pr" | "redemption");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_disc case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_disc case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_intrate_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "dim_over_b" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "investment" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "rate" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "redemption" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "dim_over_b" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "investment" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "rate" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "redemption" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("investment", 100.0),
                ("redemption", 110.0),
                ("dim_over_b", 0.5),
            ],
            "rate",
            0.2,
        ),
        (
            &[
                ("investment", 1000000.0),
                ("redemption", 1014420.0),
                ("dim_over_b", 0.25),
            ],
            "rate",
            0.05768,
        ),
        (
            &[
                ("investment", 500.0),
                ("redemption", 475.0),
                ("dim_over_b", 1.0),
            ],
            "rate",
            -0.05,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_intrate"),
            "ExcelIntrate",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_intrate: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_intrate case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_intrate case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(
            *expected_field,
            "dim_over_b" | "investment" | "rate" | "redemption"
        );
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_intrate case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_intrate case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_mduration_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "coupon" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "mduration" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "yld" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "coupon" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "mduration" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "yld" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("num_periods", 1.0),
                ("coupon", 0.0),
                ("yld", 0.1),
                ("frequency", 1.0),
            ],
            "mduration",
            0.9090909,
        ),
        (
            &[
                ("num_periods", 2.0),
                ("coupon", 0.08),
                ("yld", 0.1),
                ("frequency", 2.0),
            ],
            "mduration",
            0.9338966,
        ),
        (
            &[
                ("num_periods", 3.0),
                ("coupon", 0.05),
                ("yld", 0.05),
                ("frequency", 1.0),
            ],
            "mduration",
            2.723248,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_mduration"),
            "ExcelMduration",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_mduration: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_mduration case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_mduration case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "coupon" | "mduration" | "yld");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_mduration case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_mduration case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_oddfyield_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "dfc_over_e" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "pr" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "rate" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "redemption" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "yld" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "dfc_over_e" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "pr" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "rate" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "redemption" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "yld" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("rate", 0.08),
                ("pr", 100.0),
                ("redemption", 100.0),
                ("frequency", 2.0),
                ("dfc_over_e", 1.0),
                ("n_periods", 3.0),
            ],
            "yld",
            0.0761635303,
        ),
        (
            &[
                ("rate", 0.06),
                ("pr", 98.543689),
                ("redemption", 100.0),
                ("frequency", 2.0),
                ("dfc_over_e", 0.5),
                ("n_periods", 1.0),
            ],
            "yld",
            0.0587413311,
        ),
        (
            &[
                ("rate", 0.05),
                ("pr", 102.380952),
                ("redemption", 100.0),
                ("frequency", 1.0),
                ("dfc_over_e", 1.5),
                ("n_periods", 2.0),
            ],
            "yld",
            0.0531644821,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_oddfyield"),
            "ExcelOddfyield",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_oddfyield: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_oddfyield case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_oddfyield case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(
            *expected_field,
            "dfc_over_e" | "pr" | "rate" | "redemption" | "yld"
        );
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_oddfyield case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_oddfyield case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_oddlyield_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "a_over_e" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "dsc_over_e" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "oddlyield" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "pr" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "rate" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "redemption" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "a_over_e" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "dsc_over_e" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "oddlyield" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "pr" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "rate" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "redemption" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("rate", 0.08),
                ("pr", 100.0),
                ("redemption", 100.0),
                ("frequency", 2.0),
                ("a_over_e", 0.5),
                ("dsc_over_e", 0.5),
            ],
            "oddlyield",
            0.078431373,
        ),
        (
            &[
                ("rate", 0.06),
                ("pr", 98.0),
                ("redemption", 100.0),
                ("frequency", 2.0),
                ("a_over_e", 0.0),
                ("dsc_over_e", 1.0),
            ],
            "oddlyield",
            0.102040816,
        ),
        (
            &[
                ("rate", 0.05),
                ("pr", 99.0),
                ("redemption", 100.0),
                ("frequency", 1.0),
                ("a_over_e", 0.3),
                ("dsc_over_e", 0.2),
            ],
            "oddlyield",
            0.099502488,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_oddlyield"),
            "ExcelOddlyield",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_oddlyield: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_oddlyield case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_oddlyield case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(
            *expected_field,
            "a_over_e" | "dsc_over_e" | "oddlyield" | "pr" | "rate" | "redemption"
        );
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_oddlyield case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_oddlyield case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_pricedisc_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "discount" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "dsm_over_b" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "price" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "redemption" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "discount" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "dsm_over_b" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "price" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "redemption" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("discount", 0.05),
                ("redemption", 100.0),
                ("dsm_over_b", 0.5),
            ],
            "price",
            97.5,
        ),
        (
            &[
                ("discount", 0.0525),
                ("redemption", 100.0),
                ("dsm_over_b", 0.03888889),
            ],
            "price",
            99.795833,
        ),
        (
            &[
                ("discount", 0.06),
                ("redemption", 1000.0),
                ("dsm_over_b", 0.2777778),
            ],
            "price",
            983.3333,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_pricedisc"),
            "ExcelPricedisc",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_pricedisc: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_pricedisc case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_pricedisc case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(
            *expected_field,
            "discount" | "dsm_over_b" | "price" | "redemption"
        );
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_pricedisc case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_pricedisc case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_pricemat_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "a_over_b" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "dim_over_b" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "dsm_over_b" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "price" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "rate" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "yld" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "a_over_b" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "dim_over_b" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "dsm_over_b" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "price" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "rate" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "yld" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("rate", 0.08),
                ("yld", 0.1),
                ("dim_over_b", 0.5),
                ("dsm_over_b", 0.25),
                ("a_over_b", 0.25),
            ],
            "price",
            99.4634146,
        ),
        (
            &[
                ("rate", 0.061),
                ("yld", 0.061),
                ("dim_over_b", 0.42222222),
                ("dsm_over_b", 0.16111111),
                ("a_over_b", 0.26111111),
            ],
            "price",
            99.9844989,
        ),
        (
            &[
                ("rate", 0.0),
                ("yld", 0.05),
                ("dim_over_b", 0.9),
                ("dsm_over_b", 0.5),
                ("a_over_b", 0.4),
            ],
            "price",
            97.5609756,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_pricemat"),
            "ExcelPricemat",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_pricemat: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_pricemat case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_pricemat case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(
            *expected_field,
            "a_over_b" | "dim_over_b" | "dsm_over_b" | "price" | "rate" | "yld"
        );
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_pricemat case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_pricemat case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_received_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "discount" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "dsm_over_b" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "investment" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "received" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "discount" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "dsm_over_b" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "investment" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "received" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("investment", 1000000.0),
                ("discount", 0.05),
                ("dsm_over_b", 0.5),
            ],
            "received",
            1025641.0256410256,
        ),
        (
            &[
                ("investment", 500000.0),
                ("discount", 0.08),
                ("dsm_over_b", 1.0),
            ],
            "received",
            543478.2608695652,
        ),
        (
            &[
                ("investment", 750000.0),
                ("discount", 0.1),
                ("dsm_over_b", 0.25),
            ],
            "received",
            769230.7692307692,
        ),
        (
            &[
                ("investment", 1000000.0),
                ("discount", 0.045),
                ("dsm_over_b", 0.2555555556),
            ],
            "received",
            1011633.7886,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_received"),
            "ExcelReceived",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_received: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_received case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_received case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(
            *expected_field,
            "discount" | "dsm_over_b" | "investment" | "received"
        );
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_received case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_received case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_yield_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "dsm_over_b" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "pr" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "rate" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "redemption" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "yld" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "dsm_over_b" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "pr" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "rate" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "redemption" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "yld" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("rate", 0.08),
                ("pr", 101.4634146),
                ("redemption", 100.0),
                ("frequency", 2.0),
                ("num_coupons", 1.0),
                ("dsm_over_b", 0.0),
            ],
            "yld",
            0.05,
        ),
        (
            &[
                ("rate", 0.06),
                ("pr", 99.0501529),
                ("redemption", 100.0),
                ("frequency", 2.0),
                ("num_coupons", 2.0),
                ("dsm_over_b", 0.0),
            ],
            "yld",
            0.07,
        ),
        (
            &[
                ("rate", 0.08),
                ("pr", 99.4634146),
                ("redemption", 100.0),
                ("frequency", 2.0),
                ("num_coupons", 1.0),
                ("dsm_over_b", 0.5),
            ],
            "yld",
            0.05,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell =
            cell80::StateCell::bind(&crate::common::cell_src("excel_yield"), "ExcelYield", None)
                .unwrap_or_else(|e| panic!("bind excel_yield: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(20000000)
            .unwrap_or_else(|e| panic!("run excel_yield case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_yield case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(
            *expected_field,
            "dsm_over_b" | "pr" | "rate" | "redemption" | "yld"
        );
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_yield case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_yield case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_yieldmat_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "a_over_b" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "dim_over_b" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "dsm_over_b" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "pr" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "rate" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "yld" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "a_over_b" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "dim_over_b" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "dsm_over_b" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "pr" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "rate" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "yld" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("rate", 0.08),
                ("pr", 96.0),
                ("dim_over_b", 1.0),
                ("dsm_over_b", 0.5),
                ("a_over_b", 0.5),
            ],
            "yld",
            0.16,
        ),
        (
            &[
                ("rate", 0.05),
                ("pr", 98.0),
                ("dim_over_b", 0.75),
                ("dsm_over_b", 0.25),
                ("a_over_b", 0.5),
            ],
            "yld",
            0.1293532,
        ),
        (
            &[
                ("rate", 0.06),
                ("pr", 97.5),
                ("dim_over_b", 0.9),
                ("dsm_over_b", 0.6),
                ("a_over_b", 0.3),
            ],
            "yld",
            0.1023833,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_yieldmat"),
            "ExcelYieldmat",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_yieldmat: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_yieldmat case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_yieldmat case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(
            *expected_field,
            "a_over_b" | "dim_over_b" | "dsm_over_b" | "pr" | "rate" | "yld"
        );
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_yieldmat case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_yieldmat case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn excel_tbillprice_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "discount" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "dsm_over_360" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "price" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "discount" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "dsm_over_360" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "price" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (&[("discount", 0.08), ("dsm_over_360", 0.25)], "price", 98.0),
        (&[("discount", 0.05), ("dsm_over_360", 0.5)], "price", 97.5),
        (
            &[("discount", 0.0765), ("dsm_over_360", 0.5055555555555555)],
            "price",
            96.1325,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("excel_tbillprice"),
            "ExcelTbillprice",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_tbillprice: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_tbillprice case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_tbillprice case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "discount" | "dsm_over_360" | "price");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "excel_tbillprice case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "excel_tbillprice case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

// excel_rri declares `//! kernel_bank: on` (docs/09-cell80-abi.md's resident-bank
// feature) because inlining the full f32 kernel family alongside its own Nth-root
// Newton/binary-exponentiation locals overruns the code/scratch ceiling before
// STATE_BASE — confirmed directly: StateCell::bind (always unbanked) fails to even
// bind with "268 local slots ... overrun the 0xb000 ceiling". So this cell can't go
// through the generic StateCell harness above; it's exercised instead through the
// same banked-compile path the real admission gate/index uses
// (`cell80::Cartridge::compile` with `CartridgeOpts { kernel_bank: true, .. }`,
// mirroring `cli/meta.rs::library_cartridge`, which reads the cell's own
// `//! kernel_bank: on` header and sets this for real at index/admit time).
#[test]
fn excel_rri_matches_test_cases() {
    fn run_banked(
        fields: &[(&str, f64)],
    ) -> (cell80::Report, std::collections::HashMap<String, u64>) {
        let src = crate::common::cell_src("excel_rri");
        let entry = "ExcelRri::run";
        let addrs = cell80::state_field_addrs(&src, entry)
            .unwrap_or_else(|e| panic!("excel_rri field addrs: {e}"));
        let float_names: &[&str] = &["pv", "fv", "rate"];
        let addr_of = |name: &str| -> (u16, cell80::Ty) {
            addrs
                .iter()
                .find(|(n, _, _)| n == name)
                .map(|(_, a, t)| (*a, *t))
                .unwrap_or_else(|| panic!("excel_rri: no field `{name}`"))
        };
        let sets: Vec<(u16, cell80::Ty, u64)> = fields
            .iter()
            .map(|(n, v)| {
                let (a, t) = addr_of(n);
                let raw = if float_names.contains(n) {
                    (*v as f32).to_bits() as u64
                } else {
                    *v as u64
                };
                (a, t, raw)
            })
            .collect();
        let opts = cell80::CartridgeOpts {
            entry: Some(entry.to_string()),
            kernel_bank: true,
            ..Default::default()
        };
        let cart = cell80::Cartridge::compile(&src, cell80::CellConfig::permissive(), opts)
            .unwrap_or_else(|e| panic!("compile excel_rri (banked): {e}"));
        let program = cart.z80().unwrap_or_else(|e| panic!("excel_rri z80: {e}"));
        let mut runner = cell80::Runner::new(program);
        // A state-cell entry (`&mut self`) takes `self` as its one call arg — the
        // pointer to STATE_BASE, loaded into HL by the trampoline — the same way
        // `StateCell::run` itself calls `run_with_inputs(entry, &[STATE_BASE], ..)`
        // internally (`cell80/src/state.rs`). Omitting this leaves HL at its
        // post-reset default, so `self.nper` reads as 0 and the cell halts
        // immediately on the domain guard — confirmed directly (162-cycle escalate).
        let report = runner
            .run_with_inputs(Some(entry), &[cell80::STATE_BASE], &sets, 20_000_000)
            .unwrap_or_else(|e| panic!("run excel_rri: {e}"));
        let decoded = runner.read_named(&addrs);
        let map: std::collections::HashMap<String, u64> = decoded.into_iter().collect();
        (report, map)
    }

    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (&[("nper", 1.0), ("pv", 100.0), ("fv", 110.0)], "rate", 0.1),
        (&[("nper", 2.0), ("pv", 100.0), ("fv", 121.0)], "rate", 0.1),
        (
            &[("nper", 3.0), ("pv", 1000.0), ("fv", 1331.0)],
            "rate",
            0.1,
        ),
        (&[("nper", 2.0), ("pv", 100.0), ("fv", 81.0)], "rate", -0.1),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let (report, decoded) = run_banked(fields);
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_rri case {i}: {report:?}"
        );
        let raw = *decoded
            .get(*expected_field)
            .unwrap_or_else(|| panic!("excel_rri: no result field {expected_field}"));
        let got = f32::from_bits(raw as u32) as f64;
        let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
        assert!(
            (got - expected_value).abs() < tol,
            "excel_rri case {i}: field {expected_field} got {got} want {expected_value}"
        );
    }
}

#[test]
fn excel_nper_matches_test_cases() {
    // The pack's first transcendental cell (F2 fexp/fln landed 2026-07-11):
    // NPER's unknown sits in the exponent, so the closed form runs through the
    // owned fln. Expected values are Excel's own (NPER(0.05,-100,1000) = 14.2067,
    // the annuity-due variant 13.2536). Tolerance is the pack's semantics-oracle
    // convention; the authoritative accuracy contract is the kernel harness +
    // the cell's `accuracy:` header.
    fn run_banked(fields: &[(&str, f64)]) -> (cell80::Report, f64) {
        let src = crate::common::cell_src("excel_nper");
        let entry = "ExcelNper::run";
        let addrs = cell80::state_field_addrs(&src, entry)
            .unwrap_or_else(|e| panic!("excel_nper field addrs: {e}"));
        let float_names: &[&str] = &["rate", "pmt", "pv", "fv"];
        let sets: Vec<(u16, cell80::Ty, u64)> = fields
            .iter()
            .map(|(n, v)| {
                let (a, t) = addrs
                    .iter()
                    .find(|(fname, _, _)| fname == n)
                    .map(|(_, a, t)| (*a, *t))
                    .unwrap_or_else(|| panic!("excel_nper: no field `{n}`"));
                let raw = if float_names.contains(n) {
                    (*v as f32).to_bits() as u64
                } else {
                    *v as u64
                };
                (a, t, raw)
            })
            .collect();
        let opts = cell80::CartridgeOpts {
            entry: Some(entry.to_string()),
            kernel_bank: true,
            ..Default::default()
        };
        let cart = cell80::Cartridge::compile(&src, cell80::CellConfig::permissive(), opts)
            .unwrap_or_else(|e| panic!("compile excel_nper (banked): {e}"));
        let program = cart.z80().unwrap_or_else(|e| panic!("excel_nper z80: {e}"));
        let mut runner = cell80::Runner::new(program);
        let report = runner
            .run_with_inputs(Some(entry), &[cell80::STATE_BASE], &sets, 2_000_000)
            .unwrap_or_else(|e| panic!("run excel_nper: {e}"));
        let out = addrs
            .iter()
            .find(|(n, _, _)| n == "nper")
            .map(|(_, a, _)| f32::from_bits(runner.peek_u32(*a)) as f64)
            .unwrap();
        (report, out)
    }

    let cases: &[(&[(&str, f64)], f64)] = &[
        // Excel NPER(5%, -100, 1000) — the doc's own shape.
        (&[("rate", 0.05), ("pmt", -100.0), ("pv", 1000.0)], 14.2067),
        // rate == 0 degenerates to the linear count.
        (&[("rate", 0.0), ("pmt", -100.0), ("pv", 1000.0)], 10.0),
        // A target future value joins the stream.
        (
            &[
                ("rate", 0.05),
                ("pmt", -100.0),
                ("pv", 1000.0),
                ("fv", 100.0),
            ],
            15.2067,
        ),
        // Annuity-due (type = 1): payments at period start.
        (
            &[
                ("rate", 0.05),
                ("pmt", -100.0),
                ("pv", 1000.0),
                ("due", 1.0),
            ],
            13.2536,
        ),
    ];
    for (i, (fields, want)) in cases.iter().enumerate() {
        let (report, got) = run_banked(fields);
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_nper case {i}: {report:?}"
        );
        let tol = (want.abs() * 1e-3_f64).max(1e-3);
        assert!(
            (got - want).abs() < tol,
            "excel_nper case {i}: got {got} want {want}"
        );
    }

    // Excel's #NUM! domain, typed: an unreachable target (log of a negative)
    // and the rate == 0, pmt == 0 degenerate both escalate 0xFF06.
    let (report, _) = run_banked(&[
        ("rate", 0.05),
        ("pmt", -100.0),
        ("pv", 1000.0),
        ("fv", -3000.0),
    ]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
    let (report, _) = run_banked(&[("rate", 0.0), ("pmt", 0.0), ("pv", 1000.0)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
}

#[test]
fn excel_pduration_matches_test_cases() {
    // Excel's own documented examples: PDURATION(2.5%, 2000, 2200) = 3.86,
    // PDURATION(0.025/12, 1000, 1200) = 87.6 (the monthly-rate case exercises
    // the small-rate ln(1+r) regime the accuracy header flags).
    fn run_banked(fields: &[(&str, f64)]) -> (cell80::Report, f64) {
        let src = crate::common::cell_src("excel_pduration");
        let entry = "ExcelPduration::run";
        let addrs = cell80::state_field_addrs(&src, entry)
            .unwrap_or_else(|e| panic!("excel_pduration field addrs: {e}"));
        let sets: Vec<(u16, cell80::Ty, u64)> = fields
            .iter()
            .map(|(n, v)| {
                let (a, t) = addrs
                    .iter()
                    .find(|(fname, _, _)| fname == n)
                    .map(|(_, a, t)| (*a, *t))
                    .unwrap_or_else(|| panic!("excel_pduration: no field `{n}`"));
                (a, t, (*v as f32).to_bits() as u64)
            })
            .collect();
        let opts = cell80::CartridgeOpts {
            entry: Some(entry.to_string()),
            kernel_bank: true,
            ..Default::default()
        };
        let cart = cell80::Cartridge::compile(&src, cell80::CellConfig::permissive(), opts)
            .unwrap_or_else(|e| panic!("compile excel_pduration (banked): {e}"));
        let program = cart
            .z80()
            .unwrap_or_else(|e| panic!("excel_pduration z80: {e}"));
        let mut runner = cell80::Runner::new(program);
        let report = runner
            .run_with_inputs(Some(entry), &[cell80::STATE_BASE], &sets, 2_000_000)
            .unwrap_or_else(|e| panic!("run excel_pduration: {e}"));
        let out = addrs
            .iter()
            .find(|(n, _, _)| n == "pduration")
            .map(|(_, a, _)| f32::from_bits(runner.peek_u32(*a)) as f64)
            .unwrap();
        (report, out)
    }

    let cases: &[(&[(&str, f64)], f64)] = &[
        (&[("rate", 0.025), ("pv", 2000.0), ("fv", 2200.0)], 3.8598),
        (
            &[("rate", 0.002083333), ("pv", 1000.0), ("fv", 1200.0)],
            87.6046,
        ),
        (&[("rate", 0.1), ("pv", 100.0), ("fv", 100.0)], 0.0),
    ];
    for (i, (fields, want)) in cases.iter().enumerate() {
        let (report, got) = run_banked(fields);
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "excel_pduration case {i}: {report:?}"
        );
        let tol = (want.abs() * 1e-3_f64).max(1e-3);
        assert!(
            (got - want).abs() < tol,
            "excel_pduration case {i}: got {got} want {want}"
        );
    }

    // #NUM! domain, typed: each argument must be strictly positive.
    for bad in [
        [("rate", 0.0), ("pv", 100.0), ("fv", 110.0)],
        [("rate", 0.05), ("pv", -100.0), ("fv", 110.0)],
        [("rate", 0.05), ("pv", 100.0), ("fv", 0.0)],
    ] {
        let (report, _) = run_banked(&bad);
        assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06), "{bad:?}");
    }
}

// ── the array-input + transcendental wave (2026-07-11, the 10 ex-host_only) ──
// Driven through CellHost::run_state_values (the `.cell` v11 array lane): f32
// scalars and f32-bit array elements ride as raw bit patterns, day offsets as
// plain ints. Every case runs at DEFAULT_CYCLES — the envelopes were sized to
// the budget, and these tests are the proof.

fn run_fin(
    id: &str,
    fields: &[(&str, cell80::FieldValue)],
) -> (
    cell80::Report,
    std::collections::HashMap<String, cell80::FieldValue>,
) {
    run_fin_budget(id, fields, 2_000_000)
}

/// Like [`run_fin`], but with an explicit cycle budget — for the handful of cells
/// (this pack's own `excel_yield` documents the same need for its plain-`StateCell`
/// bisection) whose manifest prices them well above the 2,000,000 default.
/// `excel_xirr`'s own `//! limits:` header measures a worst case around 6.9-6.94M
/// T-states and recommends 12,000,000 as verified-sufficient headroom.
fn run_fin_budget(
    id: &str,
    fields: &[(&str, cell80::FieldValue)],
    budget: u64,
) -> (
    cell80::Report,
    std::collections::HashMap<String, cell80::FieldValue>,
) {
    let src = crate::common::cell_src(id);
    let entry = src
        .lines()
        .find_map(|l| l.strip_prefix("//! entry:"))
        .expect("entry header")
        .trim()
        .to_string();
    let cart = cell80::Cartridge::compile(
        &src,
        cell80::CellConfig::permissive(),
        cell80::CartridgeOpts {
            id: Some(id.into()),
            entry: Some(entry),
            kernel_bank: true,
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| panic!("compile {id}: {e}"));
    let mut host = cell80::CellHost::new();
    host.add(cart);
    let h = host.load(id).unwrap();
    let named: Vec<(String, cell80::FieldValue)> = fields
        .iter()
        .map(|(n, v)| (n.to_string(), v.clone()))
        .collect();
    let (rep, state) = host
        .run_state_values(h, &named, budget)
        .unwrap_or_else(|e| panic!("run {id}: {e}"));
    (rep, state.into_iter().collect())
}

fn fbits(v: f64) -> cell80::FieldValue {
    cell80::FieldValue::Scalar((v as f32).to_bits() as u64)
}
fn farr(vs: &[f64]) -> cell80::FieldValue {
    cell80::FieldValue::Array(vs.iter().map(|v| (*v as f32).to_bits() as u64).collect())
}
fn scalar(v: u64) -> cell80::FieldValue {
    cell80::FieldValue::Scalar(v)
}
fn out_f32(state: &std::collections::HashMap<String, cell80::FieldValue>, name: &str) -> f64 {
    match state[name] {
        cell80::FieldValue::Scalar(v) => f32::from_bits(v as u32) as f64,
        _ => panic!("{name} is not a scalar"),
    }
}
fn assert_close(got: f64, want: f64, what: &str) {
    let tol = (want.abs() * 1e-3_f64).max(1e-3);
    assert!((got - want).abs() < tol, "{what}: got {got} want {want}");
}

#[test]
fn excel_npv_matches_test_cases() {
    // Excel's own doc example: NPV(10%, -10000, 3000, 4200, 6800) = 1188.44.
    let (rep, st) = run_fin(
        "excel_npv",
        &[
            ("rate", fbits(0.10)),
            ("values", farr(&[-10000.0, 3000.0, 4200.0, 6800.0])),
            ("count", scalar(4)),
        ],
    );
    assert_eq!(rep.halt, cell80::Halt::Returned, "{rep:?}");
    assert!(rep.cycles <= 2_000_000);
    assert_close(out_f32(&st, "npv"), 1188.4434, "npv");
    // Domain: count 0 and rate == -1 are Excel's errors, typed.
    let (rep, _) = run_fin("excel_npv", &[("rate", fbits(0.1)), ("count", scalar(0))]);
    assert_eq!(rep.halt, cell80::Halt::Escalate(0xFF06));
    let (rep, _) = run_fin(
        "excel_npv",
        &[
            ("rate", fbits(-1.0)),
            ("values", farr(&[1.0])),
            ("count", scalar(1)),
        ],
    );
    assert_eq!(rep.halt, cell80::Halt::Escalate(0xFF06));
}

#[test]
fn excel_fvschedule_matches_test_cases() {
    // Excel's own doc example: FVSCHEDULE(1, {0.09, 0.11, 0.1}) = 1.3309.
    let (rep, st) = run_fin(
        "excel_fvschedule",
        &[
            ("principal", fbits(1.0)),
            ("schedule", farr(&[0.09, 0.11, 0.10])),
            ("count", scalar(3)),
        ],
    );
    assert_eq!(rep.halt, cell80::Halt::Returned, "{rep:?}");
    assert_close(out_f32(&st, "fv"), 1.33089, "fvschedule");
    // Empty schedule returns the principal unchanged (Excel's behaviour).
    let (rep, st) = run_fin(
        "excel_fvschedule",
        &[("principal", fbits(250.0)), ("count", scalar(0))],
    );
    assert_eq!(rep.halt, cell80::Halt::Returned);
    assert_close(out_f32(&st, "fv"), 250.0, "empty schedule");
}

#[test]
fn excel_irr_matches_test_cases() {
    // Excel's own doc example: IRR({-70000, 12000, 15000, 18000, 21000, 26000})
    // = 8.66% (guess omitted -> 0.1, the cell's own 0.0-means-omitted rule).
    let (rep, st) = run_fin(
        "excel_irr",
        &[
            (
                "values",
                farr(&[-70000.0, 12000.0, 15000.0, 18000.0, 21000.0, 26000.0]),
            ),
            ("count", scalar(6)),
        ],
    );
    assert_eq!(rep.halt, cell80::Halt::Returned, "{rep:?}");
    assert!(
        rep.cycles <= 2_000_000,
        "IRR blew the budget: {}",
        rep.cycles
    );
    assert_close(out_f32(&st, "irr"), 0.086631, "irr");
    // A shorter stream with a negative IRR. The true root of this 4-flow
    // stream is r = -0.182137 (host-verifiable: NPV there is ~0 in f64).
    let (rep, st) = run_fin(
        "excel_irr",
        &[
            ("values", farr(&[-70000.0, 12000.0, 15000.0, 18000.0])),
            ("count", scalar(4)),
            ("guess", fbits(-0.1)),
        ],
    );
    assert_eq!(rep.halt, cell80::Halt::Returned, "{rep:?}");
    assert_close(out_f32(&st, "irr"), -0.182137, "irr negative");
    // Domain, typed: too few flows.
    let (rep, _) = run_fin(
        "excel_irr",
        &[("values", farr(&[-1.0])), ("count", scalar(1))],
    );
    assert_eq!(rep.halt, cell80::Halt::Escalate(0xFF06));
}

#[test]
fn excel_mirr_matches_test_cases() {
    // Excel's own doc example: MIRR({-120000, 39000, 30000, 21000, 37000, 46000},
    // 10%, 12%) = 12.61%.
    let (rep, st) = run_fin(
        "excel_mirr",
        &[
            (
                "values",
                farr(&[-120000.0, 39000.0, 30000.0, 21000.0, 37000.0, 46000.0]),
            ),
            ("count", scalar(6)),
            ("finance_rate", fbits(0.10)),
            ("reinvest_rate", fbits(0.12)),
        ],
    );
    assert_eq!(rep.halt, cell80::Halt::Returned, "{rep:?}");
    assert!(
        rep.cycles <= 2_000_000,
        "MIRR blew the budget: {}",
        rep.cycles
    );
    assert_close(out_f32(&st, "mirr"), 0.126094, "mirr");
    // All-positive stream: Excel's #DIV/0!, typed.
    let (rep, _) = run_fin(
        "excel_mirr",
        &[
            ("values", farr(&[100.0, 200.0])),
            ("count", scalar(2)),
            ("finance_rate", fbits(0.1)),
            ("reinvest_rate", fbits(0.1)),
        ],
    );
    assert_eq!(rep.halt, cell80::Halt::Escalate(0xFF06));
}

#[test]
fn excel_xnpv_matches_test_cases() {
    // Host-f64 mirror oracle (Excel's 5-flow doc example exceeds the 4-slot
    // envelope): first four flows of it, day offsets from the first date.
    let values = [-10000.0_f64, 2750.0, 4250.0, 3250.0];
    let days = [0u64, 60, 303, 411];
    let rate = 0.09_f64;
    let want: f64 = values
        .iter()
        .zip(days.iter())
        .map(|(v, d)| (*v as f32) as f64 * (1.0 + rate).powf(-(*d as f64) / 365.0))
        .sum();
    let (rep, st) = run_fin(
        "excel_xnpv",
        &[
            ("rate", fbits(rate)),
            ("values", farr(&values)),
            ("days", cell80::FieldValue::Array(days.to_vec())),
            ("count", scalar(4)),
        ],
    );
    assert_eq!(rep.halt, cell80::Halt::Returned, "{rep:?}");
    assert!(
        rep.cycles <= 2_000_000,
        "XNPV blew the budget: {}",
        rep.cycles
    );
    assert_close(out_f32(&st, "xnpv"), want, "xnpv");
    // rate <= -1 is out of ln's domain — Excel's #NUM!, typed.
    let (rep, _) = run_fin(
        "excel_xnpv",
        &[
            ("rate", fbits(-1.0)),
            ("values", farr(&[1.0])),
            ("days", cell80::FieldValue::Array(vec![0])),
            ("count", scalar(1)),
        ],
    );
    assert_eq!(rep.halt, cell80::Halt::Escalate(0xFF06));
}

#[test]
fn excel_duration_matches_test_cases() {
    // Whole-period mirror oracle in f64 (Excel's published 10.9782 for this bond
    // folds in basis-1 fractional-period adjustments this cell's declared
    // whole-period scope excludes — mduration's own scope note; the whole-period
    // Macaulay is ~10.956): 30 years of semiannual coupons, 8% coupon / 9% yield.
    let want = {
        let (c, x, n) = (4.0_f64, 1.0_f64 / 1.045, 60);
        let (mut price, mut wp, mut df) = (0.0_f64, 0.0_f64, 1.0_f64);
        for k in 1..=n {
            df *= x;
            let cf = if k == n { c + 100.0 } else { c };
            price += cf * df;
            wp += k as f64 * cf * df;
        }
        wp / (price * 2.0)
    };
    let (rep, st) = run_fin(
        "excel_duration",
        &[
            ("num_periods", scalar(60)),
            ("coupon", fbits(0.08)),
            ("yld", fbits(0.09)),
            ("frequency", scalar(2)),
        ],
    );
    assert_eq!(rep.halt, cell80::Halt::Returned, "{rep:?}");
    assert_close(out_f32(&st, "duration"), want, "duration");
    // And the sibling identity: MDURATION = DURATION / (1 + yld/freq) — at
    // N=6, inside the LANDED mduration loop's own budget reach (its per-period
    // int_to_f32+fdiv walk costs ~100K/period, topping out under 20 periods at
    // the default budget; this cell's closed-form O(log N) prices N=60 above).
    let (rep2, st2) = run_fin(
        "excel_mduration",
        &[
            ("num_periods", scalar(6)),
            ("coupon", fbits(0.08)),
            ("yld", fbits(0.09)),
            ("frequency", scalar(2)),
        ],
    );
    assert_eq!(rep2.halt, cell80::Halt::Returned, "mduration: {rep2:?}");
    let (_, st20) = run_fin(
        "excel_duration",
        &[
            ("num_periods", scalar(6)),
            ("coupon", fbits(0.08)),
            ("yld", fbits(0.09)),
            ("frequency", scalar(2)),
        ],
    );
    assert_close(
        out_f32(&st2, "mduration"),
        out_f32(&st20, "duration") / 1.045,
        "duration/mduration identity",
    );
}

#[test]
fn excel_price_matches_test_cases() {
    // Excel's own doc example: PRICE(2/15/2008, 11/15/2017, 5.75%, 6.50%, 100,
    // 2, 0) = 94.63436 — upstream day-count resolution gives N=20 semiannual
    // periods, DSC/E = 0.5, A/E = 0.5 under 30/360.
    let (rep, st) = run_fin(
        "excel_price",
        &[
            ("rate", fbits(0.0575)),
            ("yld", fbits(0.065)),
            ("redemption", fbits(100.0)),
            ("frequency", scalar(2)),
            ("num_periods", scalar(20)),
            ("dsc_over_e", fbits(0.5)),
            ("a_over_e", fbits(0.5)),
        ],
    );
    assert_eq!(rep.halt, cell80::Halt::Returned, "{rep:?}");
    assert!(
        rep.cycles <= 2_000_000,
        "PRICE blew the budget: {}",
        rep.cycles
    );
    assert_close(out_f32(&st, "price"), 94.63436, "price");
}

#[test]
fn excel_oddlprice_matches_test_cases() {
    // Mirror oracle in f64 (the formula IS the contract — the algebraic inverse
    // of the landed excel_oddlyield), then the round-trip: the priced bond fed
    // back through oddlyield must recover the yield.
    let (c, a, dsc, red, yld) = (
        1.875_f64,
        0.12222222_f64,
        0.71111111_f64,
        100.0_f64,
        0.0405_f64,
    );
    let want = (red + c * (a + dsc)) / (1.0 + (yld / 2.0) * dsc) - c * a;
    let (rep, st) = run_fin(
        "excel_oddlprice",
        &[
            ("rate", fbits(0.0375)),
            ("yld", fbits(yld)),
            ("redemption", fbits(red)),
            ("frequency", scalar(2)),
            ("a_over_e", fbits(a)),
            ("dsc_over_e", fbits(dsc)),
        ],
    );
    assert_eq!(rep.halt, cell80::Halt::Returned, "{rep:?}");
    let price = out_f32(&st, "price");
    assert_close(price, want, "oddlprice");
    let (rep2, st2) = run_fin(
        "excel_oddlyield",
        &[
            ("rate", fbits(0.0375)),
            ("pr", fbits(price)),
            ("redemption", fbits(red)),
            ("frequency", scalar(2)),
            ("a_over_e", fbits(a)),
            ("dsc_over_e", fbits(dsc)),
        ],
    );
    assert_eq!(rep2.halt, cell80::Halt::Returned, "{rep2:?}");
    assert_close(
        out_f32(&st2, "oddlyield"),
        yld,
        "oddlprice/oddlyield round-trip",
    );
}

#[test]
fn excel_oddfprice_matches_test_cases() {
    // Mirror oracle in f64 over the same formula, plus a consistency pin: with a
    // FULL first coupon (dfc/e = 1) and no accrual, ODDFPRICE degenerates to
    // PRICE's regular schedule exactly.
    let (rate, yld, red, n) = (0.0575_f64, 0.065_f64, 100.0_f64, 20u16);
    let (dfc, dsc, a) = (0.65_f64, 0.5_f64, 0.35_f64);
    let c = 100.0 * rate / 2.0;
    let base: f64 = 1.0 + yld / 2.0;
    let mut want = 0.0_f64;
    for k in 1..=n {
        let coupon = if k == 1 { c * dfc } else { c };
        let redemption = if k == n { red } else { 0.0 };
        want += (coupon + redemption) * base.powf(-((k - 1) as f64 + dsc));
    }
    want -= c * a;
    let (rep, st) = run_fin(
        "excel_oddfprice",
        &[
            ("rate", fbits(rate)),
            ("yld", fbits(yld)),
            ("redemption", fbits(red)),
            ("frequency", scalar(2)),
            ("num_periods", scalar(n as u64)),
            ("dfc_over_e", fbits(dfc)),
            ("dsc_over_e", fbits(dsc)),
            ("a_over_e", fbits(a)),
        ],
    );
    assert_eq!(rep.halt, cell80::Halt::Returned, "{rep:?}");
    assert!(
        rep.cycles <= 2_000_000,
        "ODDFPRICE blew the budget: {}",
        rep.cycles
    );
    assert_close(out_f32(&st, "price"), want, "oddfprice");

    let (_, st_full) = run_fin(
        "excel_oddfprice",
        &[
            ("rate", fbits(rate)),
            ("yld", fbits(yld)),
            ("redemption", fbits(red)),
            ("frequency", scalar(2)),
            ("num_periods", scalar(20)),
            ("dfc_over_e", fbits(1.0)),
            ("dsc_over_e", fbits(0.5)),
            ("a_over_e", fbits(0.5)),
        ],
    );
    let (_, st_price) = run_fin(
        "excel_price",
        &[
            ("rate", fbits(rate)),
            ("yld", fbits(yld)),
            ("redemption", fbits(red)),
            ("frequency", scalar(2)),
            ("num_periods", scalar(20)),
            ("dsc_over_e", fbits(0.5)),
            ("a_over_e", fbits(0.5)),
        ],
    );
    assert_close(
        out_f32(&st_full, "price"),
        out_f32(&st_price, "price"),
        "oddfprice(dfc=1) == price",
    );
}

// ── XIRR (2026-07-11/12, the trig/hyperbolic + XIRR wave) ──
// The one function `docs/excel-financial-map.md` priced-not-killed rather than
// shipping in the ex-host_only wave: every XIRR evaluation is a full XNPV pass
// (its own `fln` plus up to 4 `fexp` calls, one per flow), and the bounded
// secant pays up to 6 such evaluations -- ~9.9M T-states worst case, ~5x the
// 2,000,000 default. `excel_xirr.rs`'s own `//! limits:` header measures a
// 4-flow/3-walk case at ~6.9M T-states directly on the emulator and recommends
// 12,000,000 as verified-sufficient headroom -- used here via `run_fin_budget`
// exactly like `excel_yield`'s own higher-budget test does for its bisection.

#[test]
fn excel_xirr_matches_test_cases() {
    // Exact analytic case (whole-year single period): NPV(r) = -1000 +
    // 1100/(1+r) = 0 => r = 0.1 exactly.
    let (rep, st) = run_fin_budget(
        "excel_xirr",
        &[
            ("values", farr(&[-1000.0, 1100.0])),
            ("days", cell80::FieldValue::Array(vec![0, 365])),
            ("count", scalar(2)),
            ("guess", fbits(0.0)),
        ],
        12_000_000,
    );
    assert_eq!(rep.halt, cell80::Halt::Returned, "{rep:?}");
    assert!(
        rep.cycles <= 12_000_000,
        "XIRR blew the 12M budget: {}",
        rep.cycles
    );
    assert_close(out_f32(&st, "xirr"), 0.1, "xirr 2-flow");

    // Irregular 3-flow schedule. True root (float64 XNPV formula, brentq):
    // r = 0.12379180595075324.
    let (rep, st) = run_fin_budget(
        "excel_xirr",
        &[
            ("values", farr(&[-1000.0, 500.0, 600.0])),
            ("days", cell80::FieldValue::Array(vec![0, 180, 400])),
            ("count", scalar(3)),
            ("guess", fbits(0.0)),
        ],
        12_000_000,
    );
    assert_eq!(rep.halt, cell80::Halt::Returned, "{rep:?}");
    assert_close(out_f32(&st, "xirr"), 0.12379199, "xirr 3-flow");

    // Full 4-flow envelope. True root (float64, brentq): r = 0.30805549946278044.
    let (rep, st) = run_fin_budget(
        "excel_xirr",
        &[
            ("values", farr(&[-2000.0, 300.0, 300.0, 2200.0])),
            ("days", cell80::FieldValue::Array(vec![0, 90, 270, 545])),
            ("count", scalar(4)),
            ("guess", fbits(0.0)),
        ],
        12_000_000,
    );
    assert_eq!(rep.halt, cell80::Halt::Returned, "{rep:?}");
    assert_close(out_f32(&st, "xirr"), 0.30805546, "xirr 4-flow");

    // Domain, typed: too few flows (count < 2) and too many (count > the
    // 4-flow envelope) both escalate out_of_domain before any secant walk.
    let (rep, _) = run_fin_budget(
        "excel_xirr",
        &[
            ("values", farr(&[-1000.0])),
            ("days", cell80::FieldValue::Array(vec![0])),
            ("count", scalar(1)),
            ("guess", fbits(0.0)),
        ],
        12_000_000,
    );
    assert_eq!(rep.halt, cell80::Halt::Escalate(0xFF06));

    // count=5 exceeds the fixed 4-flow envelope (only 4 u32 slots exist in
    // `values`/`days`), so the escalation fires before any array access is
    // attempted -- the 4 live slots supplied here are never read.
    let (rep, _) = run_fin_budget(
        "excel_xirr",
        &[
            ("values", farr(&[-1000.0, 100.0, 100.0, 900.0])),
            ("days", cell80::FieldValue::Array(vec![0, 90, 270, 365])),
            ("count", scalar(5)),
            ("guess", fbits(0.0)),
        ],
        12_000_000,
    );
    assert_eq!(rep.halt, cell80::Halt::Escalate(0xFF06));
}
