//! Host-oracle tests for the trig pack (`cell80/cells/trig/*.rs`) -- mechanically
//! generated from each cell's own proposed test cases (the 21-cell trig/hyperbolic
//! batch riding the F2 owned fsin/fcos/fatan2/fexp/fln transcendentals, verify->admit
//! loop) rather than hand-transcribed: every `test_cases` entry from the authoring
//! output becomes one comparison, run against the real compiled cell via
//! `StateCell::bind`/`set`/`run`/`get` (see `cell80/tests/library/common.rs` for the
//! shared `cell_src` helper). f32 fields ride raw bit patterns (`to_bits`/`from_bits`,
//! the physics/softfloat packs' own convention) and compare with a small relative-
//! tolerance epsilon rather than bit-exactness, matching `excel-mathstat.rs`/
//! `excel-financial.rs`'s own f32 test convention -- the F2 kernels are class
//! *approximate* (bounded, not bit-exact-vs-rustc), so no test here asserts bit-exact
//! equality. A domain/overflow-escalation case (`halt`/`halt_code` expected_field in
//! the authoring output) asserts `Halt::Escalate(0xFF08)` (float_domain) instead of a
//! numeric comparison, the same convention `excel-mathstat.rs`'s `excel_min`/
//! `excel_sqrt` escalation cases already use.
//
// Mechanically generated scaffolds: every case table shares one tuple shape --
// style lints the generator would re-trip next wave are allowed rather than
// hand-patched.
#![allow(
    clippy::type_complexity,
    clippy::approx_constant,
    clippy::excessive_precision
)]

use cell80::{Halt, StateCell};

fn f32_tol(got: f32, want: f32) -> bool {
    (got - want).abs() < (want.abs() * 1e-3_f32).max(1e-3_f32)
}

// `tan_f32`/`cot_f32` (fsin + fcos) and `asin_f32`/`acos_f32` (fsqrt + fatan2) each
// compose two distinct heavy F2 kernel bodies -- too large to fit before the state
// region at `STATE_BASE` even with no artificial cap, a hard architecture wall
// discovered by actually attempting the bind (all four already declare
// `kernel_bank: on` and compile clean through the real sandboxed cartridge path).
// Those four use `crate::common::BankedCell` instead of `StateCell::bind` below --
// see that helper's own doc comment for the full story.
use crate::common::BankedCell;

#[test]
fn sin_f32_matches_test_cases() {
    for (i, case) in sin_f32_cases().iter().enumerate() {
        let mut cell = StateCell::bind(&crate::common::cell_src("sin_f32"), "SinF32", None)
            .unwrap_or_else(|e| panic!("bind sin_f32: {e}"));
        cell.set("x", (case.x).to_bits() as u64).unwrap();
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run sin_f32 case {i}: {e}"));
        match case.want {
            Want::Value(want) => {
                assert_eq!(
                    report.halt,
                    Halt::Returned,
                    "{} case {} report: {:?}",
                    "sin_f32",
                    i,
                    report
                );
                let got = f32::from_bits(cell.get("result").unwrap() as u32);
                assert!(
                    f32_tol(got, want),
                    "sin_f32 case {i}: got {got} want {want}",
                );
            }
            Want::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(code),
                    "sin_f32 case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

struct SinF32Case {
    x: f32,
    want: Want,
}
enum Want {
    Value(f32),
    Halt(u16),
}
fn sin_f32_cases() -> Vec<SinF32Case> {
    vec![
        SinF32Case {
            x: 0_f32,
            want: Want::Value(0_f32),
        },
        SinF32Case {
            x: 1.5707963267948966_f32,
            want: Want::Value(1_f32),
        },
        SinF32Case {
            x: -1.5707963267948966_f32,
            want: Want::Value(-1_f32),
        },
    ]
}

#[test]
fn cos_f32_matches_test_cases() {
    for (i, case) in cos_f32_cases().iter().enumerate() {
        let mut cell = StateCell::bind(&crate::common::cell_src("cos_f32"), "CosF32", None)
            .unwrap_or_else(|e| panic!("bind cos_f32: {e}"));
        cell.set("x", (case.x).to_bits() as u64).unwrap();
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run cos_f32 case {i}: {e}"));
        match case.want {
            Want::Value(want) => {
                assert_eq!(
                    report.halt,
                    Halt::Returned,
                    "{} case {} report: {:?}",
                    "cos_f32",
                    i,
                    report
                );
                let got = f32::from_bits(cell.get("result").unwrap() as u32);
                assert!(
                    f32_tol(got, want),
                    "cos_f32 case {i}: got {got} want {want}",
                );
            }
            Want::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(code),
                    "cos_f32 case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

struct CosF32Case {
    x: f32,
    want: Want,
}

fn cos_f32_cases() -> Vec<CosF32Case> {
    vec![
        CosF32Case {
            x: 0_f32,
            want: Want::Value(1_f32),
        },
        CosF32Case {
            x: 1_f32,
            want: Want::Value(0.5403022766113281_f32),
        },
        CosF32Case {
            x: -1_f32,
            want: Want::Value(0.5403022766113281_f32),
        },
        CosF32Case {
            x: 0.5_f32,
            want: Want::Value(0.8775825500488281_f32),
        },
        CosF32Case {
            x: 3.1415927410125732_f32,
            want: Want::Value(-1_f32),
        },
    ]
}

#[test]
fn tan_f32_matches_test_cases() {
    for (i, case) in tan_f32_cases().iter().enumerate() {
        let mut cell = BankedCell::bind(&crate::common::cell_src("tan_f32"), "TanF32::run");
        cell.set("x", (case.x).to_bits() as u64);
        let report = cell.run(cell80::DEFAULT_CYCLES);
        match case.want {
            Want::Value(want) => {
                assert_eq!(
                    report.halt,
                    Halt::Returned,
                    "{} case {} report: {:?}",
                    "tan_f32",
                    i,
                    report
                );
                let got = f32::from_bits(cell.get("result") as u32);
                assert!(
                    f32_tol(got, want),
                    "tan_f32 case {i}: got {got} want {want}",
                );
            }
            Want::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(code),
                    "tan_f32 case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

struct TanF32Case {
    x: f32,
    want: Want,
}

fn tan_f32_cases() -> Vec<TanF32Case> {
    vec![
        TanF32Case {
            x: 0_f32,
            want: Want::Value(0_f32),
        },
        TanF32Case {
            x: 0.7853981852531433_f32,
            want: Want::Value(1.0000001192092896_f32),
        },
        TanF32Case {
            x: -0.7853981852531433_f32,
            want: Want::Value(-1.0000001192092896_f32),
        },
        TanF32Case {
            x: 1_f32,
            want: Want::Value(1.5574077367782593_f32),
        },
        TanF32Case {
            x: 2_f32,
            want: Want::Value(-2.18503999710083_f32),
        },
        TanF32Case {
            x: 1.5707963705062866_f32,
            want: Want::Halt(65288),
        },
        TanF32Case {
            x: 10000_f32,
            want: Want::Halt(65288),
        },
    ]
}

#[test]
fn atan2_f32_matches_test_cases() {
    for (i, case) in atan2_f32_cases().iter().enumerate() {
        let mut cell = StateCell::bind(&crate::common::cell_src("atan2_f32"), "Atan2F32", None)
            .unwrap_or_else(|e| panic!("bind atan2_f32: {e}"));
        cell.set("y", (case.y).to_bits() as u64).unwrap();
        cell.set("x", (case.x).to_bits() as u64).unwrap();
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run atan2_f32 case {i}: {e}"));
        match case.want {
            Want::Value(want) => {
                assert_eq!(
                    report.halt,
                    Halt::Returned,
                    "{} case {} report: {:?}",
                    "atan2_f32",
                    i,
                    report
                );
                let got = f32::from_bits(cell.get("angle").unwrap() as u32);
                assert!(
                    f32_tol(got, want),
                    "atan2_f32 case {i}: got {got} want {want}",
                );
            }
            Want::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(code),
                    "atan2_f32 case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

struct Atan2F32Case {
    y: f32,
    x: f32,
    want: Want,
}

fn atan2_f32_cases() -> Vec<Atan2F32Case> {
    vec![
        Atan2F32Case {
            y: 1_f32,
            x: 1_f32,
            want: Want::Value(0.7853981633974483_f32),
        },
        Atan2F32Case {
            y: 1_f32,
            x: -1_f32,
            want: Want::Value(2.356194490192345_f32),
        },
        Atan2F32Case {
            y: -1_f32,
            x: -1_f32,
            want: Want::Value(-2.356194490192345_f32),
        },
        Atan2F32Case {
            y: -1_f32,
            x: 1_f32,
            want: Want::Value(-0.7853981633974483_f32),
        },
        Atan2F32Case {
            y: 0_f32,
            x: -1_f32,
            want: Want::Value(3.141592653589793_f32),
        },
        Atan2F32Case {
            y: 3_f32,
            x: 4_f32,
            want: Want::Value(0.6435011087932844_f32),
        },
        Atan2F32Case {
            y: -3_f32,
            x: -4_f32,
            want: Want::Value(-2.498091544796509_f32),
        },
    ]
}

#[test]
fn atan_f32_matches_test_cases() {
    for (i, case) in atan_f32_cases().iter().enumerate() {
        let mut cell = StateCell::bind(&crate::common::cell_src("atan_f32"), "AtanF32", None)
            .unwrap_or_else(|e| panic!("bind atan_f32: {e}"));
        cell.set("x", (case.x).to_bits() as u64).unwrap();
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run atan_f32 case {i}: {e}"));
        match case.want {
            Want::Value(want) => {
                assert_eq!(
                    report.halt,
                    Halt::Returned,
                    "{} case {} report: {:?}",
                    "atan_f32",
                    i,
                    report
                );
                let got = f32::from_bits(cell.get("result").unwrap() as u32);
                assert!(
                    f32_tol(got, want),
                    "atan_f32 case {i}: got {got} want {want}",
                );
            }
            Want::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(code),
                    "atan_f32 case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

struct AtanF32Case {
    x: f32,
    want: Want,
}

fn atan_f32_cases() -> Vec<AtanF32Case> {
    vec![
        AtanF32Case {
            x: 0_f32,
            want: Want::Value(0_f32),
        },
        AtanF32Case {
            x: 1_f32,
            want: Want::Value(0.7853981633974483_f32),
        },
        AtanF32Case {
            x: -1_f32,
            want: Want::Value(-0.7853981633974483_f32),
        },
        AtanF32Case {
            x: 1.7320508075688772_f32,
            want: Want::Value(1.0471975511965979_f32),
        },
        AtanF32Case {
            x: 1000_f32,
            want: Want::Value(1.5697963271282298_f32),
        },
    ]
}

#[test]
fn asin_f32_matches_test_cases() {
    for (i, case) in asin_f32_cases().iter().enumerate() {
        let mut cell = BankedCell::bind(&crate::common::cell_src("asin_f32"), "AsinF32::run");
        cell.set("x", (case.x).to_bits() as u64);
        let report = cell.run(cell80::DEFAULT_CYCLES);
        match case.want {
            Want::Value(want) => {
                assert_eq!(
                    report.halt,
                    Halt::Returned,
                    "{} case {} report: {:?}",
                    "asin_f32",
                    i,
                    report
                );
                let got = f32::from_bits(cell.get("result") as u32);
                assert!(
                    f32_tol(got, want),
                    "asin_f32 case {i}: got {got} want {want}",
                );
            }
            Want::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(code),
                    "asin_f32 case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

struct AsinF32Case {
    x: f32,
    want: Want,
}

fn asin_f32_cases() -> Vec<AsinF32Case> {
    vec![
        AsinF32Case {
            x: 0_f32,
            want: Want::Value(0_f32),
        },
        AsinF32Case {
            x: 0.5_f32,
            want: Want::Value(0.5235987755982989_f32),
        },
        AsinF32Case {
            x: -0.5_f32,
            want: Want::Value(-0.5235987755982989_f32),
        },
        AsinF32Case {
            x: 1_f32,
            want: Want::Value(1.5707963267948966_f32),
        },
        AsinF32Case {
            x: -1_f32,
            want: Want::Value(-1.5707963267948966_f32),
        },
        AsinF32Case {
            x: 0.7071067811865476_f32,
            want: Want::Value(0.7853981633974483_f32),
        },
    ]
}

#[test]
fn acos_f32_matches_test_cases() {
    for (i, case) in acos_f32_cases().iter().enumerate() {
        let mut cell = BankedCell::bind(&crate::common::cell_src("acos_f32"), "AcosF32::run");
        cell.set("x", (case.x).to_bits() as u64);
        let report = cell.run(cell80::DEFAULT_CYCLES);
        match case.want {
            Want::Value(want) => {
                assert_eq!(
                    report.halt,
                    Halt::Returned,
                    "{} case {} report: {:?}",
                    "acos_f32",
                    i,
                    report
                );
                let got = f32::from_bits(cell.get("result") as u32);
                assert!(
                    f32_tol(got, want),
                    "acos_f32 case {i}: got {got} want {want}",
                );
            }
            Want::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(code),
                    "acos_f32 case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

struct AcosF32Case {
    x: f32,
    want: Want,
}

fn acos_f32_cases() -> Vec<AcosF32Case> {
    vec![
        AcosF32Case {
            x: 1_f32,
            want: Want::Value(0_f32),
        },
        AcosF32Case {
            x: -1_f32,
            want: Want::Value(3.1415927410125732_f32),
        },
        AcosF32Case {
            x: 0_f32,
            want: Want::Value(1.5707963705062866_f32),
        },
        AcosF32Case {
            x: 0.5_f32,
            want: Want::Value(1.0471975803375244_f32),
        },
        AcosF32Case {
            x: -0.5_f32,
            want: Want::Value(2.094395160675049_f32),
        },
        AcosF32Case {
            x: 1.2_f32,
            want: Want::Halt(65288),
        },
    ]
}

#[test]
fn sinh_f32_matches_test_cases() {
    for (i, case) in sinh_f32_cases().iter().enumerate() {
        let mut cell = StateCell::bind(&crate::common::cell_src("sinh_f32"), "SinhF32", None)
            .unwrap_or_else(|e| panic!("bind sinh_f32: {e}"));
        cell.set("x", (case.x).to_bits() as u64).unwrap();
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run sinh_f32 case {i}: {e}"));
        match case.want {
            Want::Value(want) => {
                assert_eq!(
                    report.halt,
                    Halt::Returned,
                    "{} case {} report: {:?}",
                    "sinh_f32",
                    i,
                    report
                );
                let got = f32::from_bits(cell.get("result").unwrap() as u32);
                assert!(
                    f32_tol(got, want),
                    "sinh_f32 case {i}: got {got} want {want}",
                );
            }
            Want::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(code),
                    "sinh_f32 case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

struct SinhF32Case {
    x: f32,
    want: Want,
}

fn sinh_f32_cases() -> Vec<SinhF32Case> {
    vec![
        SinhF32Case {
            x: 0_f32,
            want: Want::Value(0_f32),
        },
        SinhF32Case {
            x: 1_f32,
            want: Want::Value(1.1752011936438014_f32),
        },
        SinhF32Case {
            x: -2_f32,
            want: Want::Value(-3.6268604078470186_f32),
        },
        SinhF32Case {
            x: 3.5_f32,
            want: Want::Value(16.542627287634996_f32),
        },
    ]
}

#[test]
fn cosh_f32_matches_test_cases() {
    for (i, case) in cosh_f32_cases().iter().enumerate() {
        let mut cell = StateCell::bind(&crate::common::cell_src("cosh_f32"), "CoshF32", None)
            .unwrap_or_else(|e| panic!("bind cosh_f32: {e}"));
        cell.set("x", (case.x).to_bits() as u64).unwrap();
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run cosh_f32 case {i}: {e}"));
        match case.want {
            Want::Value(want) => {
                assert_eq!(
                    report.halt,
                    Halt::Returned,
                    "{} case {} report: {:?}",
                    "cosh_f32",
                    i,
                    report
                );
                let got = f32::from_bits(cell.get("result").unwrap() as u32);
                assert!(
                    f32_tol(got, want),
                    "cosh_f32 case {i}: got {got} want {want}",
                );
            }
            Want::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(code),
                    "cosh_f32 case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

struct CoshF32Case {
    x: f32,
    want: Want,
}

fn cosh_f32_cases() -> Vec<CoshF32Case> {
    vec![
        CoshF32Case {
            x: 0_f32,
            want: Want::Value(1_f32),
        },
        CoshF32Case {
            x: 1_f32,
            want: Want::Value(1.5430806_f32),
        },
        CoshF32Case {
            x: -2_f32,
            want: Want::Value(3.7621958_f32),
        },
        CoshF32Case {
            x: 5_f32,
            want: Want::Value(74.209953_f32),
        },
        CoshF32Case {
            x: 20_f32,
            want: Want::Value(242582592_f32),
        },
    ]
}

#[test]
fn tanh_f32_matches_test_cases() {
    for (i, case) in tanh_f32_cases().iter().enumerate() {
        let mut cell = StateCell::bind(&crate::common::cell_src("tanh_f32"), "TanhF32", None)
            .unwrap_or_else(|e| panic!("bind tanh_f32: {e}"));
        cell.set("x", (case.x).to_bits() as u64).unwrap();
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run tanh_f32 case {i}: {e}"));
        match case.want {
            Want::Value(want) => {
                assert_eq!(
                    report.halt,
                    Halt::Returned,
                    "{} case {} report: {:?}",
                    "tanh_f32",
                    i,
                    report
                );
                let got = f32::from_bits(cell.get("result").unwrap() as u32);
                assert!(
                    f32_tol(got, want),
                    "tanh_f32 case {i}: got {got} want {want}",
                );
            }
            Want::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(code),
                    "tanh_f32 case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

struct TanhF32Case {
    x: f32,
    want: Want,
}

fn tanh_f32_cases() -> Vec<TanhF32Case> {
    vec![
        TanhF32Case {
            x: 0_f32,
            want: Want::Value(0_f32),
        },
        TanhF32Case {
            x: 1_f32,
            want: Want::Value(0.7615941559557649_f32),
        },
        TanhF32Case {
            x: -1_f32,
            want: Want::Value(-0.7615941559557649_f32),
        },
        TanhF32Case {
            x: 2_f32,
            want: Want::Value(0.9640275800758169_f32),
        },
        TanhF32Case {
            x: 10_f32,
            want: Want::Value(1_f32),
        },
        TanhF32Case {
            x: -50_f32,
            want: Want::Value(-1_f32),
        },
        TanhF32Case {
            x: 0.001_f32,
            want: Want::Value(0.0009999996666668_f32),
        },
        TanhF32Case {
            x: 0.5_f32,
            want: Want::Value(0.46211715726000974_f32),
        },
    ]
}

#[test]
fn asinh_f32_matches_test_cases() {
    for (i, case) in asinh_f32_cases().iter().enumerate() {
        let mut cell = StateCell::bind(&crate::common::cell_src("asinh_f32"), "AsinhF32", None)
            .unwrap_or_else(|e| panic!("bind asinh_f32: {e}"));
        cell.set("x", (case.x).to_bits() as u64).unwrap();
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run asinh_f32 case {i}: {e}"));
        match case.want {
            Want::Value(want) => {
                assert_eq!(
                    report.halt,
                    Halt::Returned,
                    "{} case {} report: {:?}",
                    "asinh_f32",
                    i,
                    report
                );
                let got = f32::from_bits(cell.get("asinh").unwrap() as u32);
                assert!(
                    f32_tol(got, want),
                    "asinh_f32 case {i}: got {got} want {want}",
                );
            }
            Want::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(code),
                    "asinh_f32 case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

struct AsinhF32Case {
    x: f32,
    want: Want,
}

fn asinh_f32_cases() -> Vec<AsinhF32Case> {
    vec![
        AsinhF32Case {
            x: 0_f32,
            want: Want::Value(0_f32),
        },
        AsinhF32Case {
            x: 1_f32,
            want: Want::Value(0.8813735870195429_f32),
        },
        AsinhF32Case {
            x: -2_f32,
            want: Want::Value(-1.4436354751788099_f32),
        },
        AsinhF32Case {
            x: 5_f32,
            want: Want::Value(2.3124383412727525_f32),
        },
    ]
}

#[test]
fn acosh_f32_matches_test_cases() {
    for (i, case) in acosh_f32_cases().iter().enumerate() {
        let mut cell = StateCell::bind(&crate::common::cell_src("acosh_f32"), "AcoshF32", None)
            .unwrap_or_else(|e| panic!("bind acosh_f32: {e}"));
        cell.set("x", (case.x).to_bits() as u64).unwrap();
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run acosh_f32 case {i}: {e}"));
        match case.want {
            Want::Value(want) => {
                assert_eq!(
                    report.halt,
                    Halt::Returned,
                    "{} case {} report: {:?}",
                    "acosh_f32",
                    i,
                    report
                );
                let got = f32::from_bits(cell.get("result").unwrap() as u32);
                assert!(
                    f32_tol(got, want),
                    "acosh_f32 case {i}: got {got} want {want}",
                );
            }
            Want::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(code),
                    "acosh_f32 case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

struct AcoshF32Case {
    x: f32,
    want: Want,
}

fn acosh_f32_cases() -> Vec<AcoshF32Case> {
    vec![
        AcoshF32Case {
            x: 1_f32,
            want: Want::Value(0_f32),
        },
        AcoshF32Case {
            x: 2_f32,
            want: Want::Value(1.3169579_f32),
        },
        AcoshF32Case {
            x: 10_f32,
            want: Want::Value(2.993223_f32),
        },
        AcoshF32Case {
            x: 0.5_f32,
            want: Want::Halt(65288),
        },
    ]
}

#[test]
fn atanh_f32_matches_test_cases() {
    for (i, case) in atanh_f32_cases().iter().enumerate() {
        let mut cell = StateCell::bind(&crate::common::cell_src("atanh_f32"), "AtanhF32", None)
            .unwrap_or_else(|e| panic!("bind atanh_f32: {e}"));
        cell.set("x", (case.x).to_bits() as u64).unwrap();
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run atanh_f32 case {i}: {e}"));
        match case.want {
            Want::Value(want) => {
                assert_eq!(
                    report.halt,
                    Halt::Returned,
                    "{} case {} report: {:?}",
                    "atanh_f32",
                    i,
                    report
                );
                let got = f32::from_bits(cell.get("result").unwrap() as u32);
                assert!(
                    f32_tol(got, want),
                    "atanh_f32 case {i}: got {got} want {want}",
                );
            }
            Want::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(code),
                    "atanh_f32 case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

struct AtanhF32Case {
    x: f32,
    want: Want,
}

fn atanh_f32_cases() -> Vec<AtanhF32Case> {
    vec![
        AtanhF32Case {
            x: 0_f32,
            want: Want::Value(0_f32),
        },
        AtanhF32Case {
            x: 0.5_f32,
            want: Want::Value(0.5493061443340549_f32),
        },
        AtanhF32Case {
            x: -0.5_f32,
            want: Want::Value(-0.5493061443340549_f32),
        },
        AtanhF32Case {
            x: 0.9_f32,
            want: Want::Value(1.4722194895832204_f32),
        },
        AtanhF32Case {
            x: 0.1_f32,
            want: Want::Value(0.10033534773107562_f32),
        },
        AtanhF32Case {
            x: 1_f32,
            want: Want::Halt(65288),
        },
        AtanhF32Case {
            x: -2_f32,
            want: Want::Halt(65288),
        },
    ]
}

#[test]
fn cot_f32_matches_test_cases() {
    for (i, case) in cot_f32_cases().iter().enumerate() {
        let mut cell = BankedCell::bind(&crate::common::cell_src("cot_f32"), "CotF32::run");
        cell.set("x", (case.x).to_bits() as u64);
        let report = cell.run(cell80::DEFAULT_CYCLES);
        match case.want {
            Want::Value(want) => {
                assert_eq!(
                    report.halt,
                    Halt::Returned,
                    "{} case {} report: {:?}",
                    "cot_f32",
                    i,
                    report
                );
                let got = f32::from_bits(cell.get("result") as u32);
                assert!(
                    f32_tol(got, want),
                    "cot_f32 case {i}: got {got} want {want}",
                );
            }
            Want::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(code),
                    "cot_f32 case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

struct CotF32Case {
    x: f32,
    want: Want,
}

fn cot_f32_cases() -> Vec<CotF32Case> {
    vec![
        CotF32Case {
            x: 0.7853981633974483_f32,
            want: Want::Value(1_f32),
        },
        CotF32Case {
            x: 0.5235987755982988_f32,
            want: Want::Value(1.7320508075688772_f32),
        },
        CotF32Case {
            x: 1.0471975511965976_f32,
            want: Want::Value(0.5773502691896258_f32),
        },
        CotF32Case {
            x: -0.7853981633974483_f32,
            want: Want::Value(-1_f32),
        },
    ]
}

#[test]
fn sec_f32_matches_test_cases() {
    for (i, case) in sec_f32_cases().iter().enumerate() {
        let mut cell = StateCell::bind(&crate::common::cell_src("sec_f32"), "SecF32", None)
            .unwrap_or_else(|e| panic!("bind sec_f32: {e}"));
        cell.set("x", (case.x).to_bits() as u64).unwrap();
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run sec_f32 case {i}: {e}"));
        match case.want {
            Want::Value(want) => {
                assert_eq!(
                    report.halt,
                    Halt::Returned,
                    "{} case {} report: {:?}",
                    "sec_f32",
                    i,
                    report
                );
                let got = f32::from_bits(cell.get("result").unwrap() as u32);
                assert!(
                    f32_tol(got, want),
                    "sec_f32 case {i}: got {got} want {want}",
                );
            }
            Want::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(code),
                    "sec_f32 case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

struct SecF32Case {
    x: f32,
    want: Want,
}

fn sec_f32_cases() -> Vec<SecF32Case> {
    vec![
        SecF32Case {
            x: 0_f32,
            want: Want::Value(1_f32),
        },
        SecF32Case {
            x: 1.0471976_f32,
            want: Want::Value(2_f32),
        },
        SecF32Case {
            x: 0.7853982_f32,
            want: Want::Value(1.4142136_f32),
        },
        SecF32Case {
            x: 1.5707964_f32,
            want: Want::Halt(65288),
        },
    ]
}

#[test]
fn csc_f32_matches_test_cases() {
    for (i, case) in csc_f32_cases().iter().enumerate() {
        let mut cell = StateCell::bind(&crate::common::cell_src("csc_f32"), "CscF32", None)
            .unwrap_or_else(|e| panic!("bind csc_f32: {e}"));
        cell.set("x", (case.x).to_bits() as u64).unwrap();
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run csc_f32 case {i}: {e}"));
        match case.want {
            Want::Value(want) => {
                assert_eq!(
                    report.halt,
                    Halt::Returned,
                    "{} case {} report: {:?}",
                    "csc_f32",
                    i,
                    report
                );
                let got = f32::from_bits(cell.get("result").unwrap() as u32);
                assert!(
                    f32_tol(got, want),
                    "csc_f32 case {i}: got {got} want {want}",
                );
            }
            Want::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(code),
                    "csc_f32 case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

struct CscF32Case {
    x: f32,
    want: Want,
}

fn csc_f32_cases() -> Vec<CscF32Case> {
    vec![
        CscF32Case {
            x: 1.5707963267948966_f32,
            want: Want::Value(1_f32),
        },
        CscF32Case {
            x: 0.5235987755982988_f32,
            want: Want::Value(2_f32),
        },
        CscF32Case {
            x: -1.5707963267948966_f32,
            want: Want::Value(-1_f32),
        },
        CscF32Case {
            x: 0_f32,
            want: Want::Halt(65288),
        },
        CscF32Case {
            x: 3.1415927_f32,
            want: Want::Halt(65288),
        },
    ]
}

#[test]
fn coth_f32_matches_test_cases() {
    for (i, case) in coth_f32_cases().iter().enumerate() {
        let mut cell = StateCell::bind(&crate::common::cell_src("coth_f32"), "CothF32", None)
            .unwrap_or_else(|e| panic!("bind coth_f32: {e}"));
        cell.set("x", (case.x).to_bits() as u64).unwrap();
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run coth_f32 case {i}: {e}"));
        match case.want {
            Want::Value(want) => {
                assert_eq!(
                    report.halt,
                    Halt::Returned,
                    "{} case {} report: {:?}",
                    "coth_f32",
                    i,
                    report
                );
                let got = f32::from_bits(cell.get("result").unwrap() as u32);
                assert!(
                    f32_tol(got, want),
                    "coth_f32 case {i}: got {got} want {want}",
                );
            }
            Want::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(code),
                    "coth_f32 case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

struct CothF32Case {
    x: f32,
    want: Want,
}

fn coth_f32_cases() -> Vec<CothF32Case> {
    vec![
        CothF32Case {
            x: 1_f32,
            want: Want::Value(1.3130352854993315_f32),
        },
        CothF32Case {
            x: 2_f32,
            want: Want::Value(1.0373147207275482_f32),
        },
        CothF32Case {
            x: -1_f32,
            want: Want::Value(-1.3130352854993315_f32),
        },
        CothF32Case {
            x: 0.5_f32,
            want: Want::Value(2.163953413738653_f32),
        },
        CothF32Case {
            x: -3_f32,
            want: Want::Value(-1.0049698233136892_f32),
        },
        CothF32Case {
            x: 100_f32,
            want: Want::Value(1_f32),
        },
    ]
}

#[test]
fn sech_f32_matches_test_cases() {
    for (i, case) in sech_f32_cases().iter().enumerate() {
        let mut cell = StateCell::bind(&crate::common::cell_src("sech_f32"), "SechF32", None)
            .unwrap_or_else(|e| panic!("bind sech_f32: {e}"));
        cell.set("x", (case.x).to_bits() as u64).unwrap();
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run sech_f32 case {i}: {e}"));
        match case.want {
            Want::Value(want) => {
                assert_eq!(
                    report.halt,
                    Halt::Returned,
                    "{} case {} report: {:?}",
                    "sech_f32",
                    i,
                    report
                );
                let got = f32::from_bits(cell.get("result").unwrap() as u32);
                assert!(
                    f32_tol(got, want),
                    "sech_f32 case {i}: got {got} want {want}",
                );
            }
            Want::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(code),
                    "sech_f32 case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

struct SechF32Case {
    x: f32,
    want: Want,
}

fn sech_f32_cases() -> Vec<SechF32Case> {
    vec![
        SechF32Case {
            x: 0_f32,
            want: Want::Value(1_f32),
        },
        SechF32Case {
            x: 1_f32,
            want: Want::Value(0.6480543_f32),
        },
        SechF32Case {
            x: -2_f32,
            want: Want::Value(0.2658022_f32),
        },
        SechF32Case {
            x: 5_f32,
            want: Want::Value(0.0134753_f32),
        },
    ]
}

#[test]
fn csch_f32_matches_test_cases() {
    for (i, case) in csch_f32_cases().iter().enumerate() {
        let mut cell = StateCell::bind(&crate::common::cell_src("csch_f32"), "CschF32", None)
            .unwrap_or_else(|e| panic!("bind csch_f32: {e}"));
        cell.set("x", (case.x).to_bits() as u64).unwrap();
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run csch_f32 case {i}: {e}"));
        match case.want {
            Want::Value(want) => {
                assert_eq!(
                    report.halt,
                    Halt::Returned,
                    "{} case {} report: {:?}",
                    "csch_f32",
                    i,
                    report
                );
                let got = f32::from_bits(cell.get("result").unwrap() as u32);
                assert!(
                    f32_tol(got, want),
                    "csch_f32 case {i}: got {got} want {want}",
                );
            }
            Want::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(code),
                    "csch_f32 case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

struct CschF32Case {
    x: f32,
    want: Want,
}

fn csch_f32_cases() -> Vec<CschF32Case> {
    vec![
        CschF32Case {
            x: 1_f32,
            want: Want::Value(0.850918_f32),
        },
        CschF32Case {
            x: 2_f32,
            want: Want::Value(0.275721_f32),
        },
        CschF32Case {
            x: -1_f32,
            want: Want::Value(-0.850918_f32),
        },
    ]
}

#[test]
fn acot_f32_matches_test_cases() {
    for (i, case) in acot_f32_cases().iter().enumerate() {
        let mut cell = StateCell::bind(&crate::common::cell_src("acot_f32"), "AcotF32", None)
            .unwrap_or_else(|e| panic!("bind acot_f32: {e}"));
        cell.set("x", (case.x).to_bits() as u64).unwrap();
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run acot_f32 case {i}: {e}"));
        match case.want {
            Want::Value(want) => {
                assert_eq!(
                    report.halt,
                    Halt::Returned,
                    "{} case {} report: {:?}",
                    "acot_f32",
                    i,
                    report
                );
                let got = f32::from_bits(cell.get("result").unwrap() as u32);
                assert!(
                    f32_tol(got, want),
                    "acot_f32 case {i}: got {got} want {want}",
                );
            }
            Want::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(code),
                    "acot_f32 case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

struct AcotF32Case {
    x: f32,
    want: Want,
}

fn acot_f32_cases() -> Vec<AcotF32Case> {
    vec![
        AcotF32Case {
            x: 1_f32,
            want: Want::Value(0.7853981633974483_f32),
        },
        AcotF32Case {
            x: 0_f32,
            want: Want::Value(1.5707963267948966_f32),
        },
        AcotF32Case {
            x: -1_f32,
            want: Want::Value(2.356194490192345_f32),
        },
        AcotF32Case {
            x: 1.7320508_f32,
            want: Want::Value(0.5235987755982989_f32),
        },
        AcotF32Case {
            x: -1.7320508_f32,
            want: Want::Value(2.6179938779914944_f32),
        },
        AcotF32Case {
            x: 1000_f32,
            want: Want::Value(0.0009999996666667_f32),
        },
        AcotF32Case {
            x: -1000_f32,
            want: Want::Value(3.140592653923126_f32),
        },
    ]
}

#[test]
fn acoth_f32_matches_test_cases() {
    for (i, case) in acoth_f32_cases().iter().enumerate() {
        let mut cell = StateCell::bind(&crate::common::cell_src("acoth_f32"), "AcothF32", None)
            .unwrap_or_else(|e| panic!("bind acoth_f32: {e}"));
        cell.set("x", (case.x).to_bits() as u64).unwrap();
        let report = cell
            .run(cell80::DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run acoth_f32 case {i}: {e}"));
        match case.want {
            Want::Value(want) => {
                assert_eq!(
                    report.halt,
                    Halt::Returned,
                    "{} case {} report: {:?}",
                    "acoth_f32",
                    i,
                    report
                );
                let got = f32::from_bits(cell.get("result").unwrap() as u32);
                assert!(
                    f32_tol(got, want),
                    "acoth_f32 case {i}: got {got} want {want}",
                );
            }
            Want::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(code),
                    "acoth_f32 case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

struct AcothF32Case {
    x: f32,
    want: Want,
}

fn acoth_f32_cases() -> Vec<AcothF32Case> {
    vec![
        AcothF32Case {
            x: 2_f32,
            want: Want::Value(0.5493061443340549_f32),
        },
        AcothF32Case {
            x: -2_f32,
            want: Want::Value(-0.5493061443340549_f32),
        },
        AcothF32Case {
            x: 1.5_f32,
            want: Want::Value(0.8047189562170501_f32),
        },
    ]
}
