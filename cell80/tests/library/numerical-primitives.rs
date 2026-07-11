//! Host-oracle tests for the numerical-primitives pack (`cell80/cells/numerical-primitives/*.rs`)
//! — mechanically generated from each cell's own proposed test cases (the 22-cell
//! excel-datetime/control-systems/numerical-primitives batch, verify->admit loop) rather
//! than hand-transcribed: every `test_cases` entry from the authoring output becomes one
//! comparison inside its cell's `#[test]` function, run against the real compiled cell via
//! `StateCell::bind`/`set`/`run`/`get` (see `cell80/tests/library/common.rs` for the shared
//! `cell_src` helper). f32 fields ride raw bit patterns (`to_bits`/`from_bits`, the
//! physics/softfloat packs' own convention) and compare with a small epsilon tolerance
//! rather than bit-exactness, since these expected values are hand-derived arithmetic, not
//! a host-rustc oracle. `nth_root_f32`/`matrix_solve_3x3` both carry `//! kernel_bank: on`
//! (their unbanked size overruns the 8192-byte sandboxed cap once the full f32-kernel
//! family is inlined — `excel_rri` and 22 other excel-financial cells needed the same
//! annotation, docs/excel-financial-map.md's "Update" section) but `StateCell::bind`
//! (`Runner::compile`, used here) always compiles unbanked with no size ceiling, exactly
//! like `excel_db`'s own kernel_bank cell already does in `excel-financial.rs` — the
//! annotation only matters for the real `.cell` cartridge/admission-gate compile path, not
//! this host-oracle harness.
//
// Mechanically generated scaffolds: single-type cells degenerate to `match name
// { _ => .. }` and every case table shares one tuple shape — style lints the
// generator would re-trip next wave are allowed rather than hand-patched.
#![allow(clippy::match_single_binding, clippy::type_complexity)]

#[test]
fn bezier_cubic_f32_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "out" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "p0" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "p1" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "p2" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "p3" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "t" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "out" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "p0" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "p1" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "p2" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "p3" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "t" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("p0", 0.0),
                ("p1", 1.0),
                ("p2", 2.0),
                ("p3", 3.0),
                ("t", 0.0),
            ],
            "out",
            0.0,
        ),
        (
            &[
                ("p0", 0.0),
                ("p1", 1.0),
                ("p2", 2.0),
                ("p3", 3.0),
                ("t", 1.0),
            ],
            "out",
            3.0,
        ),
        (
            &[
                ("p0", 0.0),
                ("p1", 1.0),
                ("p2", 2.0),
                ("p3", 3.0),
                ("t", 0.5),
            ],
            "out",
            1.5,
        ),
        (
            &[
                ("p0", 0.0),
                ("p1", 10.0),
                ("p2", 0.0),
                ("p3", 10.0),
                ("t", 0.5),
            ],
            "out",
            5.0,
        ),
        (
            &[
                ("p0", -5.0),
                ("p1", 15.0),
                ("p2", -15.0),
                ("p3", 5.0),
                ("t", 0.25),
            ],
            "out",
            2.1875,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("bezier_cubic_f32"),
            "BezierCubicF32",
            None,
        )
        .unwrap_or_else(|e| panic!("bind bezier_cubic_f32: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run bezier_cubic_f32 case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "bezier_cubic_f32 case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "out" | "p0" | "p1" | "p2" | "p3" | "t");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "bezier_cubic_f32 case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "bezier_cubic_f32 case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn catmull_rom_f32_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "p0" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "p1" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "p2" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "p3" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "result" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "t" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "p0" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "p1" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "p2" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "p3" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "result" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "t" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("p0", 1.0),
                ("p1", 2.0),
                ("p2", 3.0),
                ("p3", 4.0),
                ("t", 0.0),
            ],
            "result",
            2.0,
        ),
        (
            &[
                ("p0", 1.0),
                ("p1", 2.0),
                ("p2", 3.0),
                ("p3", 4.0),
                ("t", 1.0),
            ],
            "result",
            3.0,
        ),
        (
            &[
                ("p0", 1.0),
                ("p1", 2.0),
                ("p2", 4.0),
                ("p3", 8.0),
                ("t", 0.5),
            ],
            "result",
            2.8125,
        ),
        (
            &[
                ("p0", 2.0),
                ("p1", 4.0),
                ("p2", 6.0),
                ("p3", 8.0),
                ("t", 0.75),
            ],
            "result",
            5.5,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("catmull_rom_f32"),
            "CatmullRomF32",
            None,
        )
        .unwrap_or_else(|e| panic!("bind catmull_rom_f32: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run catmull_rom_f32 case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "catmull_rom_f32 case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "p0" | "p1" | "p2" | "p3" | "result" | "t");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "catmull_rom_f32 case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "catmull_rom_f32 case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

#[test]
fn matrix_solve_3x3_matches_test_cases() {
    fn set_field(cell: &mut cell80::StateCell, name: &str, val: f64) {
        match name {
            "a11" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "a12" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "a13" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "a21" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "a22" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "a23" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "a31" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "a32" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "a33" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "b1" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "b2" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "b3" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "x1" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "x2" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            "x3" => {
                cell.set(name, (val as f32).to_bits() as u64).unwrap();
            }
            _ => {
                cell.set(name, val as u64).unwrap();
            }
        }
    }
    fn get_field(cell: &cell80::StateCell, name: &str) -> f64 {
        match name {
            "a11" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "a12" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "a13" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "a21" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "a22" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "a23" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "a31" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "a32" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "a33" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "b1" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "b2" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "b3" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "x1" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "x2" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            "x3" => f32::from_bits(cell.get(name).unwrap() as u32) as f64,
            _ => cell.get(name).unwrap() as f64,
        }
    }
    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (
            &[
                ("a11", 2.0),
                ("a12", 1.0),
                ("a13", 1.0),
                ("a21", 1.0),
                ("a22", 3.0),
                ("a23", 2.0),
                ("a31", 1.0),
                ("a32", 0.0),
                ("a33", 0.0),
                ("b1", 4.0),
                ("b2", 5.0),
                ("b3", 6.0),
            ],
            "x1",
            6.0,
        ),
        (
            &[
                ("a11", 2.0),
                ("a12", 1.0),
                ("a13", 1.0),
                ("a21", 1.0),
                ("a22", 3.0),
                ("a23", 2.0),
                ("a31", 1.0),
                ("a32", 0.0),
                ("a33", 0.0),
                ("b1", 4.0),
                ("b2", 5.0),
                ("b3", 6.0),
            ],
            "x2",
            15.0,
        ),
        (
            &[
                ("a11", 2.0),
                ("a12", 1.0),
                ("a13", 1.0),
                ("a21", 1.0),
                ("a22", 3.0),
                ("a23", 2.0),
                ("a31", 1.0),
                ("a32", 0.0),
                ("a33", 0.0),
                ("b1", 4.0),
                ("b2", 5.0),
                ("b3", 6.0),
            ],
            "x3",
            -23.0,
        ),
        (
            &[
                ("a11", 1.0),
                ("a12", 1.0),
                ("a13", 1.0),
                ("a21", 0.0),
                ("a22", 2.0),
                ("a23", 5.0),
                ("a31", 2.0),
                ("a32", 5.0),
                ("a33", -1.0),
                ("b1", 6.0),
                ("b2", -4.0),
                ("b3", 27.0),
            ],
            "x1",
            5.0,
        ),
        (
            &[
                ("a11", 1.0),
                ("a12", 1.0),
                ("a13", 1.0),
                ("a21", 0.0),
                ("a22", 2.0),
                ("a23", 5.0),
                ("a31", 2.0),
                ("a32", 5.0),
                ("a33", -1.0),
                ("b1", 6.0),
                ("b2", -4.0),
                ("b3", 27.0),
            ],
            "x2",
            3.0,
        ),
        (
            &[
                ("a11", 1.0),
                ("a12", 1.0),
                ("a13", 1.0),
                ("a21", 0.0),
                ("a22", 2.0),
                ("a23", 5.0),
                ("a31", 2.0),
                ("a32", 5.0),
                ("a33", -1.0),
                ("b1", 6.0),
                ("b2", -4.0),
                ("b3", 27.0),
            ],
            "x3",
            -2.0,
        ),
        (
            &[
                ("a11", 2.0),
                ("a12", 0.0),
                ("a13", 0.0),
                ("a21", 0.0),
                ("a22", 4.0),
                ("a23", 0.0),
                ("a31", 0.0),
                ("a32", 0.0),
                ("a33", 5.0),
                ("b1", 10.0),
                ("b2", 20.0),
                ("b3", 25.0),
            ],
            "x3",
            5.0,
        ),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let mut cell = cell80::StateCell::bind(
            &crate::common::cell_src("matrix_solve_3x3"),
            "MatrixSolve3x3",
            None,
        )
        .unwrap_or_else(|e| panic!("bind matrix_solve_3x3: {e}"));
        for (fname, fval) in fields.iter() {
            set_field(&mut cell, fname, *fval);
        }
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run matrix_solve_3x3 case {i}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "matrix_solve_3x3 case {i}: {report:?}"
        );
        let got = get_field(&cell, expected_field);
        let is_float = matches!(*expected_field, "a11" | "a12" | "a13" | "a21" | "a22" | "a23" | "a31" | "a32" | "a33" | "b1" | "b2" | "b3" | "x1" | "x2" | "x3");
        if is_float {
            let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
            assert!(
                (got - expected_value).abs() < tol,
                "matrix_solve_3x3 case {i}: field {expected_field} got {got} want {expected_value}"
            );
        } else {
            assert_eq!(
                got.round() as i64,
                *expected_value as i64,
                "matrix_solve_3x3 case {i}: field {expected_field} got {got} want {expected_value}"
            );
        }
    }
}

// nth_root_f32 declares `//! kernel_bank: on` (docs/09-cell80-abi.md's resident-bank
// feature) for the same reason excel_rri does (docs/excel-financial-map.md's "Update"
// section): inlining the full f32 kernel family alongside its own Newton/binary-
// exponentiation locals overruns the code/scratch ceiling before STATE_BASE --
// confirmed directly: `StateCell::bind` (always unbanked) fails with "260 local slots
// ... overrun the 0xb000 ceiling". So, like excel_rri, this cell is exercised through
// the same banked-compile path the real admission gate/index uses
// (`cell80::Cartridge::compile` with `CartridgeOpts { kernel_bank: true, .. }`), and at
// a raised cycle budget (20,000,000 -- the (c=1024, n=10) case hits `CycleBudget` at the
// 2,000,000 default, confirmed directly, the same non-convergent-within-default-budget
// class excel_rri's own doc comment documents).
#[test]
fn nth_root_f32_matches_test_cases() {
    fn run_banked(
        fields: &[(&str, f64)],
    ) -> (cell80::Report, std::collections::HashMap<String, u64>) {
        let src = crate::common::cell_src("nth_root_f32");
        let entry = "NthRootF32::run";
        let addrs = cell80::state_field_addrs(&src, entry)
            .unwrap_or_else(|e| panic!("nth_root_f32 field addrs: {e}"));
        let float_names: &[&str] = &["c", "root"];
        let addr_of = |name: &str| -> (u16, cell80::Ty) {
            addrs
                .iter()
                .find(|(n, _, _)| n == name)
                .map(|(_, a, t)| (*a, *t))
                .unwrap_or_else(|| panic!("nth_root_f32: no field `{name}`"))
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
            .unwrap_or_else(|e| panic!("compile nth_root_f32 (banked): {e}"));
        let program = cart.z80().unwrap_or_else(|e| panic!("nth_root_f32 z80: {e}"));
        let mut runner = cell80::Runner::new(program);
        let report = runner
            .run_with_inputs(Some(entry), &[cell80::STATE_BASE], &sets, 20_000_000)
            .unwrap_or_else(|e| panic!("run nth_root_f32: {e}"));
        let decoded = runner.read_named(&addrs);
        let map: std::collections::HashMap<String, u64> = decoded.into_iter().collect();
        (report, map)
    }

    let cases: &[(&[(&str, f64)], &str, f64)] = &[
        (&[("c", 16.0), ("n", 2.0)], "root", 4.0),
        (&[("c", 27.0), ("n", 3.0)], "root", 3.0),
        (&[("c", 42.0), ("n", 1.0)], "root", 42.0),
        (&[("c", 2.0), ("n", 2.0)], "root", 1.4142135),
        (&[("c", 1024.0), ("n", 10.0)], "root", 2.0),
    ];
    for (i, (fields, expected_field, expected_value)) in cases.iter().enumerate() {
        let (report, decoded) = run_banked(fields);
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "nth_root_f32 case {i}: {report:?}"
        );
        let raw = *decoded
            .get(*expected_field)
            .unwrap_or_else(|| panic!("nth_root_f32: no result field {expected_field}"));
        let got = f32::from_bits(raw as u32) as f64;
        let tol = (expected_value.abs() * 1e-3_f64).max(1e-3);
        assert!(
            (got - expected_value).abs() < tol,
            "nth_root_f32 case {i}: field {expected_field} got {got} want {expected_value}"
        );
    }
}

