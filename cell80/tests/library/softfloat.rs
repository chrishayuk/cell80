//! Host-oracle tests for the softfloat pack (`cell80/cells/softfloat/*.rs`) — the
//! first hand-authored f32 cells (the F-wave interim rule: float-touching cells are
//! hand-authored until repr tags reach the plan layer). Values ride `Ty::F32` state
//! fields as raw binary32 bits; every expected value is computed by host rustc f32 —
//! the same golden reference the kernel oracle banks use.

use crate::common::cell_src;
use cell80::{StateCell, DEFAULT_CYCLES};

fn run_f32_cell(id: &str, state: &str, fields: &[(&str, f32)]) -> StateCell {
    let src = cell_src(id);
    let mut cell = StateCell::bind(&src, state, None).unwrap_or_else(|e| panic!("{id}: {e}"));
    for (name, v) in fields {
        cell.set(name, v.to_bits() as u64)
            .unwrap_or_else(|e| panic!("{id}.{name}: {e}"));
    }
    let r = cell
        .run(DEFAULT_CYCLES)
        .unwrap_or_else(|e| panic!("{id}: {e}"));
    assert_eq!(r.result, 1, "{id} status");
    cell
}

fn get_f32(cell: &StateCell, field: &str) -> f32 {
    f32::from_bits(cell.get(field).expect(field) as u32)
}

#[test]
fn norm2_f32_matches_rustc() {
    let cases: [(f32, f32); 5] = [
        (3.0, 4.0), // the classic 5.0
        (0.0, 0.0),
        (-2.5, 6.75),
        (1.0e-20, 1.0e-20), // tiny magnitudes: the product underflow path
        (300.25, 0.125),
    ];
    for (x, y) in cases {
        let cell = run_f32_cell("norm2_f32", "Norm2F32", &[("x", x), ("y", y)]);
        let want = (x * x + y * y).sqrt();
        assert_eq!(
            get_f32(&cell, "len").to_bits(),
            want.to_bits(),
            "norm2({x}, {y}) diverged from rustc"
        );
    }
}

#[test]
fn lerp_f32_matches_rustc() {
    let cases: [(f32, f32, f32); 5] = [
        (0.0, 1.0, 0.5),
        (-3.5, 7.25, 0.75),
        (2.0, 2.0, 0.9), // degenerate span
        (1.0, 2.0, 0.0), // endpoints exact
        (1.0, 2.0, 1.0),
    ];
    for (a, b, t) in cases {
        let cell = run_f32_cell("lerp_f32", "LerpF32", &[("a", a), ("b", b), ("t", t)]);
        let want = a + t * (b - a);
        assert_eq!(
            get_f32(&cell, "out").to_bits(),
            want.to_bits(),
            "lerp({a}, {b}, {t}) diverged from rustc"
        );
    }
}
