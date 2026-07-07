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
