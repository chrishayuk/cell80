//! Host-oracle tests for the scoring-choice pack (`cell80/cells/scoring-choice/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::{cell_src, run_cell};
use cell80::{StateCell, DEFAULT_CYCLES};

#[test]
fn scoring_choice_cells_match_defined_behaviour() {
    fn verify(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        let result = report.result;
        (result, report, cell)
    }

    // The scoring/choice pack (library-growth.md "Next waves") — weighted_sum2/3
    // generalize the fixed-weight weighted_sum/weighted_sum_wide to caller-supplied
    // weights, so (unlike their fixed-small-weight siblings) a genuine u32 overflow is
    // reachable and escalates rather than silently wrapping.

    let (result, _, cell) = verify(
        "weighted_sum2",
        "WeightedSum2",
        &[("a", 10), ("wa", 3), ("b", 5), ("wb", 2)],
    );
    assert_eq!((result, cell.get("sum")), (40, Some(40)));
    let (_, report, _) = verify(
        "weighted_sum2",
        "WeightedSum2",
        &[("a", 65535), ("wa", 65535), ("b", 65535), ("wb", 65535)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05)); // both products near u32::MAX
    let (result, _, cell) = verify(
        "weighted_sum2",
        "WeightedSum2",
        &[("a", 1000), ("wa", 1000), ("b", 1), ("wb", 1)],
    );
    assert_eq!((result, cell.get("sum")), (65535, Some(1_000_001))); // saturates the u16 return, sum is exact

    let (result, _, cell) = verify(
        "weighted_sum3",
        "WeightedSum3",
        &[
            ("a", 10),
            ("wa", 1),
            ("b", 5),
            ("wb", 2),
            ("c", 3),
            ("wc", 4),
        ],
    );
    assert_eq!((result, cell.get("sum")), (32, Some(32)));
    let (_, report, _) = verify(
        "weighted_sum3",
        "WeightedSum3",
        &[
            ("a", 65535),
            ("wa", 65535),
            ("b", 65535),
            ("wb", 65535),
            ("c", 1),
            ("wc", 1),
        ],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // choose_best3: highest score wins; distinct from argmax3 since value != score here.
    let (result, _, _) = verify(
        "choose_best3",
        "ChooseBest3",
        &[
            ("val_a", 100),
            ("score_a", 5),
            ("val_b", 200),
            ("score_b", 9),
            ("val_c", 300),
            ("score_c", 7),
        ],
    );
    assert_eq!(result, 200);
    let (result, _, _) = verify(
        "choose_best3",
        "ChooseBest3",
        &[
            ("val_a", 100),
            ("score_a", 9),
            ("val_b", 200),
            ("score_b", 9),
            ("val_c", 300),
            ("score_c", 9),
        ],
    );
    assert_eq!(result, 100); // tie -> lowest index (a) wins

    assert_eq!(run_cell("is_clear_winner", &[90, 60, 20]), 1); // margin 30 >= 20
    assert_eq!(run_cell("is_clear_winner", &[70, 60, 20]), 0); // margin 10 < 20
    assert_eq!(run_cell("is_clear_winner", &[60, 90, 20]), 0); // malformed: top < second
}

#[test]
fn wave4_scoring_choice_generalization_scoring_choice_slice() {
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> u16 {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // clear_winner_u32: margin decisive; margin not met; malformed (top < second).
    assert_eq!(
        step(
            "clear_winner_u32",
            "ClearWinnerWide",
            &[("top", 200_000), ("second", 100_000), ("margin", 50_000)]
        ),
        1
    );
    assert_eq!(
        step(
            "clear_winner_u32",
            "ClearWinnerWide",
            &[("top", 150_000), ("second", 100_000), ("margin", 100_000)]
        ),
        0
    );
    assert_eq!(
        step(
            "clear_winner_u32",
            "ClearWinnerWide",
            &[("top", 100_000), ("second", 150_000), ("margin", 10)]
        ),
        0
    );

    // choose_best2 / choose_worst2: b wins, a wins, and the tie -> a convention.
    assert_eq!(
        step(
            "choose_best2",
            "ChooseBest2",
            &[
                ("val_a", 100),
                ("score_a", 5),
                ("val_b", 200),
                ("score_b", 9)
            ]
        ),
        200
    );
    assert_eq!(
        step(
            "choose_best2",
            "ChooseBest2",
            &[
                ("val_a", 100),
                ("score_a", 9),
                ("val_b", 200),
                ("score_b", 5)
            ]
        ),
        100
    );
    assert_eq!(
        step(
            "choose_best2",
            "ChooseBest2",
            &[
                ("val_a", 100),
                ("score_a", 9),
                ("val_b", 200),
                ("score_b", 9)
            ]
        ),
        100 // tie -> a
    );
    assert_eq!(
        step(
            "choose_worst2",
            "ChooseWorst2",
            &[
                ("val_a", 100),
                ("score_a", 9),
                ("val_b", 200),
                ("score_b", 5)
            ]
        ),
        200
    );
    assert_eq!(
        step(
            "choose_worst2",
            "ChooseWorst2",
            &[
                ("val_a", 100),
                ("score_a", 5),
                ("val_b", 200),
                ("score_b", 9)
            ]
        ),
        100
    );
    assert_eq!(
        step(
            "choose_worst2",
            "ChooseWorst2",
            &[
                ("val_a", 100),
                ("score_a", 9),
                ("val_b", 200),
                ("score_b", 9)
            ]
        ),
        100 // tie -> a
    );
}

#[test]
fn choose_worst3_matches_hand_computed_cases() {
    // choose_worst3: lowest score wins (ties -> lowest index), the 3-candidate
    // sibling of choose_worst2 and the lowest-score counterpart of choose_best3.
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> u16 {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // score_a=5 is lowest -> val_a=100
    assert_eq!(
        step(
            "choose_worst3",
            "ChooseWorst3",
            &[
                ("val_a", 100),
                ("score_a", 5),
                ("val_b", 200),
                ("score_b", 9),
                ("val_c", 300),
                ("score_c", 7),
            ]
        ),
        100
    );

    // all tie at score 9 -> lowest index (a) -> val_a=100
    assert_eq!(
        step(
            "choose_worst3",
            "ChooseWorst3",
            &[
                ("val_a", 100),
                ("score_a", 9),
                ("val_b", 200),
                ("score_b", 9),
                ("val_c", 300),
                ("score_c", 9),
            ]
        ),
        100
    );

    // score_b=3 is lowest -> val_b=200
    assert_eq!(
        step(
            "choose_worst3",
            "ChooseWorst3",
            &[
                ("val_a", 100),
                ("score_a", 9),
                ("val_b", 200),
                ("score_b", 3),
                ("val_c", 300),
                ("score_c", 7),
            ]
        ),
        200
    );

    // score_c=2 is lowest -> val_c=300
    assert_eq!(
        step(
            "choose_worst3",
            "ChooseWorst3",
            &[
                ("val_a", 100),
                ("score_a", 9),
                ("val_b", 200),
                ("score_b", 7),
                ("val_c", 300),
                ("score_c", 2),
            ]
        ),
        300
    );

    // b and c tie at lowest score 3, a is higher -> lowest index among tied (b) wins -> val_b=200
    assert_eq!(
        step(
            "choose_worst3",
            "ChooseWorst3",
            &[
                ("val_a", 100),
                ("score_a", 9),
                ("val_b", 200),
                ("score_b", 3),
                ("val_c", 300),
                ("score_c", 3),
            ]
        ),
        200
    );
}

#[test]
fn weighted_sum4_cells_match_defined_behaviour() {
    // weighted_sum4 generalizes weighted_sum2/weighted_sum3 to four caller-supplied
    // weights, chained through add_checked_u32 the same way, so a genuine u32
    // overflow is reachable and escalates rather than silently wrapping.
    fn verify(id: &str, strct: &str, fields: &[(&str, u64)]) -> (u16, cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        let result = report.result;
        (result, report, cell)
    }

    // Basic mixed weights: 10*3 + 5*2 + 1*4 + 2*5 = 30 + 10 + 4 + 10 = 54
    let (result, _, cell) = verify(
        "weighted_sum4",
        "WeightedSum4",
        &[
            ("a", 10),
            ("wa", 3),
            ("b", 5),
            ("wb", 2),
            ("c", 1),
            ("wc", 4),
            ("d", 2),
            ("wd", 5),
        ],
    );
    assert_eq!((result, cell.get("sum")), (54, Some(54)));

    // Sum fits comfortably in u32 (no escalate) but not in u16: 1000*1000 = 1,000,000,
    // so the u16 return saturates to 65535 while `sum` keeps the exact value.
    let (result, _, cell) = verify(
        "weighted_sum4",
        "WeightedSum4",
        &[
            ("a", 1000),
            ("wa", 1000),
            ("b", 0),
            ("wb", 0),
            ("c", 0),
            ("wc", 0),
            ("d", 0),
            ("wd", 0),
        ],
    );
    assert_eq!((result, cell.get("sum")), (65535, Some(1_000_000)));

    // Overflow: p1 = 65535*65535 = 4294836225, p2 = same, s1 already exceeds u32::MAX.
    let (_, report, _) = verify(
        "weighted_sum4",
        "WeightedSum4",
        &[
            ("a", 65535),
            ("wa", 65535),
            ("b", 65535),
            ("wb", 65535),
            ("c", 1),
            ("wc", 1),
            ("d", 0),
            ("wd", 0),
        ],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn choose_best4_matches_defined_behaviour() {
    // Picks the value of whichever of four (value, score) candidates has the highest
    // score; ties resolve to the lowest index, matching choose_best3's convention.
    fn verify(fields: &[(&str, u64)]) -> u16 {
        let mut cell = StateCell::bind(&cell_src("choose_best4"), "ChooseBest4", None)
            .unwrap_or_else(|e| panic!("bind choose_best4: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // b has the strictly highest score (9) -> val_b wins.
    assert_eq!(
        verify(&[
            ("val_a", 100),
            ("score_a", 5),
            ("val_b", 200),
            ("score_b", 9),
            ("val_c", 300),
            ("score_c", 7),
            ("val_d", 50),
            ("score_d", 3),
        ]),
        200
    );

    // All four scores tie at 9 -> lowest index (a) wins.
    assert_eq!(
        verify(&[
            ("val_a", 100),
            ("score_a", 9),
            ("val_b", 200),
            ("score_b", 9),
            ("val_c", 300),
            ("score_c", 9),
            ("val_d", 400),
            ("score_d", 9),
        ]),
        100
    );

    // d strictly dominates -> val_d wins.
    assert_eq!(
        verify(&[
            ("val_a", 10),
            ("score_a", 1),
            ("val_b", 20),
            ("score_b", 2),
            ("val_c", 30),
            ("score_c", 3),
            ("val_d", 40),
            ("score_d", 4),
        ]),
        40
    );

    // b and d tie at the highest score (5); b comes first -> val_b wins.
    assert_eq!(
        verify(&[
            ("val_a", 10),
            ("score_a", 1),
            ("val_b", 20),
            ("score_b", 5),
            ("val_c", 30),
            ("score_c", 2),
            ("val_d", 40),
            ("score_d", 5),
        ]),
        20
    );
}
