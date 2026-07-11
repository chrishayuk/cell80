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

#[test]
fn manhattan_wide_matches_hand_computed_expectations() {
    // Checks manhattan_wide (cells/distance/manhattan_wide.rs): dx+dy into a wide u32
    // `dist` field. The scalar `run()` return still saturates at u16::MAX (65535) per
    // the pack's euclid_sq precedent, but the named `dist` field always carries the
    // exact wide sum, so callers who read the field never lose precision even when
    // dx+dy exceeds 65535 (up to 131070 for two maximally-separated u16 coordinates).
    fn dist(x1: u16, y1: u16, x2: u16, y2: u16) -> (u16, u64) {
        let mut cell = StateCell::bind(&cell_src("manhattan_wide"), "Pts", None)
            .unwrap_or_else(|e| panic!("bind manhattan_wide: {e}"));
        for (f, v) in [("x1", x1), ("y1", y1), ("x2", x2), ("y2", y2)] {
            cell.set(f, v as u64).unwrap();
        }
        let scalar = cell.run(DEFAULT_CYCLES).unwrap().result;
        let field = cell.get("dist").unwrap();
        (scalar, field)
    }

    // dx=7, dy=5 -> 12, well within u16, no saturation.
    assert_eq!(dist(3, 4, 10, 9), (12, 12));

    // dx=5, dy=2 -> 7.
    assert_eq!(dist(0, 0, 5, 2), (7, 7));

    // dx=50, dy=50 -> 100.
    assert_eq!(dist(100, 200, 150, 250), (100, 100));

    // dx=40000, dy=40000 -> 80000, which is > u16::MAX (65535): the scalar return
    // saturates to 65535 but the wide `dist` field carries the exact sum. The old
    // narrow `manhattan` cell would have wrapped this to 80000 - 65536 = 14464.
    assert_eq!(dist(0, 0, 40000, 40000), (65535, 80000));

    // Maximum possible separation: dx=65535, dy=65535 -> 131070 (the documented
    // dx+dy ceiling), still exact in the u32 field, still saturated in the scalar.
    assert_eq!(dist(0, 0, 65535, 65535), (65535, 131070));
}
