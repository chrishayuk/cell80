//! Host-oracle tests for the ranking-stats pack (`cell80/cells/ranking-stats/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::{cell_src, run_cell};
use cell80::{StateCell, DEFAULT_CYCLES};

#[test]
fn wave4_scoring_choice_generalization_ranking_stats_slice() {
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> u16 {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // argmax3_u32 / argmin3_u32: exercised past the u16 ceiling.
    assert_eq!(
        step(
            "argmax3_u32",
            "Argmax3Wide",
            &[("a", 100_000), ("b", 200_000), ("c", 150_000)]
        ),
        1
    );
    assert_eq!(
        step(
            "argmax3_u32",
            "Argmax3Wide",
            &[("a", 100_000), ("b", 100_000), ("c", 100_000)]
        ),
        0 // tie -> lowest index
    );
    assert_eq!(
        step(
            "argmin3_u32",
            "Argmin3Wide",
            &[("a", 300_000), ("b", 100_000), ("c", 200_000)]
        ),
        1
    );
    assert_eq!(
        step(
            "argmin3_u32",
            "Argmin3Wide",
            &[("a", 100_000), ("b", 100_000), ("c", 100_000)]
        ),
        0 // tie -> lowest index
    );

    // Wave 4, slice 2: scoring/choice generalization — wide siblings of argmax3/argmin3/
    // is_clear_winner (past the u16 ceiling), and the 2-candidate siblings of choose_best3
    // for the common two-option case. choose_lowest_cost2/choose_highest_profit2 from the
    // original ~100-cell proposal were folded into choose_worst2/choose_best2's own tags
    // rather than shipped as four near-identical cells.
}

#[test]
fn first_wave_ranking_stats_cells_match_defined_behaviour() {
    let cases: &[(&str, &[u16], u16)] = &[
        ("min3", &[5, 2, 8], 2),
        ("min3", &[9, 9, 9], 9),
        ("max3", &[5, 2, 8], 8),
        ("max3", &[1, 40000, 2], 40000),
        ("median3", &[5, 2, 8], 5),
        ("median3", &[1, 2, 3], 2),
        ("median3", &[40000, 65535, 1], 40000),
        ("argmax3", &[5, 2, 8], 2),
        ("argmax3", &[9, 9, 9], 0),
        ("argmin3", &[5, 8, 2], 2),
        ("sum3", &[10, 20, 30], 60),
        ("sum3", &[60000, 60000, 60000], 65535),
        ("mean3", &[10, 20, 30], 20),
        ("mean3", &[65535, 65535, 65535], 65535),
        ("range3", &[1, 40000, 100], 39999),
        ("mode3", &[5, 5, 3], 5),
        ("mode3", &[3, 5, 5], 5),
        ("mode3", &[1, 2, 3], 1),
        ("majority3", &[5, 5, 3], 1),
        ("majority3", &[1, 2, 3], 0),
        ("midrange3", &[1, 2, 9], 5),
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

// Checks Max4::run against hand-computed expectations: a plain max, a full tie,
// the max landing in the middle/first/third position (not just first-or-last),
// an all-zero floor, and the u16 ceiling value participating twice at different
// positions, to make sure the imax(imax(imax(a,b),c),d) nesting picks correctly
// regardless of which operand holds the true maximum.
#[test]
fn max4_hand_computed_cases() {
    fn cell_src() -> String {
        let cells_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cells");
        let p = cell80::find_cell_file(&cells_dir, "max4").expect("find max4.rs");
        std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
    }
    fn step(a: u64, b: u64, c: u64, d: u64) -> u16 {
        let mut cell = StateCell::bind(&cell_src(), "Max4", None).expect("bind Max4");
        cell.set("a", a).unwrap();
        cell.set("b", b).unwrap();
        cell.set("c", c).unwrap();
        cell.set("d", d).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // c is the strict max (8).
    assert_eq!(step(5, 2, 8, 1), 8);
    // four-way tie -> the tied value itself.
    assert_eq!(step(9, 9, 9, 9), 9);
    // max sits in the middle (b), not first or last.
    assert_eq!(step(1, 40000, 2, 39999), 40000);
    // all zero.
    assert_eq!(step(0, 0, 0, 0), 0);
    // max is the u16 ceiling, sitting first.
    assert_eq!(step(65535, 1, 2, 3), 65535);
    // max is the u16 ceiling, sitting third (not last).
    assert_eq!(step(1, 2, 65535, 4), 65535);
}

// min4: smallest of four values, the arity-4 sibling of min3 (same nested-imin pattern
// one level deeper). Checked: a strict-min case, a four-way tie, min in the middle
// position, all-zero, and two u16-ceiling-adjacent cases with the min sitting first vs.
// last, to rule out any off-by-position bug in the nested imin(imin(imin(...))) chain.
#[test]
fn min4_matches_nested_imin_pattern() {
    fn step(a: u64, b: u64, c: u64, d: u64) -> u16 {
        let mut cell = StateCell::bind(&cell_src("min4"), "Min4", None)
            .unwrap_or_else(|e| panic!("bind Min4: {e}"));
        cell.set("a", a).unwrap();
        cell.set("b", b).unwrap();
        cell.set("c", c).unwrap();
        cell.set("d", d).unwrap();
        cell.run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run Min4: {e}"))
            .result
    }

    assert_eq!(step(5, 8, 2, 9), 2); // c is the strict min
    assert_eq!(step(9, 9, 9, 9), 9); // four-way tie -> the tied value itself
    assert_eq!(step(40000, 1, 40001, 2), 1); // min sits in the middle (b)
    assert_eq!(step(0, 0, 0, 0), 0); // all zero
    assert_eq!(step(65535, 65534, 65533, 1), 1); // min sits last, others near u16 ceiling
    assert_eq!(step(1, 65535, 2, 3), 1); // min sits first, not last
}

#[test]
fn argmax4_hand_computed_cases() {
    // argmax4: index (0-3) of the largest of four values, ties -> lowest index —
    // extends argmax3's nested if-chain one level deeper to handle a fourth field.
    fn step(a: u64, b: u64, c: u64, d: u64) -> u16 {
        let mut cell = StateCell::bind(&cell_src("argmax4"), "Argmax4", None)
            .unwrap_or_else(|e| panic!("bind argmax4: {e}"));
        cell.set("a", a).unwrap();
        cell.set("b", b).unwrap();
        cell.set("c", c).unwrap();
        cell.set("d", d).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // c is the strict max (8), at index 2.
    assert_eq!(step(5, 2, 8, 1), 2);
    // four-way tie -> lowest index wins.
    assert_eq!(step(9, 9, 9, 9), 0);
    // strictly increasing -> last index (3) wins.
    assert_eq!(step(1, 2, 3, 4), 3);
    // a and c tie for max (100), b and d are smaller -> lowest tied index (0) wins.
    assert_eq!(step(100, 50, 100, 99), 0);
    // d alone is the strict max, sitting past a run of equal smaller values.
    assert_eq!(step(0, 0, 0, 1), 3);
    // b and d tie for max (10) with c in between smaller -> lowest tied index (1) wins.
    assert_eq!(step(1, 10, 5, 10), 1);
}

#[test]
fn argmin4_index_of_smallest_of_four_ties_to_lowest_index() {
    // argmin4: state cell { a, b, c, d } -> u16, the four-value sibling of argmin3.
    // Extends argmin3's if-chain one level deeper; ties resolve to the lowest index.
    fn step(a: u64, b: u64, c: u64, d: u64) -> u16 {
        let mut cell = StateCell::bind(&cell_src("argmin4"), "Argmin4", None)
            .unwrap_or_else(|e| panic!("bind argmin4: {e}"));
        cell.set("a", a).unwrap();
        cell.set("b", b).unwrap();
        cell.set("c", c).unwrap();
        cell.set("d", d).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    assert_eq!(step(5, 2, 8, 1), 3); // min is d (last slot)
    assert_eq!(step(5, 2, 8, 3), 1); // min is b (second slot)
    assert_eq!(step(9, 9, 9, 9), 0); // all equal -> tie resolves to lowest index
    assert_eq!(step(1, 9, 9, 9), 0); // min is a itself
    assert_eq!(step(10, 3, 7, 3), 1); // tie between b and d (both 3) -> lower index (1) wins
    assert_eq!(step(2, 9, 2, 9), 0); // tie between a and c (both 2) -> lower index (0) wins
}

// mean4: state cell { a, b, c, d } -> u16, the four-value sibling of mean3. Extends
// mean3's div/remainder-recombine trick (a/4+b/4+c/4+d/4 + remainders/4) one operand
// deeper so the sum never has to be computed as a single wide intermediate. Checked:
// an exact-multiple-of-4 case, all four at the u16 ceiling (proves no silent overflow),
// a floor-rounding case, an all-zero floor, a remainder-recombine case, and two
// ceiling values mixed with two zeros (floor of a half-integer mean).
#[test]
fn mean4_hand_computed_cases() {
    fn cell_src() -> String {
        let cells_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cells");
        let p = cell80::find_cell_file(&cells_dir, "mean4").expect("find mean4.rs");
        std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
    }
    fn step(a: u64, b: u64, c: u64, d: u64) -> u16 {
        let mut cell = StateCell::bind(&cell_src(), "Mean4", None).expect("bind Mean4");
        cell.set("a", a).unwrap();
        cell.set("b", b).unwrap();
        cell.set("c", c).unwrap();
        cell.set("d", d).unwrap();
        cell.run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run Mean4: {e}"))
            .result
    }

    // exact multiples of 4, no remainder carry: (10+20+30+40)/4 = 25
    assert_eq!(step(10, 20, 30, 40), 25);
    // all at the u16 ceiling -> mean must equal the ceiling itself, no silent
    // overflow from the div/remainder-recombine trick
    assert_eq!(step(65535, 65535, 65535, 65535), 65535);
    // small ascending values: (1+2+3+4)/4 = 2 (floor)
    assert_eq!(step(1, 2, 3, 4), 2);
    // all zero -> 0
    assert_eq!(step(0, 0, 0, 0), 0);
    // remainder-heavy case: (5+5+5+5)/4 = 5 exactly, remainders (1 each) recombine to 1
    assert_eq!(step(5, 5, 5, 5), 5);
    // two at the u16 ceiling, two zero: (65535+65535+0+0)/4 = 32767 (floor of 131070/4 = 32767.5)
    assert_eq!(step(65535, 65535, 0, 0), 32767);
}

// midrange4: (min4 + max4) / 2, via the same (lo & hi) + ((lo ^ hi) >> 1) bit trick
// midrange3 uses, now over four inputs (imin/imax nested three deep). Checked: a plain
// case with min/max in interior positions, a four-way tie, the exact-average lower- and
// upper-bound extremes (0/65535 exercising the floor-division truncation, and a
// ceiling-adjacent case with an exact non-truncating average), to rule out any
// off-by-position bug in the imin/imax chains or an off-by-one in the bit-trick average.
#[test]
fn midrange4_hand_computed_cases() {
    fn step(a: u64, b: u64, c: u64, d: u64) -> u16 {
        let mut cell = StateCell::bind(&cell_src("midrange4"), "Midrange4", None)
            .unwrap_or_else(|e| panic!("bind Midrange4: {e}"));
        cell.set("a", a).unwrap();
        cell.set("b", b).unwrap();
        cell.set("c", c).unwrap();
        cell.set("d", d).unwrap();
        cell.run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run Midrange4: {e}"))
            .result
    }

    // min (1) sits first, max (9) sits third -> (1+9)/2 = 5.
    assert_eq!(step(1, 2, 9, 4), 5);
    // four-way tie -> min = max = 5 -> midrange = 5.
    assert_eq!(step(5, 5, 5, 5), 5);
    // min=0, max=65535 (odd) -> (0+65535)/2 floors to 32767.
    assert_eq!(step(0, 0, 0, 65535), 32767);
    // min=1, max=65535 -> (1+65535)/2 = 32768 exactly, no truncation.
    assert_eq!(step(65535, 65534, 1, 2), 32768);
    // min sits in position b (10), max sits in position c (200) -> (10+200)/2 = 105.
    assert_eq!(step(100, 10, 200, 50), 105);
}

// range4: spread of four values (max4 - min4), the arity-4 sibling of range3 (same
// nested-imax/imin composition one level deeper). Checked: a strict max/min case, a
// four-way tie (range 0), an all-zero floor, and two u16-ceiling-adjacent cases with the
// max/min occupying different positions, to rule out any off-by-position bug in the
// imax(imax(imax(a,b),c),d) - imin(imin(imin(a,b),c),d) composition.
#[test]
fn range4_hand_computed_cases() {
    fn cell_src() -> String {
        let cells_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cells");
        let p = cell80::find_cell_file(&cells_dir, "range4").expect("find range4.rs");
        std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
    }
    fn step(a: u64, b: u64, c: u64, d: u64) -> u16 {
        let mut cell = StateCell::bind(&cell_src(), "Range4", None).expect("bind Range4");
        cell.set("a", a).unwrap();
        cell.set("b", b).unwrap();
        cell.set("c", c).unwrap();
        cell.set("d", d).unwrap();
        cell.run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run Range4: {e}"))
            .result
    }

    // max=8 (c), min=1 (d) -> 7
    assert_eq!(step(5, 2, 8, 1), 7);
    // four-way tie -> range 0
    assert_eq!(step(9, 9, 9, 9), 0);
    // max=40000 (b), min=1 (a) -> 39999
    assert_eq!(step(1, 40000, 2, 39999), 39999);
    // all zero -> range 0
    assert_eq!(step(0, 0, 0, 0), 0);
    // max=65535 (a), min=1 (b) -> 65534
    assert_eq!(step(65535, 1, 2, 3), 65534);
    // min sits first (a=1), max sits second (b=65535) -> 65534
    assert_eq!(step(1, 65535, 2, 3), 65534);
}

// Checks Median4::run against hand-computed expectations: a non-exact even split,
// a four-way tie, distinct values fed in unsorted order (to confirm the
// lo1/hi1/lo2/hi2 pairing is order-invariant), a case with values near the u16
// ceiling (to confirm the midrange-style average avoids overflow), and a case
// where two of the four values tie right at the median boundary.
#[test]
fn median4_hand_computed_cases() {
    fn step(a: u64, b: u64, c: u64, d: u64) -> u16 {
        let mut cell = StateCell::bind(&cell_src("median4"), "Median4", None)
            .unwrap_or_else(|e| panic!("bind Median4: {e}"));
        cell.set("a", a).unwrap();
        cell.set("b", b).unwrap();
        cell.set("c", c).unwrap();
        cell.set("d", d).unwrap();
        cell.run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run Median4: {e}"))
            .result
    }

    // sorted [1,2,3,4], middle two = 2,3 -> floor(2.5) = 2
    assert_eq!(step(1, 2, 3, 4), 2);
    // four-way tie -> the tied value itself
    assert_eq!(step(5, 5, 5, 5), 5);
    // distinct values, unordered inputs: sorted [1,3,7,10], middle two = 3,7 -> 5
    assert_eq!(step(10, 1, 7, 3), 5);
    // overflow-safety: sorted [1,2,60000,65535], middle two = 2,60000 -> floor(60002/2) = 30001
    assert_eq!(step(60000, 65535, 1, 2), 30001);
    // a pair-tie among the middle values: sorted [2,2,5,9], middle two = 2,5 -> floor(3.5) = 3
    assert_eq!(step(2, 2, 5, 9), 3);
}

#[test]
fn mode4_hand_computed_cases() {
    // mode4: state cell { a, b, c, d } -> u16, extends mode3's "first value found to
    // repeat" convention one level deeper via priority order: a's repeat (anywhere in
    // b/c/d) wins first, then b's repeat (in c/d), then c's repeat (in d), defaulting
    // to a when all four are distinct.
    fn cell_src() -> String {
        let cells_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cells");
        let p = cell80::find_cell_file(&cells_dir, "mode4").expect("find mode4.rs");
        std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
    }
    fn step(a: u64, b: u64, c: u64, d: u64) -> u16 {
        let mut cell = StateCell::bind(&cell_src(), "Mode4", None)
            .unwrap_or_else(|e| panic!("bind Mode4: {e}"));
        cell.set("a", a).unwrap();
        cell.set("b", b).unwrap();
        cell.set("c", c).unwrap();
        cell.set("d", d).unwrap();
        cell.run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run Mode4: {e}"))
            .result
    }

    // a,b,b,a: a repeats via a==d -> caught at priority step 1 -> a (5).
    assert_eq!(step(5, 7, 7, 5), 5);
    // a,b,b,c: b repeats via b==c, a's own checks all fail -> falls to step 2 -> b (7).
    assert_eq!(step(5, 7, 7, 9), 7);
    // all-same: a==b triggers immediately -> a (4).
    assert_eq!(step(4, 4, 4, 4), 4);
    // all-distinct: no equalities anywhere -> defaults to a (1).
    assert_eq!(step(1, 2, 3, 4), 1);
    // b,a,a,b 2-2 tie (7 and 5 each appear twice) -> a==d (7==7) catches it at step 1,
    // priority favors a's group -> 7 (not 5).
    assert_eq!(step(7, 5, 5, 7), 7);
    // only c and d repeat (c==d), nothing involving a or b repeats -> falls through
    // step 1 and step 2 to step 3 -> c (3).
    assert_eq!(step(1, 2, 3, 3), 3);
}

// all_distinct3: pairwise-distinct predicate over three values -- the exact logical
// complement of majority3 ("at least two of three are equal"). Checked: a strictly
// increasing all-distinct triple, a==b duplicate, all-three-equal, a==c duplicate
// (non-adjacent pair), b==c duplicate, and an all-distinct case spanning the full
// u16 range (0, 65535, and a value in between) to rule out any range-dependent bug.
#[test]
fn all_distinct3_pairwise_distinct_matches_hand_computed_cases() {
    let cases: &[(&[u16], u16)] = &[
        (&[1, 2, 3], 1),     // all pairwise distinct
        (&[5, 5, 3], 0),     // a == b
        (&[5, 5, 5], 0),     // all three equal
        (&[1, 2, 1], 0),     // a == c (non-adjacent duplicate)
        (&[2, 3, 3], 0),     // b == c
        (&[0, 65535, 1], 1), // all distinct, spanning the u16 range
    ];

    let mut failures = Vec::new();
    for (args, exp) in cases {
        let got = run_cell("all_distinct3", args);
        if got != *exp {
            failures.push(format!("all_distinct3({args:?}) = {got}, expected {exp}"));
        }
    }
    assert!(
        failures.is_empty(),
        "cell mismatches:\n{}",
        failures.join("\n")
    );
}

#[test]
fn unanimous3_matches_hand_computed_expectations() {
    // unanimous3: strict all-agree predicate (a==b && b==c), distinct from majority3's
    // weaker at-least-two-agree threshold. Checks: all-equal -> 1, majority-but-not-all -> 0
    // (both a==b-only and a==c-only shapes), all-distinct -> 0, and the 0 / u16::MAX edges.
    let cases: &[(&str, &[u16], u16)] = &[
        ("unanimous3", &[5, 5, 5], 1), // all three equal -> unanimous
        ("unanimous3", &[5, 5, 3], 0), // majority (a==b) but not unanimous
        ("unanimous3", &[7, 3, 7], 0), // a==c but b differs -> not unanimous
        ("unanimous3", &[1, 2, 3], 0), // all distinct -> not unanimous
        ("unanimous3", &[0, 0, 0], 1), // all-zero edge case, all equal
        ("unanimous3", &[65535, 65535, 65535], 1), // max u16 value, all equal
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
fn argmax4_u32_matches_hand_computed_cases() {
    // Wide (u32) sibling of argmax4 — argmax3_u32's if-chain extended one level deeper,
    // exercised past the u16 ceiling so it's genuinely distinct from argmax4 itself.
    fn step(a: u64, b: u64, c: u64, d: u64) -> u16 {
        let mut cell = StateCell::bind(&cell_src("argmax4_u32"), "Argmax4Wide", None)
            .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("a", a).unwrap();
        cell.set("b", b).unwrap();
        cell.set("c", c).unwrap();
        cell.set("d", d).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // b is the unique largest, values past the u16 ceiling.
    assert_eq!(step(100_000, 200_000, 150_000, 50_000), 1);
    // all four equal -> tie resolves to lowest index.
    assert_eq!(step(100_000, 100_000, 100_000, 100_000), 0);
    // d is the unique largest, near u32's upper range.
    assert_eq!(step(1_000, 2_000, 3_000, 4_000_000_000), 3);
    // c and d tie for largest -> lower of the two indices (2) wins.
    assert_eq!(step(100, 200, 500_000, 500_000), 2);
    // a and c tie for largest -> lowest index overall (0) wins.
    assert_eq!(step(500_000, 100, 500_000, 100), 0);
    // b is the unique largest by a narrow margin, all wide values.
    assert_eq!(step(4_000_000, 4_000_001, 1, 2), 1);
}

#[test]
fn argmin4_u32_wide_ceiling_and_ties() {
    // argmin4_u32: wide sibling of argmin4, exercised past the u16 ceiling.
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> u16 {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // d is uniquely smallest, values exceed u16 ceiling.
    assert_eq!(
        step(
            "argmin4_u32",
            "Argmin4Wide",
            &[
                ("a", 100_000),
                ("b", 200_000),
                ("c", 150_000),
                ("d", 50_000)
            ]
        ),
        3
    );
    // b is uniquely smallest.
    assert_eq!(
        step(
            "argmin4_u32",
            "Argmin4Wide",
            &[
                ("a", 300_000),
                ("b", 100_000),
                ("c", 200_000),
                ("d", 250_000)
            ]
        ),
        1
    );
    // all four tie -> lowest index wins.
    assert_eq!(
        step(
            "argmin4_u32",
            "Argmin4Wide",
            &[
                ("a", 100_000),
                ("b", 100_000),
                ("c", 100_000),
                ("d", 100_000)
            ]
        ),
        0
    );
    // c is uniquely smallest by 1, a/b/d tie above it.
    assert_eq!(
        step(
            "argmin4_u32",
            "Argmin4Wide",
            &[("a", 70_000), ("b", 70_000), ("c", 69_999), ("d", 70_000)]
        ),
        2
    );
    // b ties with c but b has the lower index -> wins the tie.
    assert_eq!(
        step(
            "argmin4_u32",
            "Argmin4Wide",
            &[("a", 10), ("b", 3), ("c", 3), ("d", 8)]
        ),
        1
    );
}
