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
