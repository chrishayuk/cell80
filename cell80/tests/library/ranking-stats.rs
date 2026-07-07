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
