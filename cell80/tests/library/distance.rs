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


#[test]
fn euclid_dist_matches_hand_computed_expectations() {
    // Checks euclid_dist (cells/distance/euclid_dist.rs): true (non-squared) Euclidean
    // distance isqrt(dx*dx + dy*dy) -- the sqrt-closed sibling of euclid_sq. dx*dx+dy*dy
    // is combined via add_checked_u32 (escalates instead of wrapping) then reduced with
    // the branch-free bitwise integer-sqrt loop isqrt_u32/cosine_score_approx also run.
    fn dist(x1: u16, y1: u16, x2: u16, y2: u16) -> (u16, cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src("euclid_dist"), "Pts", None)
            .unwrap_or_else(|e| panic!("bind euclid_dist: {e}"));
        for (f, v) in [("x1", x1), ("y1", y1), ("x2", x2), ("y2", y2)] {
            cell.set(f, v as u64).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        let result = report.result;
        (result, report, cell)
    }

    // 3-4-5 right triangle: dx=3, dy=4, sum=9+16=25, isqrt(25)=5 exactly.
    let (r, report, cell) = dist(0, 0, 3, 4);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(r, 5);
    assert_eq!(cell.get("dist"), Some(5));

    // Same triangle scaled by 100: dx=300, dy=400, sum=250000, isqrt=500 exactly.
    let (r, report, _) = dist(0, 0, 300, 400);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(r, 500);

    // Coincident points: sum=0, isqrt(0)=0.
    let (r, report, _) = dist(5, 5, 5, 5);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(r, 0);

    // Non-perfect-square case: dx=1, dy=1, sum=2, floor(sqrt(2))=1.
    let (r, report, _) = dist(0, 0, 1, 1);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(r, 1);

    // 7-24-25 right triangle: sum=49+576=625, isqrt(625)=25 exactly.
    let (r, report, _) = dist(0, 0, 7, 24);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(r, 25);

    // Extreme separation on both axes: dx=dy=65535, sum=8_589_672_450 > u32::MAX,
    // so add_checked_u32 must escalate (halt 0xFF05, needs_wider_math) not wrap.
    let (_, report, _) = dist(0, 0, 65535, 65535);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn manhattan_i16_matches_hand_computed_expectations() {
    // Checks manhattan_i16 (cells/distance/manhattan_i16.rs): dx+dy between two
    // signed (i16) grid points into a wide u32 `dist` field, the origin-centered
    // sibling of manhattan_wide (whose Pts fields are u16-only and so cannot
    // represent a negative coordinate at all). Each coordinate difference is
    // computed via an excess-32768 shift (mapping i16's range losslessly onto
    // u16) feeding the shared iabs_diff kernel -- the same technique
    // geom_distance_3d/orientation2d/slope_fraction already use. The scalar
    // `run()` return still saturates at u16::MAX (65535) per the pack's
    // euclid_sq/manhattan_wide precedent, but the named `dist` field always
    // carries the exact wide sum.
    fn dist(x1: i16, y1: i16, x2: i16, y2: i16) -> (u16, u64) {
        let mut cell = StateCell::bind(&cell_src("manhattan_i16"), "PtsSigned", None)
            .unwrap_or_else(|e| panic!("bind manhattan_i16: {e}"));
        for (f, v) in [("x1", x1), ("y1", y1), ("x2", x2), ("y2", y2)] {
            cell.set(f, v as u16 as u64).unwrap();
        }
        let scalar = cell.run(DEFAULT_CYCLES).unwrap().result;
        let field = cell.get("dist").unwrap();
        (scalar, field)
    }

    // Identical points -> dx=0, dy=0, dist=0.
    assert_eq!(dist(0, 0, 0, 0), (0, 0));

    // Straddling the origin -> dx=|5-(-5)|=10, dy=|3-(-3)|=6, dist=16.
    assert_eq!(dist(-5, -3, 5, 3), (16, 16));

    // Mixed-sign, small -> dx=|-3-(-10)|=7, dy=|15-20|=5, dist=12.
    assert_eq!(dist(-10, 20, -3, 15), (12, 12));

    // Full i16 span on x only -> dx=|32767-(-32768)|=65535, dy=0, dist=65535.
    // 65535 fits exactly in u16, so the scalar return is NOT saturated.
    assert_eq!(dist(-32768, 0, 32767, 0), (65535, 65535));

    // Full i16 span on both axes -> dx=65535, dy=65535, dist=131070, which
    // exceeds u16::MAX. The scalar return saturates to 65535 but the wide
    // `dist` field carries the exact sum.
    assert_eq!(dist(-32768, -32768, 32767, 32767), (65535, 131070));
}

#[test]
fn chebyshev_i16_matches_hand_computed_expectations() {
    // Checks chebyshev_i16 (cells/distance/chebyshev_i16.rs): max(|dx|, |dy|) for signed
    // (i16) grid coordinates, computed via the manhattan_i16 excess-32768-shift technique
    // feeding the shared iabs_diff/imax kernels. i16 args are passed as their raw 16-bit
    // bit pattern (negative values wrap into the upper half of u16, e.g. -5 -> 65531).
    fn i16_bits(v: i16) -> u64 {
        (v as u16) as u64
    }
    fn dist(x1: i16, y1: i16, x2: i16, y2: i16) -> u16 {
        let mut cell = StateCell::bind(&cell_src("chebyshev_i16"), "PtsSigned", None)
            .unwrap_or_else(|e| panic!("bind chebyshev_i16: {e}"));
        for (f, v) in [
            ("x1", i16_bits(x1)),
            ("y1", i16_bits(y1)),
            ("x2", i16_bits(x2)),
            ("y2", i16_bits(y2)),
        ] {
            cell.set(f, v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // dx=7, dy=5 -> max=7 (both positive coordinates, same as unsigned chebyshev).
    assert_eq!(dist(3, 4, 10, 9), 7);

    // dx=5, dy=2 -> max=5.
    assert_eq!(dist(0, 0, 5, 2), 5);

    // Mixed signs crossing zero: dx=|-3-4|=7, dy=|-4-4|=8 -> max=8. This is the case
    // the unsigned chebyshev cell cannot express at all (no negative coordinates).
    assert_eq!(dist(-3, -4, 4, 4), 8);

    // Extreme corners: i16::MIN to i16::MAX on both axes -> dx=dy=65535 -> max=65535,
    // exactly u16::MAX, confirming the excess-32768 shift + iabs_diff never overflows.
    assert_eq!(dist(-32768, -32768, 32767, 32767), 65535);

    // Same point at positive coordinates -> 0.
    assert_eq!(dist(5, 5, 5, 5), 0);

    // Same point at negative coordinates -> 0 (exercises the shift on both sides equally).
    assert_eq!(dist(-10, -10, -10, -10), 0);
}
