//! Host-oracle tests for the bucket-convert pack (`cell80/cells/bucket-convert/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::run_cell;

#[test]
fn first_wave_bucket_convert_cells_match_defined_behaviour() {
    let cases: &[(&str, &[u16], u16)] = &[
        ("bucket3", &[5, 10, 20], 0),
        ("bucket3", &[15, 10, 20], 1),
        ("bucket3", &[25, 10, 20], 2),
        ("percent_to_byte", &[100], 255),
        ("percent_to_byte", &[50], 127),
        ("byte_to_percent", &[255], 100),
        ("byte_to_percent", &[127], 49),
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
fn bucket3_u32_matches_defined_behaviour() {
    // bucket3_u32: the wide u32 sibling of bucket3 -- x<t1 -> 0, x<t2 -> 1, else 2 --
    // for values beyond u16's 65535 ceiling. Exercises both threshold boundaries
    // (inclusive on the >= side) and a value deep into u32-only territory.
    fn step(x: u64, t1: u64, t2: u64) -> (u16, u64) {
        let mut cell =
            cell80::StateCell::bind(&crate::common::cell_src("bucket3_u32"), "Bucket3Wide", None)
                .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("x", x).unwrap();
        cell.set("t1", t1).unwrap();
        cell.set("t2", t2).unwrap();
        let report = cell.run(cell80::DEFAULT_CYCLES).unwrap();
        (report.result, cell.get("out").unwrap())
    }

    // Below t1 -> bucket 0.
    let (result, out) = step(100_000, 200_000, 300_000);
    assert_eq!((result, out), (0, 0));

    // Exactly at t1 (inclusive boundary, x>=t1) -> bucket 1.
    let (result, out) = step(200_000, 200_000, 300_000);
    assert_eq!((result, out), (1, 1));

    // Strictly between t1 and t2 -> bucket 1.
    let (result, out) = step(250_000, 200_000, 300_000);
    assert_eq!((result, out), (1, 1));

    // Exactly at t2 (inclusive boundary, x>=t2) -> bucket 2.
    let (result, out) = step(300_000, 200_000, 300_000);
    assert_eq!((result, out), (2, 2));

    // Well past u16::MAX (65535), the u16 bucket3 could never see this -> bucket 2.
    let (result, out) = step(4_000_000_000, 200_000, 300_000);
    assert_eq!((result, out), (2, 2));
}

#[test]
fn bucket4_matches_defined_behaviour() {
    // bucket4: the one-more-threshold arity sibling of bucket3 -- x<t1 -> 0, x<t2 -> 1,
    // x<t3 -> 2, else 3 (>= is inclusive on the upper side, matching bucket3's convention).
    // Requires a state cell purely for arg count (4 params: x + 3 thresholds).
    fn step(x: u16, t1: u16, t2: u16, t3: u16) -> (u16, u16) {
        let mut cell =
            cell80::StateCell::bind(&crate::common::cell_src("bucket4"), "Bucket4", None)
                .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("x", x as u64).unwrap();
        cell.set("t1", t1 as u64).unwrap();
        cell.set("t2", t2 as u64).unwrap();
        cell.set("t3", t3 as u64).unwrap();
        let report = cell.run(cell80::DEFAULT_CYCLES).unwrap();
        (report.result, cell.get("out").unwrap() as u16)
    }

    // Below t1 -> bucket 0.
    let (result, out) = step(5, 10, 20, 30);
    assert_eq!((result, out), (0, 0));

    // Exactly at t1 (inclusive boundary, x>=t1) -> bucket 1.
    let (result, out) = step(10, 10, 20, 30);
    assert_eq!((result, out), (1, 1));

    // Strictly between t2 and t3 -> bucket 2.
    let (result, out) = step(25, 10, 20, 30);
    assert_eq!((result, out), (2, 2));

    // Exactly at t3 (inclusive boundary, x>=t3) -> bucket 3.
    let (result, out) = step(30, 10, 20, 30);
    assert_eq!((result, out), (3, 3));

    // Well past all thresholds (u16::MAX) -> bucket 3.
    let (result, out) = step(65535, 10, 20, 30);
    assert_eq!((result, out), (3, 3));
}
