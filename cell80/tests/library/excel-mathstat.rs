//! Host-oracle tests for the excel-mathstat pack (`cell80/cells/excel-mathstat/*.rs`) --
//! mechanically generated from each cell's own proposed test cases (the 19-cell
//! composable-author + Excel Math&Trig/Statistical coverage-map batch, verify->admit loop)
//! rather than hand-transcribed: every `test_cases` entry from the authoring output becomes
//! one comparison, run against the real compiled cell via `StateCell::bind`/`set`/`run`/`get`
//! (state-cell entries) or `crate::common::run_cell` (free-function entries) -- see
//! `cell80/tests/library/common.rs` for the shared helpers. f32 fields ride raw bit patterns
//! (`to_bits`/`from_bits`, the physics/softfloat packs' own convention) and compare with a
//! small relative-tolerance epsilon rather than bit-exactness, matching
//! `excel-financial.rs`/`excel-datetime.rs`'s own f32 test convention.
//
// Mechanically generated scaffolds: single-type cells degenerate to `match name
// { _ => .. }` and every case table shares one tuple shape -- style lints the
// generator would re-trip next wave are allowed rather than hand-patched.
#![allow(
    clippy::match_single_binding,
    clippy::type_complexity,
    clippy::excessive_precision,
    clippy::approx_constant
)]

use crate::common::{cell_src, run_cell};
use cell80::{Halt, Runner, StateCell, DEFAULT_CYCLES};

fn i16_bits(v: i16) -> u16 {
    v as u16
}

fn f32_tol(got: f32, want: f32) -> bool {
    (got - want).abs() < (want.abs() * 1e-3_f32).max(1e-3_f32)
}

#[test]
fn excel_ceiling_math_matches_test_cases() {
    let cases: &[((u64, u64, u64, u64), &str, u64)] = &[
        ((12, 0, 4, 0), "result_mag", 12),
        ((7, 0, 4, 0), "result_mag", 8),
        ((7, 1, 4, 0), "result_mag", 4),
        ((7, 1, 4, 0), "result_neg", 1),
        ((7, 1, 4, 1), "result_mag", 8),
        ((8, 1, 4, 0), "result_neg", 1),
        ((0, 0, 4, 0), "result_mag", 0),
        ((5, 0, 0, 0), "result_mag", 0),
    ];
    for (i, ((number_mag, number_neg, significance, mode), field, want)) in cases.iter().enumerate()
    {
        let mut cell = StateCell::bind(&cell_src("excel_ceiling_math"), "ExcelCeilingMath", None)
            .unwrap_or_else(|e| panic!("bind excel_ceiling_math: {e}"));
        cell.set("number_mag", *number_mag).unwrap();
        cell.set("number_neg", *number_neg).unwrap();
        cell.set("significance", *significance).unwrap();
        cell.set("mode", *mode).unwrap();
        let report = cell
            .run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_ceiling_math case {i}: {e}"));
        assert_eq!(report.halt, Halt::Returned, "excel_ceiling_math case {i}");
        assert_eq!(
            cell.get(field),
            Some(*want),
            "excel_ceiling_math case {i}: field {field}"
        );
    }
}

#[test]
fn excel_ceiling_precise_matches_test_cases() {
    let cases: &[((u64, u64, u64), &str, u64)] = &[
        ((10, 0, 3), "result_mag", 12),
        ((10, 0, 3), "result_neg", 0),
        ((12, 0, 4), "result_mag", 12),
        ((12, 0, 4), "result_neg", 0),
        ((7, 1, 3), "result_mag", 6),
        ((7, 1, 3), "result_neg", 1),
        ((9, 1, 3), "result_mag", 9),
        ((9, 1, 3), "result_neg", 1),
        ((2, 1, 5), "result_mag", 0),
        ((2, 1, 5), "result_neg", 0),
        ((5, 0, 0), "result_mag", 0),
        ((5, 0, 0), "result_neg", 0),
    ];
    for (i, ((number_mag, number_neg, sig), field, want)) in cases.iter().enumerate() {
        let mut cell = StateCell::bind(
            &cell_src("excel_ceiling_precise"),
            "ExcelCeilingPrecise",
            None,
        )
        .unwrap_or_else(|e| panic!("bind excel_ceiling_precise: {e}"));
        cell.set("number_mag", *number_mag).unwrap();
        cell.set("number_neg", *number_neg).unwrap();
        cell.set("sig", *sig).unwrap();
        let report = cell
            .run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_ceiling_precise case {i}: {e}"));
        assert_eq!(
            report.halt,
            Halt::Returned,
            "excel_ceiling_precise case {i}"
        );
        assert_eq!(
            cell.get(field),
            Some(*want),
            "excel_ceiling_precise case {i}: field {field}"
        );
    }
}

#[test]
fn excel_even_matches_test_cases() {
    let cases: &[(u16, u16)] = &[
        (0, 0),
        (1, 2),
        (2, 2),
        (3, 4),
        (65535, 65534),
        (65533, 65532),
        (65532, 65532),
        (32768, 32768),
    ];
    for (i, (number, want)) in cases.iter().enumerate() {
        assert_eq!(
            run_cell("excel_even", &[*number]),
            *want,
            "excel_even case {i}"
        );
    }

    // number = 32767 (i16::MAX, odd): rounding up overflows a positive i16 -> escalate
    // needs_wider_math (halt 0xFF05 = 65285 decimal).
    let mut r = Runner::compile(&cell_src("excel_even")).unwrap();
    let report = r.run(None, &[32767], DEFAULT_CYCLES).unwrap();
    assert_eq!(report.halt, Halt::Escalate(0xFF05));
}

#[test]
fn excel_floor_math_matches_test_cases() {
    let cases: &[((u64, u64, u64, u64), &str, u64)] = &[
        ((24, 0, 5, 0), "result_mag", 20),
        ((24, 0, 5, 1), "result_mag", 20),
        ((20, 0, 5, 0), "result_mag", 20),
        ((8, 1, 3, 0), "result_mag", 9),
        ((8, 1, 3, 0), "result_neg", 1),
        ((8, 1, 3, 1), "result_mag", 6),
        ((3, 1, 10, 1), "result_mag", 0),
        ((3, 1, 10, 1), "result_neg", 0),
        ((17, 0, 0, 0), "result_mag", 0),
        ((65535, 1, 7, 0), "result_mag", 65541),
    ];
    for (i, ((num_mag, num_neg, sig, mode), field, want)) in cases.iter().enumerate() {
        let mut cell = StateCell::bind(&cell_src("excel_floor_math"), "ExcelFloorMath", None)
            .unwrap_or_else(|e| panic!("bind excel_floor_math: {e}"));
        cell.set("num_mag", *num_mag).unwrap();
        cell.set("num_neg", *num_neg).unwrap();
        cell.set("sig", *sig).unwrap();
        cell.set("mode", *mode).unwrap();
        let report = cell
            .run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_floor_math case {i}: {e}"));
        assert_eq!(report.halt, Halt::Returned, "excel_floor_math case {i}");
        assert_eq!(
            cell.get(field),
            Some(*want),
            "excel_floor_math case {i}: field {field}"
        );
    }
}

#[test]
fn excel_mod_matches_test_cases() {
    let cases: &[((i16, i16), i16)] = &[
        ((3, 2), 1),
        ((-3, 2), 1),
        ((3, -2), -1),
        ((-3, -2), -1),
        ((10, 5), 0),
        ((-7, 3), 2),
        ((7, 3), 1),
    ];
    for (i, ((number, divisor), want)) in cases.iter().enumerate() {
        let got = run_cell("excel_mod", &[i16_bits(*number), i16_bits(*divisor)]);
        assert_eq!(got, i16_bits(*want), "excel_mod case {i}");
    }
}

// excel_mround was authored and verified (MROUND(10,3)=9, MROUND(100000,7)=100002, sign-
// mismatch escalation, etc. all matched by hand) but backed out at the admission-gate step:
// its (u32,u16,u32,u16,u32,u16) field shape exactly matches checked-arithmetic/smag_max.rs
// (mag_a,neg_a,mag_b,neg_b,mag,neg), and the fingerprint probe bank assigns fields
// cyclically by declaration order, so `mult_neg`/`neg_b` both land on `probe[0]` -- a
// value that is >1 (an invalid sign flag) on nearly every DEFAULT_PROBES entry, forcing
// both cells to escalate identically on the same out-of-domain guard almost every time and
// never actually reaching a probe where MROUND's rounding and MAX's comparison would
// visibly disagree (e.g. MROUND(10,3)=9 vs max(10,3)=10, a real divergence the bank never
// reaches). A probe-bank coincidence, not a true behavioural duplicate -- but the standing
// rule backs out the new cell on any flagged pair regardless (docs/library-growth.md).

#[test]
fn excel_floor_precise_matches_test_cases() {
    fn step(number: f32, significance: f32, sig_omitted: u64) -> StateCell {
        let mut cell = StateCell::bind(&cell_src("excel_floor_precise"), "ExcelFloorPrecise", None)
            .unwrap_or_else(|e| panic!("bind excel_floor_precise: {e}"));
        cell.set("number", number.to_bits() as u64).unwrap();
        cell.set("significance", significance.to_bits() as u64)
            .unwrap();
        cell.set("sig_omitted", sig_omitted).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        assert_eq!(report.halt, Halt::Returned);
        cell
    }
    let cases: &[((f32, f32, u64), f32)] = &[
        ((10.0, 3.0, 0), 9.0),
        ((-10.0, 3.0, 0), -12.0),
        ((-10.0, -3.0, 0), -12.0),
        ((7.5, 0.0, 1), 7.0),
        ((-2.5, 0.0, 1), -3.0),
        ((123.456, 0.0, 0), 0.0),
        ((9.0, 3.0, 0), 9.0),
    ];
    for (i, ((number, significance, sig_omitted), want)) in cases.iter().enumerate() {
        let cell = step(*number, *significance, *sig_omitted);
        let got = f32::from_bits(cell.get("result").unwrap() as u32);
        assert!(
            f32_tol(got, *want),
            "excel_floor_precise case {i}: got {got} want {want}"
        );
    }
}

#[test]
fn excel_iso_ceiling_matches_test_cases() {
    let cases: &[((i16, i16), i16)] = &[
        ((1152i16, 256i16), 1280i16),
        ((-1152i16, 256i16), -1024i16),
        ((-1536i16, -1024i16), -1024i16),
        ((1792i16, -768i16), 2304i16),
        ((2048i16, 1024i16), 2048i16),
        ((1280i16, 0i16), 0i16),
        ((265i16, 64i16), 320i16),
    ];
    for (i, ((number_q8, significance_q8), want)) in cases.iter().enumerate() {
        let got = run_cell(
            "excel_iso_ceiling",
            &[i16_bits(*number_q8), i16_bits(*significance_q8)],
        );
        assert_eq!(got, i16_bits(*want), "excel_iso_ceiling case {i}");
    }
}

#[test]
fn excel_odd_matches_test_cases() {
    fn step(number: f32) -> f32 {
        let mut cell = StateCell::bind(&cell_src("excel_odd"), "ExcelOdd", None)
            .unwrap_or_else(|e| panic!("bind excel_odd: {e}"));
        cell.set("number", number.to_bits() as u64).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        assert_eq!(report.halt, Halt::Returned);
        f32::from_bits(cell.get("result").unwrap() as u32)
    }
    let cases: &[(f32, f32)] = &[
        (1.5, 3.0),
        (3.0, 3.0),
        (-1.5, -3.0),
        (2.0, 3.0),
        (0.0, 1.0),
        (-4.0, -5.0),
    ];
    for (i, (number, want)) in cases.iter().enumerate() {
        let got = step(*number);
        assert!(
            f32_tol(got, *want),
            "excel_odd case {i}: got {got} want {want}"
        );
    }
}

#[test]
fn excel_round_matches_test_cases() {
    fn step(number: f32, digits_mag: u64, digits_neg: u64) -> f32 {
        let mut cell = StateCell::bind(&cell_src("excel_round"), "ExcelRound", None)
            .unwrap_or_else(|e| panic!("bind excel_round: {e}"));
        cell.set("number", number.to_bits() as u64).unwrap();
        cell.set("digits_mag", digits_mag).unwrap();
        cell.set("digits_neg", digits_neg).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        assert_eq!(report.halt, Halt::Returned);
        f32::from_bits(cell.get("result").unwrap() as u32)
    }
    let cases: &[((f32, u64, u64), f32)] = &[
        ((1.5, 0, 0), 2.0),
        ((-1.5, 0, 0), -2.0),
        ((3.14159, 2, 0), 3.14),
        ((1250.0, 2, 1), 1300.0),
        ((-1250.0, 2, 1), -1300.0),
        ((0.126, 2, 0), 0.13),
    ];
    for (i, ((number, digits_mag, digits_neg), want)) in cases.iter().enumerate() {
        let got = step(*number, *digits_mag, *digits_neg);
        assert!(
            f32_tol(got, *want),
            "excel_round case {i}: got {got} want {want}"
        );
    }
}

#[test]
fn excel_rounddown_matches_test_cases() {
    let cases: &[((u64, u64, u64, u64), &str, u64)] = &[
        ((2360, 0, 2, 1), "result_mag", 2300),
        ((2360, 1, 2, 1), "result_neg", 1),
        ((1234, 0, 3, 0), "result_mag", 1234),
        ((9876, 1, 0, 0), "result_neg", 1),
        ((50, 1, 3, 1), "result_neg", 0),
        ((65535, 0, 1, 1), "result_mag", 65530),
    ];
    for (i, ((mag, neg, digits_mag, digits_neg), field, want)) in cases.iter().enumerate() {
        let mut cell = StateCell::bind(&cell_src("excel_rounddown"), "ExcelRounddown", None)
            .unwrap_or_else(|e| panic!("bind excel_rounddown: {e}"));
        cell.set("mag", *mag).unwrap();
        cell.set("neg", *neg).unwrap();
        cell.set("digits_mag", *digits_mag).unwrap();
        cell.set("digits_neg", *digits_neg).unwrap();
        let report = cell
            .run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_rounddown case {i}: {e}"));
        assert_eq!(report.halt, Halt::Returned, "excel_rounddown case {i}");
        assert_eq!(
            cell.get(field),
            Some(*want),
            "excel_rounddown case {i}: field {field}"
        );
    }
}

#[test]
fn excel_roundup_matches_test_cases() {
    let cases: &[((u64, u64, u64, u64), &str, u64)] = &[
        ((125, 0, 1, 1), "result_mag", 130),
        ((125, 1, 2, 1), "result_mag", 200),
        ((125, 1, 2, 1), "result_neg", 1),
        ((7, 1, 0, 0), "result_mag", 7),
        ((100, 0, 1, 1), "result_mag", 100),
        ((999, 0, 2, 1), "result_mag", 1000),
        ((0, 1, 3, 1), "result_mag", 0),
    ];
    for (i, ((num_mag, num_neg, digits_mag, digits_neg), field, want)) in cases.iter().enumerate() {
        let mut cell = StateCell::bind(&cell_src("excel_roundup"), "ExcelRoundUp", None)
            .unwrap_or_else(|e| panic!("bind excel_roundup: {e}"));
        cell.set("num_mag", *num_mag).unwrap();
        cell.set("num_neg", *num_neg).unwrap();
        cell.set("digits_mag", *digits_mag).unwrap();
        cell.set("digits_neg", *digits_neg).unwrap();
        let report = cell
            .run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_roundup case {i}: {e}"));
        assert_eq!(report.halt, Halt::Returned, "excel_roundup case {i}");
        assert_eq!(
            cell.get(field),
            Some(*want),
            "excel_roundup case {i}: field {field}"
        );
    }
}

#[test]
fn excel_sqrt_matches_test_cases() {
    fn step(number: f32) -> f32 {
        let mut cell = StateCell::bind(&cell_src("excel_sqrt"), "ExcelSqrt", None)
            .unwrap_or_else(|e| panic!("bind excel_sqrt: {e}"));
        cell.set("number", number.to_bits() as u64).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        assert_eq!(report.halt, Halt::Returned);
        f32::from_bits(cell.get("result").unwrap() as u32)
    }
    let cases: &[(f32, f32)] = &[
        (4.0, 2.0),
        (9.0, 3.0),
        (2.0, 1.4142135381698608),
        (0.0, 0.0),
    ];
    for (i, (number, want)) in cases.iter().enumerate() {
        let got = step(*number);
        assert!(
            f32_tol(got, *want),
            "excel_sqrt case {i}: got {got} want {want}"
        );
    }

    // Negative radicand -> escalate out_of_domain (Excel's own #NUM!).
    let mut cell = StateCell::bind(&cell_src("excel_sqrt"), "ExcelSqrt", None).unwrap();
    cell.set("number", (-1.0f32).to_bits() as u64).unwrap();
    let report = cell.run(DEFAULT_CYCLES).unwrap();
    assert_eq!(report.halt, Halt::Escalate(0xFF06));
}

#[test]
fn excel_trunc_matches_test_cases() {
    fn step(number: f32, num_digits: u64) -> f32 {
        let mut cell = StateCell::bind(&cell_src("excel_trunc"), "ExcelTrunc", None)
            .unwrap_or_else(|e| panic!("bind excel_trunc: {e}"));
        cell.set("number", number.to_bits() as u64).unwrap();
        cell.set("num_digits", num_digits).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        assert_eq!(report.halt, Halt::Returned);
        f32::from_bits(cell.get("result").unwrap() as u32)
    }
    let cases: &[((f32, u64), f32)] = &[
        ((8.9, 0), 8.0),
        ((-8.9, 0), -8.0),
        ((76.8, 0), 76.0),
        ((3.14159, 2), 3.14),
        ((-3.14159, 2), -3.14),
        ((9.999, 1), 9.9),
    ];
    for (i, ((number, num_digits), want)) in cases.iter().enumerate() {
        let got = step(*number, *num_digits);
        assert!(
            f32_tol(got, *want),
            "excel_trunc case {i}: got {got} want {want}"
        );
    }
}

#[test]
fn excel_radians_matches_test_cases() {
    let cases: &[(u16, u16)] = &[
        (0, 0),
        (45, 201),
        (90, 402),
        (180, 804),
        (360, 1608),
        (14672, 65534),
    ];
    for (i, (degrees, want)) in cases.iter().enumerate() {
        assert_eq!(
            run_cell("excel_radians", &[*degrees]),
            *want,
            "excel_radians case {i}"
        );
    }
}

#[test]
fn excel_degrees_matches_test_cases() {
    fn step(angle: f32) -> f32 {
        let mut cell = StateCell::bind(&cell_src("excel_degrees"), "ExcelDegrees", None)
            .unwrap_or_else(|e| panic!("bind excel_degrees: {e}"));
        cell.set("angle", angle.to_bits() as u64).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        assert_eq!(report.halt, Halt::Returned);
        f32::from_bits(cell.get("result").unwrap() as u32)
    }
    let cases: &[(f32, f32)] = &[
        (0.0, 0.0),
        (1.0, 57.29577951308232),
        (3.14159265, 180.0),
        (-1.5707963, -90.0),
        (2.5, 143.2394487827058),
    ];
    for (i, (angle, want)) in cases.iter().enumerate() {
        let got = step(*angle);
        assert!(
            f32_tol(got, *want),
            "excel_degrees case {i}: got {got} want {want}"
        );
    }
}

// -- array-reduction batch (SUM/AVERAGE/MAX/MIN/MEDIAN/STDEV.P/STDEV.S/VAR.S/LARGE/
// SMALL/SUMSQ/COUNTBLANK) -- mechanically generated the same way as the rest of this
// file: every `test_cases` entry from the authoring output becomes one comparison,
// run against the real compiled cell via `StateCell::bind`/`set`/`set_array`/`run`/
// `get`. Array-state fields (`values: [u32; 16]`, carrying f32 bit patterns) ride
// `StateCell::set_array` -- see `cell80/src/state.rs` for the exact API -- rather than
// the scalar `set` every other cell in this file uses. A domain/overflow-escalation
// case (`halt`/`halt_code` expected_field in the authoring output) asserts
// `Halt::Escalate(code)` instead of a numeric comparison. Two count=17 cases
// (`excel_min`, `excel_stdev_p`) supplied 17 raw values in the authoring output for
// an arity-17 escalation case, but the `[u32; 16]` envelope can only ever hold 16 --
// `StateCell::set_array` itself rejects anything longer, before the cell even runs --
// so those two are truncated to the first 16 here; the escalation fires on `count >
// 16` before any array indexing regardless of what the (never-read) 16 live slots hold.

enum ArrOutcome { Value(f32), Halt(u16) }

#[test]
fn excel_sum_matches_test_cases() {
    let cases: &[(&[f32], u16, ArrOutcome)] = &[
        (&[3_f32, 2_f32], 2u16, ArrOutcome::Value(5_f32)),
        (&[10_f32, 20_f32, 30_f32], 3u16, ArrOutcome::Value(60_f32)),
        (&[5.5_f32, -2.5_f32, 10_f32, -1_f32], 4u16, ArrOutcome::Value(12_f32)),
        (&[42_f32], 1u16, ArrOutcome::Value(42_f32)),
        (&[1_f32, 2_f32, 3_f32, 4_f32, 5_f32, 6_f32, 7_f32, 8_f32, 9_f32, 10_f32, 11_f32, 12_f32, 13_f32, 14_f32, 15_f32, 16_f32], 16u16, ArrOutcome::Value(136_f32)),
    ];
    for (i, (values, count, want)) in cases.iter().enumerate() {
        let mut cell = StateCell::bind(&cell_src("excel_sum"), "ExcelSum", None)
            .unwrap_or_else(|e| panic!("bind excel_sum: {e}"));
        let bits: Vec<u64> = values.iter().map(|v| v.to_bits() as u64).collect();
        cell.set_array("values", &bits).unwrap();
        cell.set("count", *count as u64).unwrap();
        let report = cell
            .run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_sum case {i}: {e}"));
        match want {
            ArrOutcome::Value(w) => {
                assert_eq!(report.halt, Halt::Returned, "excel_sum case {i}: {report:?}");
                let got = f32::from_bits(cell.get("total").unwrap() as u32);
                assert!(
                    f32_tol(got, *w),
                    "excel_sum case {i}: got {got} want {w}",
                );
            }
            ArrOutcome::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(*code),
                    "excel_sum case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

#[test]
fn excel_average_matches_test_cases() {
    let cases: &[(&[f32], u16, ArrOutcome)] = &[
        (&[10_f32, 15_f32, 32_f32], 3u16, ArrOutcome::Value(19_f32)),
        (&[42_f32], 1u16, ArrOutcome::Value(42_f32)),
        (&[1_f32, 2_f32], 2u16, ArrOutcome::Value(1.5_f32)),
        (&[-5_f32, 5_f32, 10_f32], 3u16, ArrOutcome::Value(3.3333333_f32)),
        (&[1_f32, 2_f32, 3_f32, 4_f32, 5_f32, 6_f32, 7_f32, 8_f32, 9_f32, 10_f32, 11_f32, 12_f32, 13_f32, 14_f32, 15_f32, 16_f32], 16u16, ArrOutcome::Value(8.5_f32)),
        (&[7_f32, 7_f32, 7_f32, 7_f32], 4u16, ArrOutcome::Value(7_f32)),
        (&[0_f32, 0_f32, 0_f32], 3u16, ArrOutcome::Value(0_f32)),
        (&[], 0u16, ArrOutcome::Halt(65286)),
        (&[1_f32, 1_f32, 1_f32, 1_f32, 1_f32, 1_f32, 1_f32, 1_f32, 1_f32, 1_f32, 1_f32, 1_f32, 1_f32, 1_f32, 1_f32, 1_f32], 17u16, ArrOutcome::Halt(65286)),
    ];
    for (i, (values, count, want)) in cases.iter().enumerate() {
        let mut cell = StateCell::bind(&cell_src("excel_average"), "ExcelAverage", None)
            .unwrap_or_else(|e| panic!("bind excel_average: {e}"));
        let bits: Vec<u64> = values.iter().map(|v| v.to_bits() as u64).collect();
        cell.set_array("values", &bits).unwrap();
        cell.set("count", *count as u64).unwrap();
        let report = cell
            .run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_average case {i}: {e}"));
        match want {
            ArrOutcome::Value(w) => {
                assert_eq!(report.halt, Halt::Returned, "excel_average case {i}: {report:?}");
                let got = f32::from_bits(cell.get("average").unwrap() as u32);
                assert!(
                    f32_tol(got, *w),
                    "excel_average case {i}: got {got} want {w}",
                );
            }
            ArrOutcome::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(*code),
                    "excel_average case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

#[test]
fn excel_max_matches_test_cases() {
    let cases: &[(&[f32], u16, ArrOutcome)] = &[
        (&[3_f32, 8_f32, -1_f32, 8_f32, 2.5_f32], 5u16, ArrOutcome::Value(8_f32)),
        (&[-5_f32, -12_f32, -3_f32, -100_f32], 4u16, ArrOutcome::Value(-3_f32)),
        (&[42_f32], 1u16, ArrOutcome::Value(42_f32)),
        (&[10_f32, 10_f32, 5_f32], 3u16, ArrOutcome::Value(10_f32)),
        (&[1_f32, 2_f32, 3_f32, 4_f32, 5_f32, 6_f32, 7_f32, 8_f32, 9_f32, 10_f32, 11_f32, 12_f32, 13_f32, 14_f32, 15_f32, 16_f32], 16u16, ArrOutcome::Value(16_f32)),
        (&[99_f32, 1_f32, 2_f32, 3_f32], 4u16, ArrOutcome::Value(99_f32)),
        (&[1.25_f32, 1.5_f32, 1.125_f32], 3u16, ArrOutcome::Value(1.5_f32)),
    ];
    for (i, (values, count, want)) in cases.iter().enumerate() {
        let mut cell = StateCell::bind(&cell_src("excel_max"), "ExcelMax", None)
            .unwrap_or_else(|e| panic!("bind excel_max: {e}"));
        let bits: Vec<u64> = values.iter().map(|v| v.to_bits() as u64).collect();
        cell.set_array("values", &bits).unwrap();
        cell.set("count", *count as u64).unwrap();
        let report = cell
            .run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_max case {i}: {e}"));
        match want {
            ArrOutcome::Value(w) => {
                assert_eq!(report.halt, Halt::Returned, "excel_max case {i}: {report:?}");
                let got = f32::from_bits(cell.get("max").unwrap() as u32);
                assert!(
                    f32_tol(got, *w),
                    "excel_max case {i}: got {got} want {w}",
                );
            }
            ArrOutcome::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(*code),
                    "excel_max case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

#[test]
fn excel_min_matches_test_cases() {
    let cases: &[(&[f32], u16, ArrOutcome)] = &[
        (&[5_f32, 3_f32, 9_f32, 1_f32, 7_f32], 5u16, ArrOutcome::Value(1_f32)),
        (&[-2.5_f32, -8.25_f32, -1_f32], 3u16, ArrOutcome::Value(-8.25_f32)),
        (&[42_f32], 1u16, ArrOutcome::Value(42_f32)),
        (&[16_f32, 15_f32, 14_f32, 13_f32, 12_f32, 11_f32, 10_f32, 9_f32, 8_f32, 7_f32, 6_f32, 5_f32, 4_f32, 3_f32, 2_f32, 1_f32], 16u16, ArrOutcome::Value(1_f32)),
        (&[], 0u16, ArrOutcome::Halt(65286)),
        (&[1_f32, 2_f32, 3_f32, 4_f32, 5_f32, 6_f32, 7_f32, 8_f32, 9_f32, 10_f32, 11_f32, 12_f32, 13_f32, 14_f32, 15_f32, 16_f32], 17u16, ArrOutcome::Halt(65286)),
        (&[f32::NAN], 1u16, ArrOutcome::Halt(65288)),
    ];
    for (i, (values, count, want)) in cases.iter().enumerate() {
        let mut cell = StateCell::bind(&cell_src("excel_min"), "ExcelMin", None)
            .unwrap_or_else(|e| panic!("bind excel_min: {e}"));
        let bits: Vec<u64> = values.iter().map(|v| v.to_bits() as u64).collect();
        cell.set_array("values", &bits).unwrap();
        cell.set("count", *count as u64).unwrap();
        let report = cell
            .run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_min case {i}: {e}"));
        match want {
            ArrOutcome::Value(w) => {
                assert_eq!(report.halt, Halt::Returned, "excel_min case {i}: {report:?}");
                let got = f32::from_bits(cell.get("min").unwrap() as u32);
                assert!(
                    f32_tol(got, *w),
                    "excel_min case {i}: got {got} want {w}",
                );
            }
            ArrOutcome::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(*code),
                    "excel_min case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

#[test]
fn excel_median_matches_test_cases() {
    let cases: &[(&[f32], u16, ArrOutcome)] = &[
        (&[3_f32, 1_f32, 2_f32], 3u16, ArrOutcome::Value(2_f32)),
        (&[1_f32, 2_f32, 3_f32, 4_f32], 4u16, ArrOutcome::Value(2.5_f32)),
        (&[7_f32, 7_f32, 1_f32, 3_f32, 9_f32, 2_f32, 5_f32], 7u16, ArrOutcome::Value(5_f32)),
        (&[-5_f32, 10_f32, 0_f32, -2_f32, 8_f32, -1_f32], 6u16, ArrOutcome::Value(-0.5_f32)),
        (&[42_f32], 1u16, ArrOutcome::Value(42_f32)),
        (&[16_f32, 15_f32, 14_f32, 13_f32, 12_f32, 11_f32, 10_f32, 9_f32, 8_f32, 7_f32, 6_f32, 5_f32, 4_f32, 3_f32, 2_f32, 1_f32], 16u16, ArrOutcome::Value(8.5_f32)),
    ];
    for (i, (values, count, want)) in cases.iter().enumerate() {
        let mut cell = StateCell::bind(&cell_src("excel_median"), "ExcelMedian", None)
            .unwrap_or_else(|e| panic!("bind excel_median: {e}"));
        let bits: Vec<u64> = values.iter().map(|v| v.to_bits() as u64).collect();
        cell.set_array("values", &bits).unwrap();
        cell.set("count", *count as u64).unwrap();
        let report = cell
            .run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_median case {i}: {e}"));
        match want {
            ArrOutcome::Value(w) => {
                assert_eq!(report.halt, Halt::Returned, "excel_median case {i}: {report:?}");
                let got = f32::from_bits(cell.get("median").unwrap() as u32);
                assert!(
                    f32_tol(got, *w),
                    "excel_median case {i}: got {got} want {w}",
                );
            }
            ArrOutcome::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(*code),
                    "excel_median case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

#[test]
fn excel_stdev_p_matches_test_cases() {
    let cases: &[(&[f32], u16, ArrOutcome)] = &[
        (&[2_f32, 4_f32, 4_f32, 4_f32, 5_f32, 5_f32, 7_f32, 9_f32], 8u16, ArrOutcome::Value(2_f32)),
        (&[42_f32], 1u16, ArrOutcome::Value(0_f32)),
        (&[10_f32, 20_f32], 2u16, ArrOutcome::Value(5_f32)),
        (&[-2_f32, -1_f32, 0_f32, 1_f32, 2_f32], 5u16, ArrOutcome::Value(1.4142135623730951_f32)),
        (&[7_f32, 7_f32, 7_f32, 7_f32], 4u16, ArrOutcome::Value(0_f32)),
        (&[1_f32, 2_f32, 3_f32, 4_f32, 5_f32, 6_f32, 7_f32, 8_f32, 9_f32, 10_f32, 11_f32, 12_f32, 13_f32, 14_f32, 15_f32, 16_f32], 16u16, ArrOutcome::Value(4.609772228646444_f32)),
        (&[], 0u16, ArrOutcome::Halt(65286)),
        (&[1_f32, 2_f32, 3_f32, 4_f32, 5_f32, 6_f32, 7_f32, 8_f32, 9_f32, 10_f32, 11_f32, 12_f32, 13_f32, 14_f32, 15_f32, 16_f32], 17u16, ArrOutcome::Halt(65286)),
    ];
    for (i, (values, count, want)) in cases.iter().enumerate() {
        let mut cell = StateCell::bind(&cell_src("excel_stdev_p"), "ExcelStdevP", None)
            .unwrap_or_else(|e| panic!("bind excel_stdev_p: {e}"));
        let bits: Vec<u64> = values.iter().map(|v| v.to_bits() as u64).collect();
        cell.set_array("values", &bits).unwrap();
        cell.set("count", *count as u64).unwrap();
        let report = cell
            .run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_stdev_p case {i}: {e}"));
        match want {
            ArrOutcome::Value(w) => {
                assert_eq!(report.halt, Halt::Returned, "excel_stdev_p case {i}: {report:?}");
                let got = f32::from_bits(cell.get("result").unwrap() as u32);
                assert!(
                    f32_tol(got, *w),
                    "excel_stdev_p case {i}: got {got} want {w}",
                );
            }
            ArrOutcome::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(*code),
                    "excel_stdev_p case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

#[test]
fn excel_stdev_s_matches_test_cases() {
    let cases: &[(&[f32], u16, ArrOutcome)] = &[
        (&[2_f32, 4_f32, 4_f32, 4_f32, 5_f32, 5_f32, 7_f32, 9_f32], 8u16, ArrOutcome::Value(2.1380899_f32)),
        (&[10_f32, 20_f32], 2u16, ArrOutcome::Value(7.0710678_f32)),
        (&[1_f32, 2_f32, 3_f32], 3u16, ArrOutcome::Value(1_f32)),
        (&[-2_f32, -1_f32, 0_f32, 1_f32, 2_f32], 5u16, ArrOutcome::Value(1.5811388_f32)),
        (&[1_f32, 2_f32, 3_f32, 4_f32, 5_f32, 6_f32, 7_f32, 8_f32, 9_f32, 10_f32, 11_f32, 12_f32, 13_f32, 14_f32, 15_f32, 16_f32], 16u16, ArrOutcome::Value(4.7609523_f32)),
    ];
    for (i, (values, count, want)) in cases.iter().enumerate() {
        let mut cell = StateCell::bind(&cell_src("excel_stdev_s"), "ExcelStdevS", None)
            .unwrap_or_else(|e| panic!("bind excel_stdev_s: {e}"));
        let bits: Vec<u64> = values.iter().map(|v| v.to_bits() as u64).collect();
        cell.set_array("values", &bits).unwrap();
        cell.set("count", *count as u64).unwrap();
        let report = cell
            .run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_stdev_s case {i}: {e}"));
        match want {
            ArrOutcome::Value(w) => {
                assert_eq!(report.halt, Halt::Returned, "excel_stdev_s case {i}: {report:?}");
                let got = f32::from_bits(cell.get("result").unwrap() as u32);
                assert!(
                    f32_tol(got, *w),
                    "excel_stdev_s case {i}: got {got} want {w}",
                );
            }
            ArrOutcome::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(*code),
                    "excel_stdev_s case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

// excel_var_s was authored, verified, and mechanically tested (all cases above
// passed against the real compiled cell), but backed out at the admission-gate
// step: its `(values: [u32;16], count: u16, variance: f32)` shape is identical to
// excel_stdev_s's, and the fingerprint's array-probing path feeds small raw
// integers (0..~2500) directly into the u32 array slots with no f32-bit
// reinterpretation (unlike the scalar-field path, which already special-cases
// `Ty::F32` for exactly this reason -- see `cell80/src/fingerprint.rs`'s
// `Fingerprint::compute`). Every probe value is therefore a subnormal float once
// decoded via `f32_from_bits`, and squaring a subnormal underflows to exactly
// 0.0 -- so both a sum-of-squared-deviations (variance) and any other
// non-negative squared-deviation reduction collapse to bit-identical 0.0 on
// every probe. Confirmed directly (a scratch `Fingerprint::of` comparison):
// `excel_var_s` and `excel_stdev_s` agree 1.0, not because they're the same
// operation but because the probe bank can't tell them apart once every input
// underflows. A genuine probe-bank coincidence, not a true behavioural
// duplicate -- but the standing rule backs out the new cell on any flagged pair
// regardless (docs/library-growth.md), the same call already made for
// `excel_mround` above. Root fix would need `ArrayElem`/`Ty::Array` to carry an
// f32 sub-kind so the array branch can apply the same `(v as f32).to_bits()`
// reinterpretation the scalar branch already does -- out of scope here, flagged
// for a future wave.

#[test]
fn excel_large_matches_test_cases() {
    let cases: &[(&[f32], u16, u16, ArrOutcome)] = &[
        (&[3_f32, 1_f32, 4_f32, 1_f32, 5_f32, 9_f32, 2_f32, 6_f32], 8u16, 1u16, ArrOutcome::Value(9_f32)),
        (&[3_f32, 1_f32, 4_f32, 1_f32, 5_f32, 9_f32, 2_f32, 6_f32], 8u16, 3u16, ArrOutcome::Value(5_f32)),
        (&[3_f32, 1_f32, 4_f32, 1_f32, 5_f32, 9_f32, 2_f32, 6_f32], 8u16, 8u16, ArrOutcome::Value(1_f32)),
        (&[-5_f32, 10_f32, -3_f32, 0_f32], 4u16, 2u16, ArrOutcome::Value(0_f32)),
        (&[42_f32], 1u16, 1u16, ArrOutcome::Value(42_f32)),
        (&[7_f32, 7_f32, 7_f32, 2_f32], 4u16, 3u16, ArrOutcome::Value(7_f32)),
        (&[1_f32, 2_f32, 3_f32], 3u16, 0u16, ArrOutcome::Halt(65286)),
        (&[1_f32, 2_f32, 3_f32], 3u16, 4u16, ArrOutcome::Halt(65286)),
        (&[], 0u16, 1u16, ArrOutcome::Halt(65286)),
        (&[1_f32, 2_f32, 3_f32, 4_f32, 5_f32, 6_f32, 7_f32, 8_f32, 9_f32, 10_f32, 11_f32, 12_f32, 13_f32, 14_f32, 15_f32, 16_f32], 17u16, 1u16, ArrOutcome::Halt(65286)),
    ];
    for (i, (values, count, k, want)) in cases.iter().enumerate() {
        let mut cell = StateCell::bind(&cell_src("excel_large"), "ExcelLarge", None)
            .unwrap_or_else(|e| panic!("bind excel_large: {e}"));
        let bits: Vec<u64> = values.iter().map(|v| v.to_bits() as u64).collect();
        cell.set_array("values", &bits).unwrap();
        cell.set("count", *count as u64).unwrap();
        cell.set("k", *k as u64).unwrap();
        let report = cell
            .run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_large case {i}: {e}"));
        match want {
            ArrOutcome::Value(w) => {
                assert_eq!(report.halt, Halt::Returned, "excel_large case {i}: {report:?}");
                let got = f32::from_bits(cell.get("result").unwrap() as u32);
                assert!(
                    f32_tol(got, *w),
                    "excel_large case {i}: got {got} want {w}",
                );
            }
            ArrOutcome::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(*code),
                    "excel_large case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

#[test]
fn excel_small_matches_test_cases() {
    let cases: &[(&[f32], u16, u16, ArrOutcome)] = &[
        (&[5_f32, 3_f32, 8_f32, 1_f32], 4u16, 1u16, ArrOutcome::Value(1_f32)),
        (&[5_f32, 3_f32, 8_f32, 1_f32], 4u16, 4u16, ArrOutcome::Value(8_f32)),
        (&[5_f32, 3_f32, 8_f32, 1_f32], 4u16, 2u16, ArrOutcome::Value(3_f32)),
        (&[1_f32, 2_f32, 2_f32, 3_f32], 4u16, 2u16, ArrOutcome::Value(2_f32)),
        (&[1_f32, 2_f32, 2_f32, 3_f32], 4u16, 3u16, ArrOutcome::Value(2_f32)),
        (&[-5_f32, 2_f32, -1_f32, 0_f32, 7_f32], 5u16, 2u16, ArrOutcome::Value(-1_f32)),
        (&[42_f32], 1u16, 1u16, ArrOutcome::Value(42_f32)),
        (&[16_f32, 15_f32, 14_f32, 13_f32, 12_f32, 11_f32, 10_f32, 9_f32, 8_f32, 7_f32, 6_f32, 5_f32, 4_f32, 3_f32, 2_f32, 1_f32], 16u16, 9u16, ArrOutcome::Value(9_f32)),
        (&[5_f32, 3_f32, 8_f32], 3u16, 0u16, ArrOutcome::Halt(65286)),
        (&[5_f32, 3_f32, 8_f32], 3u16, 4u16, ArrOutcome::Halt(65286)),
        (&[], 0u16, 1u16, ArrOutcome::Halt(65286)),
        (&[1_f32, 2_f32, 3_f32, 4_f32, 5_f32, 6_f32, 7_f32, 8_f32, 9_f32, 10_f32, 11_f32, 12_f32, 13_f32, 14_f32, 15_f32, 16_f32], 17u16, 1u16, ArrOutcome::Halt(65286)),
    ];
    for (i, (values, count, k, want)) in cases.iter().enumerate() {
        let mut cell = StateCell::bind(&cell_src("excel_small"), "ExcelSmall", None)
            .unwrap_or_else(|e| panic!("bind excel_small: {e}"));
        let bits: Vec<u64> = values.iter().map(|v| v.to_bits() as u64).collect();
        cell.set_array("values", &bits).unwrap();
        cell.set("count", *count as u64).unwrap();
        cell.set("k", *k as u64).unwrap();
        let report = cell
            .run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_small case {i}: {e}"));
        match want {
            ArrOutcome::Value(w) => {
                assert_eq!(report.halt, Halt::Returned, "excel_small case {i}: {report:?}");
                let got = f32::from_bits(cell.get("result").unwrap() as u32);
                assert!(
                    f32_tol(got, *w),
                    "excel_small case {i}: got {got} want {w}",
                );
            }
            ArrOutcome::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(*code),
                    "excel_small case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

// excel_sumsq was authored, verified, and mechanically tested (all cases above
// passed against the real compiled cell), but backed out at the admission-gate
// step for the identical reason `excel_var_s` was (see that cell's own note
// above): its `(values: [u32;16], count: u16, sumsq: f32)` shape collides with
// `excel_stdev_p`'s under the same subnormal-underflow probe-bank gap --
// confirmed a coincidence (`Fingerprint::of` agreement 1.0 against
// `excel_stdev_p`), not a true behavioural duplicate, but backed out per the
// standing rule regardless.

#[test]
fn excel_countblank_matches_test_cases() {
    // (blank_mask, count) -> want blanks. Hand-derived (the authoring output
    // only proposed one trivial all-zero case): bit i of blank_mask is 1 when
    // range slot i is blank; blanks = popcount(blank_mask & ((1 << count) - 1)).
    let cases: &[(u16, u16, u16)] = &[
        (0, 5, 0),                 // no bits set at all -> 0 blanks
        (0b0000_0000_0001_0110, 5, 3), // bits 1,2,4 set, all within count=5 -> 3
        (0xFFFF, 3, 3),             // every bit set, only first 3 slots counted -> 3
        (0b0000_0000_0000_1110, 2, 1), // bits 1,2,3 set; only bit1 is within count=2 -> 1
        (0xFFFF, 16, 16),           // full 16-slot envelope, every slot blank -> 16
    ];
    for (i, (blank_mask, count, want)) in cases.iter().enumerate() {
        let mut cell = StateCell::bind(&cell_src("excel_countblank"), "ExcelCountBlank", None)
            .unwrap_or_else(|e| panic!("bind excel_countblank: {e}"));
        cell.set("blank_mask", *blank_mask as u64).unwrap();
        cell.set("count", *count as u64).unwrap();
        let report = cell
            .run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_countblank case {i}: {e}"));
        assert_eq!(report.halt, Halt::Returned, "excel_countblank case {i}: {report:?}");
        assert_eq!(
            cell.get("blanks"),
            Some(*want as u64),
            "excel_countblank case {i}"
        );
    }

    // count == 0 and count > 16 both escalate out_of_domain before any counting.
    let mut cell = StateCell::bind(&cell_src("excel_countblank"), "ExcelCountBlank", None).unwrap();
    cell.set("blank_mask", 0).unwrap();
    cell.set("count", 0).unwrap();
    let report = cell.run(DEFAULT_CYCLES).unwrap();
    assert_eq!(report.halt, Halt::Escalate(0xFF06));

    let mut cell = StateCell::bind(&cell_src("excel_countblank"), "ExcelCountBlank", None).unwrap();
    cell.set("blank_mask", 0xFFFF).unwrap();
    cell.set("count", 17).unwrap();
    let report = cell.run(DEFAULT_CYCLES).unwrap();
    assert_eq!(report.halt, Halt::Escalate(0xFF06));
}

#[test]
fn excel_sumsq_matches_test_cases() {
    // Re-landed after the fingerprint array-probe fix (cell80/src/fingerprint.rs):
    // this cell was originally backed out as a false-positive admission-gate
    // duplicate of excel_stdev_p/excel_stdev_s (both collapsed to identical
    // fingerprints because u32 array elements were probed with raw small
    // integers -- subnormal floats whose squares all underflow to 0.0). The
    // gate now probes u32 array elements as small floats' bit patterns, so a
    // sum-of-squares reduction separates cleanly from a mean/variance reduction.
    let cases: &[(&[f32], u16, ArrOutcome)] = &[
        (&[3_f32, 4_f32], 2u16, ArrOutcome::Value(25_f32)),
        (&[5_f32], 1u16, ArrOutcome::Value(25_f32)),
        (&[-2_f32, 3_f32, -4_f32], 3u16, ArrOutcome::Value(29_f32)),
        (&[1_f32, 1_f32, 1_f32, 1_f32, 1_f32, 1_f32, 1_f32, 1_f32, 1_f32, 1_f32, 1_f32, 1_f32, 1_f32, 1_f32, 1_f32, 1_f32], 16u16, ArrOutcome::Value(16_f32)),
        (&[0_f32, 2.5_f32], 2u16, ArrOutcome::Value(6.25_f32)),
        (&[1.5_f32, 2.5_f32, 3.5_f32], 3u16, ArrOutcome::Value(20.75_f32)),
        (&[], 0u16, ArrOutcome::Halt(65286)),
        (&[1_f32, 2_f32, 3_f32, 4_f32, 5_f32, 6_f32, 7_f32, 8_f32, 9_f32, 10_f32, 11_f32, 12_f32, 13_f32, 14_f32, 15_f32, 16_f32], 17u16, ArrOutcome::Halt(65286)),
    ];
    for (i, (values, count, want)) in cases.iter().enumerate() {
        let mut cell = StateCell::bind(&cell_src("excel_sumsq"), "ExcelSumsq", None)
            .unwrap_or_else(|e| panic!("bind excel_sumsq: {e}"));
        let bits: Vec<u64> = values.iter().map(|v| v.to_bits() as u64).collect();
        cell.set_array("values", &bits).unwrap();
        cell.set("count", *count as u64).unwrap();
        let report = cell
            .run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_sumsq case {i}: {e}"));
        match want {
            ArrOutcome::Value(w) => {
                assert_eq!(report.halt, Halt::Returned, "excel_sumsq case {i}: {report:?}");
                let got = f32::from_bits(cell.get("sumsq").unwrap() as u32);
                assert!(
                    f32_tol(got, *w),
                    "excel_sumsq case {i}: got {got} want {w}",
                );
            }
            ArrOutcome::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(*code),
                    "excel_sumsq case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}

#[test]
fn excel_var_s_matches_test_cases() {
    // Re-landed after the same fingerprint array-probe fix as excel_sumsq above.
    let cases: &[(&[f32], u16, ArrOutcome)] = &[
        (&[2_f32, 4_f32, 4_f32, 4_f32, 5_f32, 5_f32, 7_f32, 9_f32], 8u16, ArrOutcome::Value(4.5714285714285_f32)),
        (&[10_f32, 20_f32], 2u16, ArrOutcome::Value(50_f32)),
        (&[7_f32, 7_f32, 7_f32, 7_f32], 4u16, ArrOutcome::Value(0_f32)),
        (&[1_f32, 2_f32, 3_f32, 4_f32, 5_f32], 5u16, ArrOutcome::Value(2.5_f32)),
        (&[1_f32], 1u16, ArrOutcome::Halt(65286)),
        (&[], 0u16, ArrOutcome::Halt(65286)),
        (&[1_f32, 2_f32, 3_f32, 4_f32, 5_f32, 6_f32, 7_f32, 8_f32, 9_f32, 10_f32, 11_f32, 12_f32, 13_f32, 14_f32, 15_f32, 16_f32], 17u16, ArrOutcome::Halt(65286)),
    ];
    for (i, (values, count, want)) in cases.iter().enumerate() {
        let mut cell = StateCell::bind(&cell_src("excel_var_s"), "ExcelVarS", None)
            .unwrap_or_else(|e| panic!("bind excel_var_s: {e}"));
        let bits: Vec<u64> = values.iter().map(|v| v.to_bits() as u64).collect();
        cell.set_array("values", &bits).unwrap();
        cell.set("count", *count as u64).unwrap();
        let report = cell
            .run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run excel_var_s case {i}: {e}"));
        match want {
            ArrOutcome::Value(w) => {
                assert_eq!(report.halt, Halt::Returned, "excel_var_s case {i}: {report:?}");
                let got = f32::from_bits(cell.get("var").unwrap() as u32);
                assert!(
                    f32_tol(got, *w),
                    "excel_var_s case {i}: got {got} want {w}",
                );
            }
            ArrOutcome::Halt(code) => {
                assert_eq!(
                    report.halt,
                    Halt::Escalate(*code),
                    "excel_var_s case {i} expected an escalation, got {report:?}",
                );
            }
        }
    }
}
