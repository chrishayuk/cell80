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


#[test]
fn euclid_sq_i16_matches_hand_computed_expectations() {
    // Checks euclid_sq_i16 (cells/distance/euclid_sq_i16.rs): dx*dx + dy*dy (no sqrt)
    // between two signed (i16) grid points into a wide u32 `dist` field -- the 2D signed
    // sibling of euclid_sq (whose Pts fields are u16-only and so cannot represent a
    // negative coordinate). Each coordinate difference is computed via an excess-32768
    // shift feeding the shared iabs_diff kernel (the manhattan_i16/chebyshev_i16
    // technique), and the two squared terms are combined via add_checked_u32 (the
    // geom_distance_3d technique) so a maximally-separated pair escalates instead of
    // silently wrapping past u32::MAX.
    fn i16_bits(v: i16) -> u64 {
        (v as u16) as u64
    }
    fn dist(x1: i16, y1: i16, x2: i16, y2: i16) -> (u16, cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src("euclid_sq_i16"), "PtsSigned", None)
            .unwrap_or_else(|e| panic!("bind euclid_sq_i16: {e}"));
        for (f, v) in [
            ("x1", i16_bits(x1)),
            ("y1", i16_bits(y1)),
            ("x2", i16_bits(x2)),
            ("y2", i16_bits(y2)),
        ] {
            cell.set(f, v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        let result = report.result;
        (result, report, cell)
    }

    // 3-4-5 right triangle: dx=3, dy=4 -> dist=9+16=25.
    let (r, report, cell) = dist(0, 0, 3, 4);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(r, 25);
    assert_eq!(cell.get("dist"), Some(25));

    // Straddling the origin: dx=|5-(-5)|=10, dy=|3-(-3)|=6 -> dist=100+36=136.
    let (r, report, cell) = dist(-5, -3, 5, 3);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(r, 136);
    assert_eq!(cell.get("dist"), Some(136));

    // Mixed-sign, small: dx=|-3-(-10)|=7, dy=|15-20|=5 -> dist=49+25=74.
    let (r, report, _) = dist(-10, 20, -3, 15);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(r, 74);

    // Scaled 3-4-5 triangle: dx=300, dy=400 -> dist=90000+160000=250000, which exceeds
    // u16::MAX so the scalar return saturates to 65535 but the wide field stays exact.
    let (r, report, cell) = dist(0, 0, 300, 400);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(r, 65535);
    assert_eq!(cell.get("dist"), Some(250_000));

    // Full i16 span on x only: dx=|32767-(-32768)|=65535, dy=0 -> dist=65535*65535=
    // 4294836225, just under u32::MAX (4294967295), so add_checked_u32 must NOT
    // escalate here -- only the scalar return saturates.
    let (r, report, cell) = dist(-32768, 0, 32767, 0);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(r, 65535);
    assert_eq!(cell.get("dist"), Some(4_294_836_225));

    // Full i16 span on both axes: dx=dy=65535 -> dist=4294836225+4294836225=8589672450,
    // which exceeds u32::MAX, so add_checked_u32 must escalate (halt 0xFF05,
    // needs_wider_math) rather than silently wrap.
    let (_, report, _) = dist(-32768, -32768, 32767, 32767);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn euclid_dist_i16_matches_hand_computed_expectations() {
    // Checks euclid_dist_i16 (cells/distance/euclid_dist_i16.rs): true (non-squared)
    // Euclidean distance isqrt(dx*dx + dy*dy) between two signed (i16) grid points --
    // the origin-centered sibling of euclid_dist (whose Pts fields are u16-only and so
    // cannot represent a negative coordinate at all). Each coordinate difference is
    // computed via an excess-32768 shift (the chebyshev_i16/manhattan_i16 technique)
    // feeding the shared iabs_diff kernel, then combined via add_checked_u32 (escalates
    // instead of wrapping) and reduced with the branch-free bitwise integer-sqrt loop
    // euclid_dist/isqrt_u32/cosine_score_approx also run.
    fn i16_bits(v: i16) -> u64 {
        (v as u16) as u64
    }
    fn dist(x1: i16, y1: i16, x2: i16, y2: i16) -> (u16, cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src("euclid_dist_i16"), "PtsSigned", None)
            .unwrap_or_else(|e| panic!("bind euclid_dist_i16: {e}"));
        for (f, v) in [
            ("x1", i16_bits(x1)),
            ("y1", i16_bits(y1)),
            ("x2", i16_bits(x2)),
            ("y2", i16_bits(y2)),
        ] {
            cell.set(f, v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        let result = report.result;
        (result, report, cell)
    }

    // 3-4-5 right triangle, all positive: dx=3, dy=4, sum=25, isqrt=5 exactly.
    let (r, report, cell) = dist(0, 0, 3, 4);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(r, 5);
    assert_eq!(cell.get("dist"), Some(5));

    // Mixed-sign, non-perfect-square: dx=|-3-4|=7, dy=|-4-4|=8, sum=113,
    // floor(sqrt(113))=10 -- the case unsigned euclid_dist cannot express at all.
    let (r, report, _) = dist(-3, -4, 4, 4);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(r, 10);

    // Full i16 span on x only: dx=|32767-(-32768)|=65535, dy=0, sum=65535^2 exactly,
    // isqrt=65535 exactly.
    let (r, report, _) = dist(-32768, 0, 32767, 0);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(r, 65535);

    // Coincident points at a positive coordinate: sum=0, isqrt(0)=0.
    let (r, report, _) = dist(5, 5, 5, 5);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(r, 0);

    // Small mixed-sign case: dx=|-10-(-3)|=7, dy=|20-15|=5, sum=74, floor(sqrt(74))=8.
    let (r, report, _) = dist(-10, 20, -3, 15);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(r, 8);

    // Full i16 span on both axes: dx=dy=65535, sum=8_589_672_450 > u32::MAX, so
    // add_checked_u32 must escalate (halt 0xFF05, needs_wider_math) not wrap, matching
    // euclid_dist's own escalation behaviour at the same extreme.
    let (_, report, _) = dist(-32768, -32768, 32767, 32767);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn manhattan_path3_matches_hand_computed_expectations() {
    // Checks manhattan_path3 (cells/distance/manhattan_path3.rs): total Manhattan
    // path distance across three consecutive grid points p1->p2->p3, i.e.
    // manhattan(p1,p2) + manhattan(p2,p3), into a wide u32 `dist` field -- the
    // multi-hop sibling of manhattan/manhattan_wide, which are point-pair only.
    // The scalar `run()` return saturates at u16::MAX (65535) per the pack's
    // manhattan_wide/manhattan_i16/euclid_sq precedent, but the named `dist`
    // field always carries the exact wide sum.
    fn dist(x1: u16, y1: u16, x2: u16, y2: u16, x3: u16, y3: u16) -> (u16, u64) {
        let mut cell = StateCell::bind(&cell_src("manhattan_path3"), "Path3", None)
            .unwrap_or_else(|e| panic!("bind manhattan_path3: {e}"));
        for (f, v) in [
            ("x1", x1), ("y1", y1),
            ("x2", x2), ("y2", y2),
            ("x3", x3), ("y3", y3),
        ] {
            cell.set(f, v as u64).unwrap();
        }
        let scalar = cell.run(DEFAULT_CYCLES).unwrap().result;
        let field = cell.get("dist").unwrap();
        (scalar, field)
    }

    // p2 == p3, second leg is zero. seg1 dx=3,dy=4 -> 7. seg2 -> 0. total 7.
    assert_eq!(dist(0, 0, 3, 4, 3, 4), (7, 7));

    // seg1 dx=5,dy=0 -> 5. seg2 dx=0,dy=5 -> 5. total 10.
    assert_eq!(dist(0, 0, 5, 0, 5, 5), (10, 10));

    // Out-and-back route. seg1 dx=10,dy=10 -> 20. seg2 dx=10,dy=10 -> 20. total 40.
    assert_eq!(dist(10, 10, 0, 0, 10, 10), (40, 40));

    // Mixed-direction abs check. seg1 dx=|50-20|=30,dy=|50-80|=30 -> 60.
    // seg2 dx=|20-70|=50,dy=|80-10|=70 -> 120. total 180.
    assert_eq!(dist(50, 50, 20, 80, 70, 10), (180, 180));

    // All three points coincide -> 0.
    assert_eq!(dist(100, 100, 100, 100, 100, 100), (0, 0));

    // Wide-field saturation check. seg1 dx=65535,dy=0 -> 65535.
    // seg2 dx=65535,dy=65535 -> 131070. total 196605, which exceeds u16::MAX
    // (65535): the scalar return saturates but the wide `dist` field carries
    // the exact sum.
    assert_eq!(dist(0, 0, 65535, 0, 0, 65535), (65535, 196605));
}

#[test]
fn euclid_dist_path3_matches_hand_computed_expectations() {
    // Checks euclid_dist_path3 (cells/distance/euclid_dist_path3.rs): total true
    // (non-squared) Euclidean path distance across three consecutive points, i.e.
    // euclid_dist(p1,p2) + euclid_dist(p2,p3) -- the rooted-per-segment sibling of
    // manhattan_path3. Each leg runs euclid_dist's own excess-shift/add_checked_u32/
    // inline-isqrt chain a second time (isqrt_u32 is a state cell, so it can't be
    // called as a subroutine across a call boundary -- the chain must be duplicated
    // by hand, exactly as it is inline in euclid_dist itself).
    fn path3(x1: u16, y1: u16, x2: u16, y2: u16, x3: u16, y3: u16) -> (u16, cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src("euclid_dist_path3"), "Path3", None)
            .unwrap_or_else(|e| panic!("bind euclid_dist_path3: {e}"));
        for (f, v) in [
            ("x1", x1), ("y1", y1), ("x2", x2), ("y2", y2), ("x3", x3), ("y3", y3),
        ] {
            cell.set(f, v as u64).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        let result = report.result;
        (result, report, cell)
    }

    // Both legs are 3-4-5 triangles: seg1=5, seg2=5, total=10.
    let (r, report, cell) = path3(0, 0, 3, 4, 6, 8);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(r, 10);
    assert_eq!(cell.get("dist"), Some(10));

    // Second leg is zero-length (p3 == p2): seg1=5, seg2=0, total=5.
    assert_eq!(path3(0, 0, 3, 4, 3, 4).0, 5);

    // All three points coincident -> both legs zero -> total=0.
    assert_eq!(path3(5, 5, 5, 5, 5, 5).0, 0);

    // Non-perfect-square legs: each leg dx=1,dy=1,sum=2, floor(sqrt(2))=1 -> total=2.
    assert_eq!(path3(0, 0, 1, 1, 2, 2).0, 2);

    // Leg1 is a 3-4-5 triangle scaled by 100 (dx=300,dy=400 -> 500 exactly); leg2 is a
    // 7-24-25 triangle (dx=7,dy=24 -> 25 exactly). total = 500 + 25 = 525.
    assert_eq!(path3(0, 0, 300, 400, 307, 424).0, 525);

    // Saturation: (0,0)->(65535,0)->(0,0). Leg1: dx=65535,dy=0, sum=65535^2=4294836225
    // (<= u32::MAX, no escalation), isqrt=65535 exactly. Leg2 is the same computation
    // mirrored back, isqrt=65535. total=131070, which exceeds u16::MAX: the scalar
    // return saturates to 65535 but the wide `dist` field carries the exact sum, per
    // the pack's manhattan_wide/euclid_sq saturation precedent.
    let (r, report, cell) = path3(0, 0, 65535, 0, 0, 0);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(r, 65535);
    assert_eq!(cell.get("dist"), Some(131070));

    // Escalation: (0,0)->(65535,65535)->(0,0). Leg1 alone: dx=dy=65535, sum =
    // 65535^2 + 65535^2 = 8589672450 > u32::MAX, so add_checked_u32 must escalate
    // (halt 0xFF05, needs_wider_math) before segment 2 is even reached -- the same
    // escalation condition euclid_dist itself hits on this exact corner.
    let (_, report, _) = path3(0, 0, 65535, 65535, 0, 0);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn chebyshev_path3_matches_hand_computed_expectations() {
    // Checks chebyshev_path3 (cells/distance/chebyshev_path3.rs): total Chebyshev
    // ('king-move') path cost across three consecutive grid points, chebyshev(p1,p2) +
    // chebyshev(p2,p3), into a wide u32 dist field -- the arity-3 sibling chebyshev/
    // chebyshev_i16 lack, since a single max never needs widening but summing two maxes
    // across a path can (up to 131070 for two maximally-separated hops).
    fn dist(x1: u16, y1: u16, x2: u16, y2: u16, x3: u16, y3: u16) -> (u16, u64) {
        let mut cell = StateCell::bind(&cell_src("chebyshev_path3"), "Path3", None)
            .unwrap_or_else(|e| panic!("bind chebyshev_path3: {e}"));
        for (f, v) in [
            ("x1", x1), ("y1", y1),
            ("x2", x2), ("y2", y2),
            ("x3", x3), ("y3", y3),
        ] {
            cell.set(f, v as u64).unwrap();
        }
        let scalar = cell.run(DEFAULT_CYCLES).unwrap().result;
        let field = cell.get("dist").unwrap();
        (scalar, field)
    }

    // c1 = max(|3-10|,|4-9|) = max(7,5) = 7; c2 = max(|10-15|,|9-1|) = max(5,8) = 8; sum = 15
    assert_eq!(dist(3, 4, 10, 9, 15, 1), (15, 15));

    // p3 == p2: c1 = max(5,2) = 5; c2 = max(0,0) = 0; sum = 5
    assert_eq!(dist(0, 0, 5, 2, 5, 2), (5, 5));

    // c1 = max(50,50) = 50; c2 = max(150,150) = 150; sum = 200 (no saturation)
    assert_eq!(dist(100, 200, 150, 250, 300, 100), (200, 200));

    // all three points identical: c1 = 0, c2 = 0, sum = 0
    assert_eq!(dist(7, 7, 7, 7, 7, 7), (0, 0));

    // maximal separation each hop: c1 = max(65535,0) = 65535; c2 = max(65535,65535) = 65535;
    // sum = 131070, which exceeds u16::MAX (65535) -- scalar saturates, wide field stays exact.
    assert_eq!(dist(0, 0, 65535, 0, 0, 65535), (65535, 131070));
}
