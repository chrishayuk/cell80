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
