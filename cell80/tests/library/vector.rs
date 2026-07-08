//! Host-oracle tests for the vector pack (`cell80/cells/vector/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::{cell_src, run_cell};
use cell80::{StateCell, DEFAULT_CYCLES};

#[test]
fn vector_state_cells_match_defined_behaviour() {
    // dot2 (wave 3, pilot batch): a 4-field state cell purely for arg count (2 vectors),
    // not width — mirrors the manhattan/chebyshev shape.
    let mut cell = StateCell::bind(&cell_src("dot2"), "Dot2", None).unwrap();
    for (f, v) in [("ax", 3u64), ("ay", 4), ("bx", 2), ("by", 1)] {
        cell.set(f, v).unwrap();
    }
    assert_eq!(cell.run(DEFAULT_CYCLES).unwrap().result, 10); // 3*2 + 4*1
}

#[test]
fn first_wave_vector_cells_match_defined_behaviour() {
    let cases: &[(&str, &[u16], u16)] = &[("norm2_sq", &[3, 4], 25), ("norm2_sq", &[0, 0], 0)];

    let mut failures = Vec::new();
    for (id, args, exp) in cases {
        let got = run_cell(id, args);
        if got != *exp {
            failures.push(format!("{id}({args:?}) = {got}, expected {exp}"));
        }
    }
    assert!(
        failures.is_empty(),
        "cell mismatches:\n{}",
        failures.join("\n")
    );
}

#[test]
fn wave11_3d_vector_cells_match_defined_behaviour() {
    // Wave 11 (docs/math-server-map.md's vector category): cross_product and
    // vectors_parallel both track each signed component as a (magnitude, sign) pair
    // rather than forming a raw i16 arithmetic result, since the dialect has no
    // signed-32-bit width for an intermediate product. Cross-checked against an
    // independent Python reference implementation, including a 2,000-case random
    // sweep of cross_product against the true integer cross product, before
    // transcribing any test row here.
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
    fn cross(a: (i16, i16, i16), b: (i16, i16, i16)) -> (cell80::Report, StateCell) {
        step(
            "cross_product",
            "CrossProduct",
            &[
                ("ax", i16_bits(a.0)),
                ("ay", i16_bits(a.1)),
                ("az", i16_bits(a.2)),
                ("bx", i16_bits(b.0)),
                ("by", i16_bits(b.1)),
                ("bz", i16_bits(b.2)),
            ],
        )
    }

    // cross_product: unit basis vectors, i x j = k.
    let (_, cell) = cross((1, 0, 0), (0, 1, 0));
    assert_eq!(cell.get("rx_mag"), Some(0));
    assert_eq!(cell.get("ry_mag"), Some(0));
    assert_eq!(cell.get("rz_mag"), Some(1));
    assert_eq!(cell.get("rz_neg"), Some(0));

    // cross_product: known case (2,3,4) x (5,6,7) = (-3, 6, -3).
    let (_, cell) = cross((2, 3, 4), (5, 6, 7));
    assert_eq!((cell.get("rx_mag"), cell.get("rx_neg")), (Some(3), Some(1)));
    assert_eq!((cell.get("ry_mag"), cell.get("ry_neg")), (Some(6), Some(0)));
    assert_eq!((cell.get("rz_mag"), cell.get("rz_neg")), (Some(3), Some(1)));

    // vectors_parallel: same direction, anti-parallel (negative scalar), and a larger
    // positive scalar — all parallel. A non-parallel pair returns 0.
    fn parallel(a: (i16, i16, i16), b: (i16, i16, i16)) -> u16 {
        let (_, cell) = step(
            "vectors_parallel",
            "VectorsParallel",
            &[
                ("ax", i16_bits(a.0)),
                ("ay", i16_bits(a.1)),
                ("az", i16_bits(a.2)),
                ("bx", i16_bits(b.0)),
                ("by", i16_bits(b.1)),
                ("bz", i16_bits(b.2)),
            ],
        );
        cell.get("result").unwrap() as u16
    }
    assert_eq!(parallel((3, 4, 5), (6, 8, 10)), 1);
    assert_eq!(parallel((3, 4, 5), (-9, -12, -15)), 1);
    assert_eq!(parallel((3, 4, 5), (15, 20, 25)), 1);
    assert_eq!(parallel((1, 0, 0), (0, 1, 0)), 0);
    assert_eq!(parallel((0, 0, 0), (0, 0, 0)), 1); // the zero vector is trivially parallel
}
