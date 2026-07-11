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
            ("x1", 0), ("y1", 0), ("w1", 10), ("h1", 10),
            ("x2", 5), ("y2", 5), ("w2", 10), ("h2", 10),
        ]),
        (5, 5, 5, 5, 1)
    );

    // Disjoint boxes (0,0,5,5) and (10,10,5,5) -> right(5) <= left(10) -> not valid.
    assert_eq!(
        step(&[
            ("x1", 0), ("y1", 0), ("w1", 5), ("h1", 5),
            ("x2", 10), ("y2", 10), ("w2", 5), ("h2", 5),
        ]),
        (0, 0, 0, 0, 0)
    );

    // Edge-touching boxes (0,0,5,5) and (5,0,5,5) -> right(5) > left(5) is false -> not valid
    // (matches aabb_intersect's edge-touching-doesn't-count convention).
    assert_eq!(
        step(&[
            ("x1", 0), ("y1", 0), ("w1", 5), ("h1", 5),
            ("x2", 5), ("y2", 0), ("w2", 5), ("h2", 5),
        ]),
        (0, 0, 0, 0, 0)
    );

    // Box2 fully inside box1 (0,0,20,20) contains (5,5,5,5) -> intersection equals box2.
    assert_eq!(
        step(&[
            ("x1", 0), ("y1", 0), ("w1", 20), ("h1", 20),
            ("x2", 5), ("y2", 5), ("w2", 5), ("h2", 5),
        ]),
        (5, 5, 5, 5, 1)
    );

    // Asymmetric partial overlap (2,3,8,4) [2,10)x[3,7) and (6,1,10,10) [6,16)x[1,11)
    // -> left=max(2,6)=6, top=max(3,1)=3, right=min(10,16)=10, bottom=min(7,11)=7
    // -> intersection (6,3,4,4), valid=1.
    assert_eq!(
        step(&[
            ("x1", 2), ("y1", 3), ("w1", 8), ("h1", 4),
            ("x2", 6), ("y2", 1), ("w2", 10), ("h2", 10),
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
            ("x1", 0), ("y1", 0), ("w1", 10), ("h1", 10),
            ("x2", 5), ("y2", 5), ("w2", 10), ("h2", 10),
        ]),
        (0, 0, 15, 15)
    );

    // Disjoint boxes (0,0,5,5) and (20,20,5,5) -> union still spans their full extent (0,0,25,25).
    assert_eq!(
        step(&[
            ("x1", 0), ("y1", 0), ("w1", 5), ("h1", 5),
            ("x2", 20), ("y2", 20), ("w2", 5), ("h2", 5),
        ]),
        (0, 0, 25, 25)
    );

    // Box2 fully inside box1 -> union collapses to box1 exactly.
    assert_eq!(
        step(&[
            ("x1", 0), ("y1", 0), ("w1", 10), ("h1", 10),
            ("x2", 2), ("y2", 2), ("w2", 3), ("h2", 3),
        ]),
        (0, 0, 10, 10)
    );

    // Asymmetric boxes: mins and maxes come from different boxes on each axis
    // (box1 wins on the right edge, box2 wins on the bottom edge).
    assert_eq!(
        step(&[
            ("x1", 3), ("y1", 7), ("w1", 4), ("h1", 2),
            ("x2", 1), ("y2", 1), ("w2", 2), ("h2", 20),
        ]),
        (1, 1, 6, 20)
    );

    // Identical boxes -> union equals the box itself (idempotent).
    assert_eq!(
        step(&[
            ("x1", 5), ("y1", 5), ("w1", 10), ("h1", 10),
            ("x2", 5), ("y2", 5), ("w2", 10), ("h2", 10),
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
    let cell = step("grid_coords_u32", "GridCoordsWide", &[("index", 0), ("width", 5)]);
    assert_eq!((cell.get("x"), cell.get("y")), (Some(0), Some(0)));

    // index=7, width=5 -> x=7%5=2, y=7/5=1: mirrors the narrow grid_coords case.
    let cell = step("grid_coords_u32", "GridCoordsWide", &[("index", 7), ("width", 5)]);
    assert_eq!((cell.get("x"), cell.get("y")), (Some(2), Some(1)));

    // index=100000, width=300 -> 300*333=99900, remainder 100 -> x=100, y=333.
    let cell = step("grid_coords_u32", "GridCoordsWide", &[("index", 100_000), ("width", 300)]);
    assert_eq!((cell.get("x"), cell.get("y")), (Some(100), Some(333)));

    // width=0 guard: must return (0, 0) rather than halting on DivByZero (unlike
    // div_floor_u32/mod_u32, which halt on a zero divisor).
    let cell = step("grid_coords_u32", "GridCoordsWide", &[("index", 123_456), ("width", 0)]);
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

