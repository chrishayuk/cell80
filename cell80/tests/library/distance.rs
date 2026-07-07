//! Host-oracle tests for the distance pack (`cell80/cells/distance/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::cell_src;
use cell80::{StateCell, DEFAULT_CYCLES};

#[test]
fn distance_state_cells_match_defined_behaviour() {
    fn dist(id: &str, x1: u16, y1: u16, x2: u16, y2: u16) -> u16 {
        let mut cell = StateCell::bind(&cell_src(id), "Pts", None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in [("x1", x1), ("y1", y1), ("x2", x2), ("y2", y2)] {
            cell.set(f, v as u64).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // The 4-point distance cells exceed the 3-arg convention, so they're state cells (a
    // `Pts` struct, like `manhattan`): set the four coordinates by name, run, read the result.
    // chebyshev = max(|dx|, |dy|); euclid_sq = dx² + dy².
    assert_eq!(dist("chebyshev", 3, 4, 10, 9), 7); // max(7, 5)
    assert_eq!(dist("chebyshev", 0, 0, 5, 2), 5);
    assert_eq!(dist("euclid_sq", 0, 0, 3, 4), 25); // 9 + 16
    assert_eq!(dist("euclid_sq", 1, 1, 4, 5), 25); // 9 + 16

    // euclid_sq's `dist` is a wide u32 field: past the u16 ceiling the scalar result
    // saturates but the named field carries the exact value.
    let mut cell = StateCell::bind(&cell_src("euclid_sq"), "Pts", None).unwrap();
    for (f, v) in [("x1", 0u64), ("y1", 0), ("x2", 300), ("y2", 400)] {
        cell.set(f, v).unwrap();
    }
    assert_eq!(cell.run(DEFAULT_CYCLES).unwrap().result, 65535); // saturated scalar
    assert_eq!(cell.get("dist"), Some(250_000)); // 300² + 400², exact and wide
}
