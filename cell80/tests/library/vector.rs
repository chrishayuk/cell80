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
