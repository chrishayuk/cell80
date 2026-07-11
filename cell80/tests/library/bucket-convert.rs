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


#[test]
fn bucket4_u32_matches_defined_behaviour() {
    // bucket4_u32: the wide u32 sibling of bucket4 -- x<t1 -> 0, x<t2 -> 1, x<t3 -> 2,
    // else 3 (>= is inclusive on the upper side, matching bucket3_u32/bucket4's convention) --
    // for values beyond u16's 65535 ceiling. Exercises all three threshold boundaries plus
    // a value deep into u32-only territory.
    fn step(x: u64, t1: u64, t2: u64, t3: u64) -> (u16, u64) {
        let mut cell =
            cell80::StateCell::bind(&crate::common::cell_src("bucket4_u32"), "Bucket4Wide", None)
                .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("x", x).unwrap();
        cell.set("t1", t1).unwrap();
        cell.set("t2", t2).unwrap();
        cell.set("t3", t3).unwrap();
        let report = cell.run(cell80::DEFAULT_CYCLES).unwrap();
        (report.result, cell.get("out").unwrap())
    }

    // Below t1 -> bucket 0.
    let (result, out) = step(100_000, 200_000, 300_000, 400_000);
    assert_eq!((result, out), (0, 0));

    // Exactly at t1 (inclusive boundary, x>=t1) -> bucket 1.
    let (result, out) = step(200_000, 200_000, 300_000, 400_000);
    assert_eq!((result, out), (1, 1));

    // Strictly between t2 and t3 -> bucket 2.
    let (result, out) = step(350_000, 200_000, 300_000, 400_000);
    assert_eq!((result, out), (2, 2));

    // Exactly at t3 (inclusive boundary, x>=t3) -> bucket 3.
    let (result, out) = step(400_000, 200_000, 300_000, 400_000);
    assert_eq!((result, out), (3, 3));

    // Well past u16::MAX (65535), the u16 bucket4 could never see this -> bucket 3.
    let (result, out) = step(4_000_000_000, 200_000, 300_000, 400_000);
    assert_eq!((result, out), (3, 3));
}

#[test]
fn bucket3_i16_matches_hand_computed_expectations() {
    // bucket3_i16(x, t1, t2): the signed sibling of bucket3 -- x<t1 -> 0, x<t2 -> 1,
    // else 2 (>= inclusive on both boundaries, matching bucket3's convention) -- but
    // over i16 via plain signed comparison (no sign-magnitude needed: this is a single
    // signed value flowing through comparisons, not a combination of two signed
    // quantities). Negative args are passed/read as their two's-complement u16 bit
    // pattern (-5 <-> 65531), the convention this file's other i16 cases use throughout.
    fn run(x: u16, t1: u16, t2: u16) -> u16 {
        run_cell("bucket3_i16", &[x, t1, t2])
    }

    // x=-10, t1=-5, t2=5: x < t1 -> bucket 0.
    assert_eq!(run(65526, 65531, 5), 0);

    // x=-3, t1=-5, t2=5: t1 <= x < t2 -> bucket 1.
    assert_eq!(run(65533, 65531, 5), 1);

    // x=5, t1=-5, t2=5: x == t2 (inclusive boundary) -> bucket 2.
    assert_eq!(run(5, 65531, 5), 2);

    // x=-5, t1=-5, t2=5: x == t1 (inclusive boundary) -> bucket 1.
    assert_eq!(run(65531, 65531, 5), 1);

    // x=100, t1=-100, t2=-50: x well past both thresholds (still positive) -> bucket 2.
    assert_eq!(run(100, 65436, 65486), 2);

    // x=i16::MIN (-32768), t1=-100, t2=100: below both -> bucket 0.
    assert_eq!(run(32768, 65436, 100), 0);
}

#[test]
fn byte_to_permille_matches_defined_behaviour() {
    // byte_to_permille: 0..255 byte scale -> 0..1000 per-mille scale via the reduced
    // fraction b*200/51 (equivalent to b*1000/255, reduced by gcd(1000,255)=5 so the
    // multiply b*200 never exceeds u16::MAX -- max input 255*200=51000 fits comfortably).
    // Hand-computed floor-division expectations:
    //   0   -> 0
    //   1   -> 200/51   = 3   (51*3=153, rem 47)
    //   51  -> 10200/51 = 200 (exact)
    //   100 -> 20000/51 = 392 (51*392=19992, rem 8)
    //   128 -> 25600/51 = 501 (51*501=25551, rem 49)
    //   255 -> 51000/51 = 1000 (exact, max value)
    let cases: &[(&str, &[u16], u16)] = &[
        ("byte_to_permille", &[0], 0),
        ("byte_to_permille", &[1], 3),
        ("byte_to_permille", &[51], 200),
        ("byte_to_permille", &[100], 392),
        ("byte_to_permille", &[128], 501),
        ("byte_to_permille", &[255], 1000),
    ];

    let mut failures = Vec::new();
    for (id, args, exp) in cases {
        let got = run_cell(id, args);
        if got != *exp {
            failures.push(format!("{id}({args:?}) = {got}, expected {exp}"));
        }
    }
    assert!(failures.is_empty(), "cell mismatches:\n{}", failures.join("\n"));
}

#[test]
fn permille_to_byte_matches_defined_behaviour() {
    // permille_to_byte: converts a 0..1000 per-mille value to a 0..255 byte value
    // via the reduced fraction pm*51/200 (255/1000 reduced by gcd 5 -> 51/200).
    // Checks both scale endpoints, an exact-division midpoint, and two
    // truncating-division cases near the top of the range.
    let cases: &[(u16, u16)] = &[
        (0, 0),       // 0*51/200 = 0
        (1000, 255),  // 1000*51/200 = 51000/200 = 255 (exact top of scale)
        (200, 51),    // 200*51/200 = 10200/200 = 51 (exact)
        (500, 127),   // 500*51/200 = 25500/200 = 127.5 -> 127 (truncation)
        (999, 254),   // 999*51/200 = 50949/200 = 254.745 -> 254 (truncation)
    ];

    let mut failures = Vec::new();
    for (pm, exp) in cases {
        let got = run_cell("permille_to_byte", &[*pm]);
        if got != *exp {
            failures.push(format!("permille_to_byte({pm}) = {got}, expected {exp}"));
        }
    }
    assert!(
        failures.is_empty(),
        "cell mismatches:\n{}",
        failures.join("\n")
    );
}
