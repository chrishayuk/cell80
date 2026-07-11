//! Host-oracle tests for the spatial-grid pack (`cell80/cells/spatial-grid/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::{cell_src, run_cell};
use cell80::{StateCell, DEFAULT_CYCLES};

#[test]
fn spatial_grid_state_cells_match_defined_behaviour() {
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> u16 {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // point_in_rect / aabb_intersect (wave 3): both half-open — edge-touching doesn't count.

    assert_eq!(
        step(
            "point_in_rect",
            "PointInRect",
            &[
                ("px", 5),
                ("py", 5),
                ("rx", 0),
                ("ry", 0),
                ("rw", 10),
                ("rh", 10)
            ],
        ),
        1
    );
    assert_eq!(
        step(
            "point_in_rect",
            "PointInRect",
            &[
                ("px", 15),
                ("py", 5),
                ("rx", 0),
                ("ry", 0),
                ("rw", 10),
                ("rh", 10)
            ],
        ),
        0
    );
    assert_eq!(
        step(
            "point_in_rect",
            "PointInRect",
            &[
                ("px", 10),
                ("py", 5),
                ("rx", 0),
                ("ry", 0),
                ("rw", 10),
                ("rh", 10)
            ],
        ),
        0 // on the right edge — half-open, doesn't count
    );

    assert_eq!(
        step(
            "aabb_intersect",
            "AabbIntersect",
            &[
                ("x1", 0),
                ("y1", 0),
                ("w1", 10),
                ("h1", 10),
                ("x2", 5),
                ("y2", 5),
                ("w2", 10),
                ("h2", 10),
            ],
        ),
        1
    );
    assert_eq!(
        step(
            "aabb_intersect",
            "AabbIntersect",
            &[
                ("x1", 0),
                ("y1", 0),
                ("w1", 10),
                ("h1", 10),
                ("x2", 20),
                ("y2", 20),
                ("w2", 5),
                ("h2", 5),
            ],
        ),
        0
    );
    assert_eq!(
        step(
            "aabb_intersect",
            "AabbIntersect",
            &[
                ("x1", 0),
                ("y1", 0),
                ("w1", 10),
                ("h1", 10),
                ("x2", 10),
                ("y2", 0),
                ("w2", 5),
                ("h2", 5),
            ],
        ),
        0 // edge-touching, not overlapping
    );
}

#[test]
fn library_growth_backlog_spatial_grid_slice() {
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        let result = report.result;
        (result, report, cell)
    }

    // morton_encode / morton_decode: round-trip and known corner values.
    let (_, _, cell) = step("morton_encode", "MortonEncode", &[("x", 0), ("y", 0)]);
    assert_eq!(cell.get("code"), Some(0));
    let (_, _, cell) = step(
        "morton_encode",
        "MortonEncode",
        &[("x", 65535), ("y", 65535)],
    );
    assert_eq!(cell.get("code"), Some(4_294_967_295));
    let (_, _, cell) = step("morton_encode", "MortonEncode", &[("x", 1), ("y", 0)]);
    assert_eq!(cell.get("code"), Some(1));
    let (_, _, cell) = step("morton_encode", "MortonEncode", &[("x", 0), ("y", 1)]);
    assert_eq!(cell.get("code"), Some(2));

    let (_, _, cell) = step("morton_decode", "MortonDecode", &[("code", 0)]);
    assert_eq!((cell.get("x"), cell.get("y")), (Some(0), Some(0)));
    let (_, _, cell) = step("morton_decode", "MortonDecode", &[("code", 4_294_967_295)]);
    assert_eq!((cell.get("x"), cell.get("y")), (Some(65535), Some(65535)));
    let (_, _, cell) = step("morton_decode", "MortonDecode", &[("code", 1)]);
    assert_eq!((cell.get("x"), cell.get("y")), (Some(1), Some(0)));
    let (_, _, cell) = step("morton_decode", "MortonDecode", &[("code", 2)]);
    assert_eq!((cell.get("x"), cell.get("y")), (Some(0), Some(1)));

    // bresenham_step: the (0,0)-(4,2) line (dx=4, dy=2), hand-verified against a full
    // reference line generator — err carried as a (mag, neg) pair since state fields can't
    // be i16.
    let (_, _, cell) = step(
        "bresenham_step",
        "BresenhamStep",
        &[("dx", 4), ("dy", 2), ("err_mag", 2), ("err_neg", 0)],
    );
    assert_eq!(cell.get("step_x"), Some(1));
    assert_eq!(cell.get("step_y"), Some(0));
    assert_eq!(
        (cell.get("err_mag"), cell.get("err_neg")),
        (Some(0), Some(0))
    );
    let (_, _, cell) = step(
        "bresenham_step",
        "BresenhamStep",
        &[("dx", 4), ("dy", 2), ("err_mag", 0), ("err_neg", 0)],
    );
    assert_eq!(cell.get("step_x"), Some(1));
    assert_eq!(cell.get("step_y"), Some(1));
    assert_eq!(
        (cell.get("err_mag"), cell.get("err_neg")),
        (Some(2), Some(0))
    );
}

#[test]
fn first_wave_spatial_grid_cells_match_defined_behaviour() {
    let cases: &[(&str, &[u16], u16)] = &[
        ("grid_index", &[3, 2, 10], 23),
        ("grid_index", &[0, 0, 10], 0),
    ];

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
fn grid_coords_is_the_inverse_of_grid_index_and_guards_width_zero() {
    // Local helper: bind GridCoords, set index/width, run, return the cell for get() reads.
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> StateCell {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap();
        cell
    }

    // index=0, width=5 -> x=0%5=0, y=0/5=0 (origin).
    let cell = step("grid_coords", "GridCoords", &[("index", 0), ("width", 5)]);
    assert_eq!((cell.get("x"), cell.get("y")), (Some(0), Some(0)));

    // index=7, width=5 -> x=7%5=2, y=7/5=1: mirrors grid_index(2,1,5) == 7, the round-trip.
    let cell = step("grid_coords", "GridCoords", &[("index", 7), ("width", 5)]);
    assert_eq!((cell.get("x"), cell.get("y")), (Some(2), Some(1)));

    // index=23, width=5 -> x=23%5=3, y=23/5=4.
    let cell = step("grid_coords", "GridCoords", &[("index", 23), ("width", 5)]);
    assert_eq!((cell.get("x"), cell.get("y")), (Some(3), Some(4)));

    // width=0 guard: must return (0, 0) rather than halting on DivByZero.
    let cell = step("grid_coords", "GridCoords", &[("index", 100), ("width", 0)]);
    assert_eq!((cell.get("x"), cell.get("y")), (Some(0), Some(0)));

    // index=9999, width=10 -> x=9999%10=9, y=9999/10=999 (larger grid sanity check).
    let cell = step(
        "grid_coords",
        "GridCoords",
        &[("index", 9999), ("width", 10)],
    );
    assert_eq!((cell.get("x"), cell.get("y")), (Some(9), Some(999)));
}

// point_in_circle: exact squared-distance-vs-radius circle membership predicate, no sqrt.
// Checks center, exact boundary (two ways: an axis-aligned radius and a 3-4-5 Pythagorean
// triple), just-outside on both, and a degenerate zero-radius circle at/off center.
#[test]
fn point_in_circle_matches_defined_behaviour() {
    fn step(px: u16, py: u16, cx: u16, cy: u16, r: u16) -> u16 {
        let mut cell = StateCell::bind(&cell_src("point_in_circle"), "PointInCircle", None)
            .unwrap_or_else(|e| panic!("bind point_in_circle: {e}"));
        cell.set("px", px as u64).unwrap();
        cell.set("py", py as u64).unwrap();
        cell.set("cx", cx as u64).unwrap();
        cell.set("cy", cy as u64).unwrap();
        cell.set("r", r as u64).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    assert_eq!(step(5, 5, 5, 5, 3), 1); // point at center: dist_sq=0 <= r_sq=9
    assert_eq!(step(8, 5, 5, 5, 3), 1); // exactly on boundary: dx=3, dist_sq=9 <= r_sq=9
    assert_eq!(step(9, 5, 5, 5, 3), 0); // just outside: dx=4, dist_sq=16 > r_sq=9
    assert_eq!(step(3, 4, 0, 0, 5), 1); // classic 3-4-5 triangle, exactly on boundary: 9+16=25 <= 25
    assert_eq!(step(3, 5, 0, 0, 5), 0); // dx=3,dy=5: dist_sq=9+25=34 > r_sq=25
    assert_eq!(step(2, 2, 2, 2, 0), 1); // zero-radius circle, point at center: 0 <= 0
    assert_eq!(step(3, 2, 2, 2, 0), 0); // zero-radius circle, point off center: 1 > 0
}

// aabb_contains (spatial-grid): box-in-box containment — a genuinely different predicate
// from aabb_intersect's overlap test. Checks all four edges of the inner box lie within
// (or exactly on) the outer box's edges, via a half-open-style <=/>= inequality chain.
#[test]
fn aabb_contains_matches_defined_behaviour() {
    fn step(fields: &[(&str, u64)]) -> u16 {
        let mut cell = StateCell::bind(&cell_src("aabb_contains"), "AabbContains", None)
            .unwrap_or_else(|e| panic!("bind aabb_contains: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // Strictly inside: outer (0,0,10,10), inner (2,2,4,4) -> right=6<=10, bottom=6<=10.
    assert_eq!(
        step(&[
            ("x1", 0),
            ("y1", 0),
            ("w1", 10),
            ("h1", 10),
            ("x2", 2),
            ("y2", 2),
            ("w2", 4),
            ("h2", 4),
        ]),
        1
    );

    // Equal boxes are contained (boundary-inclusive).
    assert_eq!(
        step(&[
            ("x1", 0),
            ("y1", 0),
            ("w1", 10),
            ("h1", 10),
            ("x2", 0),
            ("y2", 0),
            ("w2", 10),
            ("h2", 10),
        ]),
        1
    );

    // Overlaps (aabb_intersect would say 1) but pokes past the right/bottom edge ->
    // NOT contained. This is the case that distinguishes containment from overlap.
    assert_eq!(
        step(&[
            ("x1", 0),
            ("y1", 0),
            ("w1", 10),
            ("h1", 10),
            ("x2", 5),
            ("y2", 5),
            ("w2", 10),
            ("h2", 10),
        ]),
        0
    );

    // Entirely disjoint -> not contained.
    assert_eq!(
        step(&[
            ("x1", 0),
            ("y1", 0),
            ("w1", 10),
            ("h1", 10),
            ("x2", 20),
            ("y2", 20),
            ("w2", 5),
            ("h2", 5),
        ]),
        0
    );

    // Inner starts before outer's origin on both axes -> not contained.
    assert_eq!(
        step(&[
            ("x1", 5),
            ("y1", 5),
            ("w1", 10),
            ("h1", 10),
            ("x2", 0),
            ("y2", 0),
            ("w2", 3),
            ("h2", 3),
        ]),
        0
    );
}

// aabb_intersection: the actual overlapping rectangle (ix,iy,iw,ih) of two AABBs, plus a
// valid flag (0 when they don't truly overlap) -- contrasts with aabb_intersect's plain
// 0/1 verdict by returning the intersection region itself. Cases hand-computed via
// left=max(x1,x2), top=max(y1,y2), right=min(x1+w1,x2+w2), bottom=min(y1+h1,y2+h2);
// valid=(right>left)&&(bottom>top); when invalid, ix/iy/iw/ih are all 0.
#[test]
fn aabb_intersection_matches_hand_computed_expectations() {
    fn step(fields: &[(&str, u64)]) -> (u64, u64, u64, u64, u64) {
        let mut cell = StateCell::bind(&cell_src("aabb_intersection"), "AabbIntersection", None)
            .unwrap_or_else(|e| panic!("bind aabb_intersection: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap();
        (
            cell.get("ix").unwrap(),
            cell.get("iy").unwrap(),
            cell.get("iw").unwrap(),
            cell.get("ih").unwrap(),
            cell.get("valid").unwrap(),
        )
    }

    // Overlapping boxes (0,0,10,10) [0,10)x[0,10) and (5,5,10,10) [5,15)x[5,15)
    // -> left=max(0,5)=5, top=max(0,5)=5, right=min(10,15)=10, bottom=min(10,15)=10
    // -> intersection (5,5,5,5), valid=1.
    assert_eq!(
        step(&[
            ("x1", 0),
            ("y1", 0),
            ("w1", 10),
            ("h1", 10),
            ("x2", 5),
            ("y2", 5),
            ("w2", 10),
            ("h2", 10),
        ]),
        (5, 5, 5, 5, 1)
    );

    // Disjoint boxes (0,0,5,5) and (10,10,5,5) -> right(5) <= left(10) -> not valid.
    assert_eq!(
        step(&[
            ("x1", 0),
            ("y1", 0),
            ("w1", 5),
            ("h1", 5),
            ("x2", 10),
            ("y2", 10),
            ("w2", 5),
            ("h2", 5),
        ]),
        (0, 0, 0, 0, 0)
    );

    // Edge-touching boxes (0,0,5,5) and (5,0,5,5) -> right(5) > left(5) is false -> not valid
    // (matches aabb_intersect's edge-touching-doesn't-count convention).
    assert_eq!(
        step(&[
            ("x1", 0),
            ("y1", 0),
            ("w1", 5),
            ("h1", 5),
            ("x2", 5),
            ("y2", 0),
            ("w2", 5),
            ("h2", 5),
        ]),
        (0, 0, 0, 0, 0)
    );

    // Box2 fully inside box1 (0,0,20,20) contains (5,5,5,5) -> intersection equals box2.
    assert_eq!(
        step(&[
            ("x1", 0),
            ("y1", 0),
            ("w1", 20),
            ("h1", 20),
            ("x2", 5),
            ("y2", 5),
            ("w2", 5),
            ("h2", 5),
        ]),
        (5, 5, 5, 5, 1)
    );

    // Asymmetric partial overlap (2,3,8,4) [2,10)x[3,7) and (6,1,10,10) [6,16)x[1,11)
    // -> left=max(2,6)=6, top=max(3,1)=3, right=min(10,16)=10, bottom=min(7,11)=7
    // -> intersection (6,3,4,4), valid=1.
    assert_eq!(
        step(&[
            ("x1", 2),
            ("y1", 3),
            ("w1", 8),
            ("h1", 4),
            ("x2", 6),
            ("y2", 1),
            ("w2", 10),
            ("h2", 10),
        ]),
        (6, 3, 4, 4, 1)
    );
}

// aabb_union (spatial-grid): the smallest AABB that contains both input boxes -- always
// defined (unlike aabb_intersect's overlap test), the merge/bounding counterpart to
// aabb_intersect's overlap predicate and aabb_contains' containment predicate.
#[test]
fn aabb_union_matches_defined_behaviour() {
    fn step(fields: &[(&str, u64)]) -> (u64, u64, u64, u64) {
        let mut cell = StateCell::bind(&cell_src("aabb_union"), "AabbUnion", None)
            .unwrap_or_else(|e| panic!("bind aabb_union: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap();
        (
            cell.get("ux").unwrap(),
            cell.get("uy").unwrap(),
            cell.get("uw").unwrap(),
            cell.get("uh").unwrap(),
        )
    }

    // Overlapping boxes (0,0,10,10) and (5,5,10,10) -> union spans (0,0) to (15,15).
    assert_eq!(
        step(&[
            ("x1", 0),
            ("y1", 0),
            ("w1", 10),
            ("h1", 10),
            ("x2", 5),
            ("y2", 5),
            ("w2", 10),
            ("h2", 10),
        ]),
        (0, 0, 15, 15)
    );

    // Disjoint boxes (0,0,5,5) and (20,20,5,5) -> union still spans their full extent (0,0,25,25).
    assert_eq!(
        step(&[
            ("x1", 0),
            ("y1", 0),
            ("w1", 5),
            ("h1", 5),
            ("x2", 20),
            ("y2", 20),
            ("w2", 5),
            ("h2", 5),
        ]),
        (0, 0, 25, 25)
    );

    // Box2 fully inside box1 -> union collapses to box1 exactly.
    assert_eq!(
        step(&[
            ("x1", 0),
            ("y1", 0),
            ("w1", 10),
            ("h1", 10),
            ("x2", 2),
            ("y2", 2),
            ("w2", 3),
            ("h2", 3),
        ]),
        (0, 0, 10, 10)
    );

    // Asymmetric boxes: mins and maxes come from different boxes on each axis
    // (box1 wins on the right edge, box2 wins on the bottom edge).
    assert_eq!(
        step(&[
            ("x1", 3),
            ("y1", 7),
            ("w1", 4),
            ("h1", 2),
            ("x2", 1),
            ("y2", 1),
            ("w2", 2),
            ("h2", 20),
        ]),
        (1, 1, 6, 20)
    );

    // Identical boxes -> union equals the box itself (idempotent).
    assert_eq!(
        step(&[
            ("x1", 5),
            ("y1", 5),
            ("w1", 10),
            ("h1", 10),
            ("x2", 5),
            ("y2", 5),
            ("w2", 10),
            ("h2", 10),
        ]),
        (5, 5, 10, 10)
    );
}

// grid_coords_u32 (spatial-grid): wide sibling of grid_coords over a u32 index/y pair —
// checks the divmod inverse relationship, a case where y exceeds u16 range (exercising the
// "wide" field), and the width==0 guard returning (0,0) instead of halting on DivByZero.
#[test]
fn grid_coords_u32_matches_defined_behaviour() {
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> StateCell {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap();
        cell
    }

    // index=0, width=5 -> x=0%5=0, y=0/5=0 (origin).
    let cell = step(
        "grid_coords_u32",
        "GridCoordsWide",
        &[("index", 0), ("width", 5)],
    );
    assert_eq!((cell.get("x"), cell.get("y")), (Some(0), Some(0)));

    // index=7, width=5 -> x=7%5=2, y=7/5=1: mirrors the narrow grid_coords case.
    let cell = step(
        "grid_coords_u32",
        "GridCoordsWide",
        &[("index", 7), ("width", 5)],
    );
    assert_eq!((cell.get("x"), cell.get("y")), (Some(2), Some(1)));

    // index=100000, width=300 -> 300*333=99900, remainder 100 -> x=100, y=333.
    let cell = step(
        "grid_coords_u32",
        "GridCoordsWide",
        &[("index", 100_000), ("width", 300)],
    );
    assert_eq!((cell.get("x"), cell.get("y")), (Some(100), Some(333)));

    // width=0 guard: must return (0, 0) rather than halting on DivByZero (unlike
    // div_floor_u32/mod_u32, which halt on a zero divisor).
    let cell = step(
        "grid_coords_u32",
        "GridCoordsWide",
        &[("index", 123_456), ("width", 0)],
    );
    assert_eq!((cell.get("x"), cell.get("y")), (Some(0), Some(0)));

    // index=u32::MAX=4294967295, width=65535 -> since 65535*65537 == 65536^2-1 == 4294967295
    // exactly, x=0, y=65537 -- a y value that overflows u16, exercising the genuinely "wide"
    // y field (this is exactly what distinguishes this cell from plain grid_coords).
    let cell = step(
        "grid_coords_u32",
        "GridCoordsWide",
        &[("index", 4_294_967_295), ("width", 65535)],
    );
    assert_eq!((cell.get("x"), cell.get("y")), (Some(0), Some(65537)));
}


// grid_index_u32 (spatial-grid): wide/checked sibling of grid_index -- the encode-side
// counterpart to grid_coords_u32's decode. Checks the flatten relationship y*width+x for a
// case matching plain grid_index, a case matching grid_coords_u32's own decode example, the
// exact round-trip partner of grid_coords_u32's wide-y (>u16) case, and both escalation paths
// (multiply overflow alone, and add overflow alone) rather than silent u32 wraparound.
#[test]
fn grid_index_u32_matches_hand_computed_values() {
    fn step(fields: &[(&str, u64)]) -> StateCell {
        let mut cell = StateCell::bind(&cell_src("grid_index_u32"), "GridIndexWide", None)
            .unwrap_or_else(|e| panic!("bind grid_index_u32: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell
    }

    // x=0, y=0, width=5 -> index = 0*5+0 = 0.
    let mut cell = step(&[("x", 0), ("y", 0), ("width", 5)]);
    cell.run(DEFAULT_CYCLES).unwrap();
    assert_eq!(cell.get("index"), Some(0));

    // x=2, y=1, width=5 -> index = 1*5+2 = 7 (mirrors grid_coords's index=7,width=5->x=2,y=1).
    let mut cell = step(&[("x", 2), ("y", 1), ("width", 5)]);
    cell.run(DEFAULT_CYCLES).unwrap();
    assert_eq!(cell.get("index"), Some(7));

    // x=100, y=333, width=300 -> index = 333*300+100 = 99900+100 = 100000
    // (mirrors grid_coords_u32's index=100000,width=300->x=100,y=333).
    let mut cell = step(&[("x", 100), ("y", 333), ("width", 300)]);
    cell.run(DEFAULT_CYCLES).unwrap();
    assert_eq!(cell.get("index"), Some(100_000));

    // x=0, y=65537, width=65535 -> row = 65537*65535 = 65536*65535+65535 = 4294901760+65535
    // = 4294967295 = u32::MAX exactly; index = 4294967295+0 = 4294967295 (fits, no overflow).
    // This is the exact round-trip partner of grid_coords_u32's index=u32::MAX,width=65535
    // -> x=0,y=65537 case (the wide-y case that overflows u16, distinguishing this cell from
    // plain grid_index).
    let mut cell = step(&[("x", 0), ("y", 65537), ("width", 65535)]);
    cell.run(DEFAULT_CYCLES).unwrap();
    assert_eq!(cell.get("index"), Some(4_294_967_295));

    // y=100000, width=65535 -> row = 100000*65535 = 6553500000 > u32::MAX (4294967295):
    // the multiply alone overflows u32, so this must escalate rather than wrap.
    let mut cell = step(&[("x", 0), ("y", 100_000), ("width", 65535)]);
    let report = cell.run(DEFAULT_CYCLES).unwrap();
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // y=65537, width=65535 -> row = u32::MAX exactly (fits, no overflow on the multiply);
    // x=1 -> u32::MAX+1 overflows the add alone, so this must also escalate.
    let mut cell = step(&[("x", 1), ("y", 65537), ("width", 65535)]);
    let report = cell.run(DEFAULT_CYCLES).unwrap();
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

// point_aabb_dist_sq (spatial-grid): exact squared distance from a point to the nearest
// point on/in an axis-aligned box -- 0 when the point is inside. Checks strict interior,
// single-axis clamp, diagonal-corner clamp, the box's far edge (inclusive, not half-open
// like point_in_rect), a clamp on the low side of both axes, and a degenerate zero-size
// box (which reduces to plain point-to-point squared distance, the classic 3-4-5 triangle).
#[test]
fn point_aabb_dist_sq_matches_hand_computed_expectations() {
    fn step(fields: &[(&str, u64)]) -> u64 {
        let mut cell = StateCell::bind(&cell_src("point_aabb_dist_sq"), "PointAabbDistSq", None)
            .unwrap_or_else(|e| panic!("bind point_aabb_dist_sq: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap();
        cell.get("dist_sq").unwrap()
    }

    // Strictly inside box (0,0,10,10) -> clamps to itself -> 0.
    assert_eq!(
        step(&[("px", 5), ("py", 5), ("rx", 0), ("ry", 0), ("rw", 10), ("rh", 10)]),
        0
    );

    // Outside on x only (py stays in range): cx clamps to rx+rw=10, cy=5.
    // dx = 15-10 = 5, dy = 0 -> dist_sq = 25.
    assert_eq!(
        step(&[("px", 15), ("py", 5), ("rx", 0), ("ry", 0), ("rw", 10), ("rh", 10)]),
        25
    );

    // Outside diagonally, nearest corner (10,10): dx=5, dy=5 -> dist_sq = 50.
    assert_eq!(
        step(&[("px", 15), ("py", 15), ("rx", 0), ("ry", 0), ("rw", 10), ("rh", 10)]),
        50
    );

    // Exactly on the box's far edge (rx+rw, py inside) -> inclusive, still counts as
    // inside (clamp uses strict >, not >=) -> dx=0, dy=0 -> dist_sq = 0.
    assert_eq!(
        step(&[("px", 10), ("py", 5), ("rx", 0), ("ry", 0), ("rw", 10), ("rh", 10)]),
        0
    );

    // Outside below both rx and ry (upper-left of box (5,5,10,10)): cx=5, cy=5,
    // dx=5, dy=5 -> dist_sq = 50.
    assert_eq!(
        step(&[("px", 0), ("py", 0), ("rx", 5), ("ry", 5), ("rw", 10), ("rh", 10)]),
        50
    );

    // Degenerate zero-size box (a single point at the origin): clamps fully to (0,0),
    // classic 3-4-5 triangle -> dist_sq = 3*3 + 4*4 = 25.
    assert_eq!(
        step(&[("px", 3), ("py", 4), ("rx", 0), ("ry", 0), ("rw", 0), ("rh", 0)]),
        25
    );
}

// aabb_from_points (spatial-grid): normalizes two arbitrary, unordered corner points into a
// well-formed AABB (x,y,w,h) -- the drag-select input shape, distinct from every other aabb_*
// cell here which takes two already-formed boxes rather than raw corners.
#[test]
fn aabb_from_points_matches_hand_computed_cases() {
    fn step(fields: &[(&str, u64)]) -> (u64, u64, u64, u64) {
        let mut cell = StateCell::bind(&cell_src("aabb_from_points"), "AabbFromPoints", None)
            .unwrap_or_else(|e| panic!("bind aabb_from_points: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap();
        (
            cell.get("x").unwrap(),
            cell.get("y").unwrap(),
            cell.get("w").unwrap(),
            cell.get("h").unwrap(),
        )
    }

    // Already-ordered corners (top-left, bottom-right) -> (2,3,8,5).
    assert_eq!(
        step(&[("x1", 2), ("y1", 3), ("x2", 10), ("y2", 8)]),
        (2, 3, 8, 5)
    );

    // Fully reversed corners (bottom-right, top-left) -- classic drag-up-and-left case --
    // yields the identical box as the case above.
    assert_eq!(
        step(&[("x1", 10), ("y1", 8), ("x2", 2), ("y2", 3)]),
        (2, 3, 8, 5)
    );

    // Mixed order: x descending, y ascending.
    assert_eq!(
        step(&[("x1", 20), ("y1", 1), ("x2", 5), ("y2", 9)]),
        (5, 1, 15, 8)
    );

    // Identical points collapse to a degenerate zero-area box at that point.
    assert_eq!(
        step(&[("x1", 7), ("y1", 7), ("x2", 7), ("y2", 7)]),
        (7, 7, 0, 0)
    );

    // Large u16 values near the top of range, reversed order -- checks abs-diff/min don't
    // overflow or wrap.
    assert_eq!(
        step(&[("x1", 65000), ("y1", 65535), ("x2", 100), ("y2", 0)]),
        (100, 0, 64900, 65535)
    );
}

// aabb_expand: inflates a single AABB by a uniform margin on all four sides. Checks an
// ordinary expand, the low-edge saturating_sub clamp at 0, a zero-margin no-op, a low-edge
// clamp where margin exceeds x/y, a high-edge saturating_add clamp at u16::MAX, and a margin
// so large that margin*2 itself overflows u16 before ever reaching w/h.
#[test]
fn aabb_expand_matches_hand_computed_cases() {
    fn step(fields: &[(&str, u64)]) -> (u16, u16, u16, u16) {
        let mut cell = StateCell::bind(&cell_src("aabb_expand"), "AabbExpand", None)
            .unwrap_or_else(|e| panic!("bind aabb_expand: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap();
        (
            cell.get("nx").unwrap() as u16,
            cell.get("ny").unwrap() as u16,
            cell.get("nw").unwrap() as u16,
            cell.get("nh").unwrap() as u16,
        )
    }

    // x=10,y=10,w=20,h=20,margin=5 -> nx=5,ny=5,nw=30,nh=30: ordinary expand.
    assert_eq!(
        step(&[("x", 10), ("y", 10), ("w", 20), ("h", 20), ("margin", 5)]),
        (5, 5, 30, 30)
    );

    // x=0,y=0,w=10,h=10,margin=3 -> nx/ny floor at 0 instead of wrapping negative.
    assert_eq!(
        step(&[("x", 0), ("y", 0), ("w", 10), ("h", 10), ("margin", 3)]),
        (0, 0, 16, 16)
    );

    // margin=0 is a no-op.
    assert_eq!(
        step(&[("x", 5), ("y", 5), ("w", 10), ("h", 10), ("margin", 0)]),
        (5, 5, 10, 10)
    );

    // margin exceeds x/y -> low edge saturates at 0 rather than wrapping.
    assert_eq!(
        step(&[("x", 3), ("y", 2), ("w", 5), ("h", 5), ("margin", 10)]),
        (0, 0, 25, 25)
    );

    // w+2*margin overflows u16::MAX -> saturates instead of wrapping.
    assert_eq!(
        step(&[
            ("x", 0),
            ("y", 0),
            ("w", 65530),
            ("h", 65530),
            ("margin", 10)
        ]),
        (0, 0, 65535, 65535)
    );

    // margin itself so large that margin*2 overflows u16 before being added to w/h.
    assert_eq!(
        step(&[("x", 0), ("y", 0), ("w", 100), ("h", 100), ("margin", 40000)]),
        (0, 0, 65535, 65535)
    );
}

#[test]
fn aabb_center_matches_defined_behaviour() {
    // Center point (cx,cy) of a single AABB (x,y,w,h): cx=x+w/2, cy=y+h/2 (integer floor division).
    // Distinct from aabb_union/aabb_intersection which relate two boxes -- this derives a point from one.
    fn center(x: u16, y: u16, w: u16, h: u16) -> (u16, u16) {
        let mut cell = StateCell::bind(&cell_src("aabb_center"), "AabbCenter", None)
            .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("x", x as u64).unwrap();
        cell.set("y", y as u64).unwrap();
        cell.set("w", w as u64).unwrap();
        cell.set("h", h as u64).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap();
        let cx = cell.get("cx").unwrap() as u16;
        let cy = cell.get("cy").unwrap() as u16;
        (cx, cy)
    }

    // Simple square at origin.
    assert_eq!(center(0, 0, 10, 10), (5, 5));
    // Offset rect with even w/h.
    assert_eq!(center(10, 20, 4, 6), (12, 23));
    // Unit box -- odd w/h=1 floors the half to 0, center stays at the corner.
    assert_eq!(center(5, 5, 1, 1), (5, 5));
    // Degenerate zero-size box.
    assert_eq!(center(0, 0, 0, 0), (0, 0));
    // Odd w/h elsewhere -- floor division on both axes.
    assert_eq!(center(100, 200, 7, 9), (103, 204));
}

#[test]
fn morton_encode_3d_matches_hand_computed_values() {
    // Checks the 3-axis Morton encode against hand-computed expected codes: the origin,
    // each axis's unit vector (pins down the interleave order x@bit0/y@bit1/z@bit2), the
    // combined (1,1,1) octree-cell code, and a max/masking case that exercises both the
    // "only the low 10 bits of each u16 matter" rule and the full 30-bit-set output.
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> StateCell {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap();
        cell
    }

    let cell = step("morton_encode_3d", "MortonEncode3d", &[("x", 0), ("y", 0), ("z", 0)]);
    assert_eq!(cell.get("code"), Some(0));

    // Unit vectors on each axis land at bit 0 (x), bit 1 (y), bit 2 (z) respectively.
    let cell = step("morton_encode_3d", "MortonEncode3d", &[("x", 1), ("y", 0), ("z", 0)]);
    assert_eq!(cell.get("code"), Some(1));
    let cell = step("morton_encode_3d", "MortonEncode3d", &[("x", 0), ("y", 1), ("z", 0)]);
    assert_eq!(cell.get("code"), Some(2));
    let cell = step("morton_encode_3d", "MortonEncode3d", &[("x", 0), ("y", 0), ("z", 1)]);
    assert_eq!(cell.get("code"), Some(4));

    // (1,1,1) sets all three low bits: the first octree cell's code.
    let cell = step("morton_encode_3d", "MortonEncode3d", &[("x", 1), ("y", 1), ("z", 1)]);
    assert_eq!(cell.get("code"), Some(7));

    // Max case: 65535 masks down to its low 10 bits (0x3FF) on every axis. Per-axis
    // splitBy3(0x3FF) = 0x09249249 (153391689); combined via the x|y<<1|z<<2 interleave:
    // 153391689 | (153391689<<1) | (153391689<<2) = 1073741823 = 0x3FFFFFFF (2^30 - 1).
    let cell = step(
        "morton_encode_3d",
        "MortonEncode3d",
        &[("x", 65535), ("y", 65535), ("z", 65535)],
    );
    assert_eq!(cell.get("code"), Some(1_073_741_823));
}

// morton_decode_3d: inverse of morton_encode_3d. Checks the three bit-planes (x at
// positions 0,3,6..., y at 1,4,7..., z at 2,5,8...) decode independently, plus the
// all-bits-set corner and a mixed-bit value hand-interleaved from x=5,y=3,z=1.
#[test]
fn morton_decode_3d_matches_hand_computed_values() {
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> StateCell {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap();
        cell
    }

    let cell = step("morton_decode_3d", "MortonDecode3d", &[("code", 0)]);
    assert_eq!(
        (cell.get("x"), cell.get("y"), cell.get("z")),
        (Some(0), Some(0), Some(0))
    );

    // bit0 -> x's bit0
    let cell = step("morton_decode_3d", "MortonDecode3d", &[("code", 1)]);
    assert_eq!(
        (cell.get("x"), cell.get("y"), cell.get("z")),
        (Some(1), Some(0), Some(0))
    );

    // bit1 -> y's bit0
    let cell = step("morton_decode_3d", "MortonDecode3d", &[("code", 2)]);
    assert_eq!(
        (cell.get("x"), cell.get("y"), cell.get("z")),
        (Some(0), Some(1), Some(0))
    );

    // bit2 -> z's bit0
    let cell = step("morton_decode_3d", "MortonDecode3d", &[("code", 4)]);
    assert_eq!(
        (cell.get("x"), cell.get("y"), cell.get("z")),
        (Some(0), Some(0), Some(1))
    );

    // all 30 bits set -> every coordinate maxes out at 1023 (10 bits)
    let cell = step(
        "morton_decode_3d",
        "MortonDecode3d",
        &[("code", 1_073_741_823)],
    );
    assert_eq!(
        (cell.get("x"), cell.get("y"), cell.get("z")),
        (Some(1023), Some(1023), Some(1023))
    );

    // code=87 (0x57 = 0b1010111) hand-interleaves to x=5 (0b101), y=3 (0b011), z=1 (0b001)
    let cell = step("morton_decode_3d", "MortonDecode3d", &[("code", 87)]);
    assert_eq!(
        (cell.get("x"), cell.get("y"), cell.get("z")),
        (Some(5), Some(3), Some(1))
    );
}
