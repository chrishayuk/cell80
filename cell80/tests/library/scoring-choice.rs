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

#[test]
fn choose_worst4_matches_hand_computed_cases() {
    // choose_worst4: lowest score wins (ties -> lowest index), the 4-candidate
    // sibling of choose_worst3 and the lowest-score counterpart of choose_best4.
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> u16 {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // score_a=5 is lowest among 5,9,7,6 -> val_a=100
    assert_eq!(
        step(
            "choose_worst4",
            "ChooseWorst4",
            &[
                ("val_a", 100),
                ("score_a", 5),
                ("val_b", 200),
                ("score_b", 9),
                ("val_c", 300),
                ("score_c", 7),
                ("val_d", 400),
                ("score_d", 6),
            ]
        ),
        100
    );

    // all tie at score 9 -> lowest index (a) -> val_a=100
    assert_eq!(
        step(
            "choose_worst4",
            "ChooseWorst4",
            &[
                ("val_a", 100),
                ("score_a", 9),
                ("val_b", 200),
                ("score_b", 9),
                ("val_c", 300),
                ("score_c", 9),
                ("val_d", 400),
                ("score_d", 9),
            ]
        ),
        100
    );

    // score_c=1 is lowest among 9,7,1,4 -> val_c=300
    assert_eq!(
        step(
            "choose_worst4",
            "ChooseWorst4",
            &[
                ("val_a", 100),
                ("score_a", 9),
                ("val_b", 200),
                ("score_b", 7),
                ("val_c", 300),
                ("score_c", 1),
                ("val_d", 400),
                ("score_d", 4),
            ]
        ),
        300
    );

    // score_d=0 is lowest among 9,7,5,0 -> val_d=400
    assert_eq!(
        step(
            "choose_worst4",
            "ChooseWorst4",
            &[
                ("val_a", 100),
                ("score_a", 9),
                ("val_b", 200),
                ("score_b", 7),
                ("val_c", 300),
                ("score_c", 5),
                ("val_d", 400),
                ("score_d", 0),
            ]
        ),
        400
    );

    // b, c, d tie at lowest score 3, a is higher -> lowest index among tied (b) wins -> val_b=200
    assert_eq!(
        step(
            "choose_worst4",
            "ChooseWorst4",
            &[
                ("val_a", 100),
                ("score_a", 9),
                ("val_b", 200),
                ("score_b", 3),
                ("val_c", 300),
                ("score_c", 3),
                ("val_d", 400),
                ("score_d", 3),
            ]
        ),
        200
    );
}

#[test]
fn clear_winner3_matches_hand_computed_cases() {
    // clear_winner3: decides decisiveness directly from three raw candidate scores by
    // exploiting that, for three values, the second-highest equals the median
    // (top = max(max(a,b),c), second = median3(a,b,c)); distinct from is_clear_winner /
    // clear_winner_u32, which require the caller to have already picked top/second.
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> u16 {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // top=90, second(median)=60, diff=30 >= margin 30 -> decisive (1)
    assert_eq!(
        step(
            "clear_winner3",
            "ClearWinner3",
            &[
                ("score_a", 90),
                ("score_b", 60),
                ("score_c", 20),
                ("margin", 30)
            ]
        ),
        1
    );

    // top=70, second=65, diff=5 < margin 20 -> not decisive (0)
    assert_eq!(
        step(
            "clear_winner3",
            "ClearWinner3",
            &[
                ("score_a", 70),
                ("score_b", 60),
                ("score_c", 65),
                ("margin", 20)
            ]
        ),
        0
    );

    // a and b tie at the top (90,90,20): the median IS the tied value (90), so diff=0 -> not decisive
    assert_eq!(
        step(
            "clear_winner3",
            "ClearWinner3",
            &[
                ("score_a", 90),
                ("score_b", 90),
                ("score_c", 20),
                ("margin", 5)
            ]
        ),
        0
    );

    // all three equal (50,50,50): top=second=50, diff=0 >= margin 0 -> decisive (1)
    assert_eq!(
        step(
            "clear_winner3",
            "ClearWinner3",
            &[
                ("score_a", 50),
                ("score_b", 50),
                ("score_c", 50),
                ("margin", 0)
            ]
        ),
        1
    );

    // c is the clear top (10,20,100): diff=80 >= margin 80, exact boundary -> decisive (1)
    assert_eq!(
        step(
            "clear_winner3",
            "ClearWinner3",
            &[
                ("score_a", 10),
                ("score_b", 20),
                ("score_c", 100),
                ("margin", 80)
            ]
        ),
        1
    );

    // same candidates, margin one more than the diff -> not decisive (0)
    assert_eq!(
        step(
            "clear_winner3",
            "ClearWinner3",
            &[
                ("score_a", 10),
                ("score_b", 20),
                ("score_c", 100),
                ("margin", 81)
            ]
        ),
        0
    );
}


// weighted_avg2 (scoring-choice): normalized two-input weighted mean, distinct from
// weighted_sum2 which returns the raw (un-normalized) a*wa + b*wb combined score.
#[test]
fn weighted_avg2_matches_hand_computed() {
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

    // Equal weights -> plain average: (10*1 + 20*1)/(1+1) = 15
    let (result, _, cell) = verify(
        "weighted_avg2",
        "WeightedAvg2",
        &[("a", 10), ("wa", 1), ("b", 20), ("wb", 1)],
    );
    assert_eq!((result, cell.get("sum")), (15, Some(30)));

    // Unequal weights, pulled toward a: (100*3 + 0*1)/(3+1) = 75
    let (result, _, cell) = verify(
        "weighted_avg2",
        "WeightedAvg2",
        &[("a", 100), ("wa", 3), ("b", 0), ("wb", 1)],
    );
    assert_eq!((result, cell.get("sum")), (75, Some(300)));

    // wa+wb == 0 -> guarded, result is 0 regardless of a/b.
    let (result, _, cell) = verify(
        "weighted_avg2",
        "WeightedAvg2",
        &[("a", 500), ("wa", 0), ("b", 900), ("wb", 0)],
    );
    assert_eq!((result, cell.get("sum")), (0, Some(0)));

    // Integer-division truncation (floor, not rounded): (7+8)/2 = 15/2 = 7
    let (result, _, cell) = verify(
        "weighted_avg2",
        "WeightedAvg2",
        &[("a", 7), ("wa", 1), ("b", 8), ("wb", 1)],
    );
    assert_eq!((result, cell.get("sum")), (7, Some(15)));

    // One weight zero -> degenerates to the other value exactly: (42*0+200*5)/5 = 200
    let (result, _, cell) = verify(
        "weighted_avg2",
        "WeightedAvg2",
        &[("a", 42), ("wa", 0), ("b", 200), ("wb", 5)],
    );
    assert_eq!((result, cell.get("sum")), (200, Some(1000)));

    // Both products near u32::MAX: numerator overflow escalates (needs_wider_math),
    // matching weighted_sum2's convention for the same shape of input.
    let (_, report, _) = verify(
        "weighted_avg2",
        "WeightedAvg2",
        &[("a", 65535), ("wa", 65535), ("b", 65535), ("wb", 65535)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn weighted_avg3_hand_computed_cases() {
    // weighted_avg3 = (a*wa + b*wb + c*wc) / (wa+wb+wc), 0 if the weight total is zero.
    // Normalized sibling of weighted_sum3 (same numerator, but divided by the weight total).
    fn verify(fields: &[(&str, u64)]) -> (u16, cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src("weighted_avg3"), "WeightedAvg3", None)
            .unwrap_or_else(|e| panic!("bind weighted_avg3: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        let result = report.result;
        (result, report, cell)
    }

    // Case 1: a=10,wa=1,b=5,wb=2,c=3,wc=4 -> sum=10+10+12=32, denom=7, 32/7=4 (floor).
    let (result, _, cell) = verify(&[
        ("a", 10),
        ("wa", 1),
        ("b", 5),
        ("wb", 2),
        ("c", 3),
        ("wc", 4),
    ]);
    assert_eq!((result, cell.get("sum")), (4, Some(32)));

    // Case 2: equal weights collapse to a plain average of a,b,c.
    let (result, _, cell) = verify(&[
        ("a", 10),
        ("wa", 1),
        ("b", 20),
        ("wb", 1),
        ("c", 30),
        ("wc", 1),
    ]);
    assert_eq!((result, cell.get("sum")), (20, Some(60)));

    // Case 3: all weights zero -> denom==0 is guarded to return 0, not a divide-by-zero halt.
    let (result, _, cell) = verify(&[
        ("a", 100),
        ("wa", 0),
        ("b", 200),
        ("wb", 0),
        ("c", 300),
        ("wc", 0),
    ]);
    assert_eq!((result, cell.get("sum")), (0, Some(0)));

    // Case 4: only b carries weight -> the average collapses exactly to b's value.
    let (result, _, cell) = verify(&[
        ("a", 50),
        ("wa", 0),
        ("b", 7),
        ("wb", 5),
        ("c", 1000),
        ("wc", 0),
    ]);
    assert_eq!((result, cell.get("sum")), (7, Some(35)));

    // Case 5: large weighted sum (65,535,000) that stays exact in the u32 `sum` field and
    // still fits u16 once divided by the weight total (65535000 / 1002 = 65404, rem 192).
    let (result, _, cell) = verify(&[
        ("a", 65535),
        ("wa", 1000),
        ("b", 0),
        ("wb", 1),
        ("c", 0),
        ("wc", 1),
    ]);
    assert_eq!((result, cell.get("sum")), (65404, Some(65_535_000)));

    // Case 6: overflow — a*wa and b*wb are each near u32::MAX, so the chained
    // add_checked_u32 must halt with 0xFF05 (needs_wider_math) rather than wrap silently.
    let (_, report, _) = verify(&[
        ("a", 65535),
        ("wa", 65535),
        ("b", 65535),
        ("wb", 65535),
        ("c", 1),
        ("wc", 1),
    ]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn is_clear_loser_bottom_side_margin_check() {
    // is_clear_loser is the bottom-side mirror of is_clear_winner: 1 if the
    // second-lowest score beats the bottom score by at least margin, else 0.
    // bottom > second_lowest is malformed and always reads as "no clear loser".
    assert_eq!(run_cell("is_clear_loser", &[10, 50, 30]), 1); // gap 40 >= margin 30 -> clear loser
    assert_eq!(run_cell("is_clear_loser", &[10, 30, 30]), 0); // gap 20 < margin 30 -> not clear
    assert_eq!(run_cell("is_clear_loser", &[50, 10, 5]), 0); // malformed: bottom > second_lowest
    assert_eq!(run_cell("is_clear_loser", &[20, 20, 0]), 1); // gap 0 >= margin 0 -> degenerate clear
    assert_eq!(run_cell("is_clear_loser", &[0, 65535, 65535]), 1); // u16 edge: gap 65535 >= 65535
    assert_eq!(run_cell("is_clear_loser", &[100, 100, 1]), 0); // gap 0 < margin 1 -> not clear
}

#[test]
fn clear_loser3_hand_computed_cases() {
    fn verify(fields: &[(&str, u64)]) -> u16 {
        let src = crate::common::cell_src("clear_loser3");
        let mut cell = cell80::StateCell::bind(&src, "ClearLoser3", None).unwrap();
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(cell80::DEFAULT_CYCLES).unwrap().result
    }

    // Decisive loser: bottom=5, second_lowest=10 (median of 10,5,20), diff=5 >= margin(3)
    assert_eq!(
        verify(&[("score_a", 10), ("score_b", 5), ("score_c", 20), ("margin", 3)]),
        1
    );
    // Not decisive: bottom=8, second_lowest=10, diff=2 < margin(3)
    assert_eq!(
        verify(&[("score_a", 10), ("score_b", 8), ("score_c", 20), ("margin", 3)]),
        0
    );
    // All tied: diff=0 >= margin(0) -> decisive (zero-margin ties count)
    assert_eq!(
        verify(&[("score_a", 7), ("score_b", 7), ("score_c", 7), ("margin", 0)]),
        1
    );
    // Exact boundary: bottom=1, second_lowest=4, diff=3 >= margin(3)
    assert_eq!(
        verify(&[("score_a", 1), ("score_b", 4), ("score_c", 9), ("margin", 3)]),
        1
    );
    // Two lowest tied at 5: second_lowest equals bottom, diff=0 < margin(1)
    assert_eq!(
        verify(&[("score_a", 5), ("score_b", 5), ("score_c", 20), ("margin", 1)]),
        0
    );
}

#[test]
fn score_margin3_matches_hand_computed_cases() {
    // score_margin3: the raw winning margin among three candidate scores (top minus
    // the second-highest, which for three values is exactly the median) — the value
    // clear_winner3 computes internally but only ever exposes as a >=margin boolean.
    fn step(args: &[u16]) -> u16 {
        run_cell("score_margin3", args)
    }

    // top=90, lo=20, second=60 (median) -> margin 30
    assert_eq!(step(&[90, 60, 20]), 30);

    // all three tied at 50 -> top=second=50 -> margin 0
    assert_eq!(step(&[50, 50, 50]), 0);

    // top=100, lo=10, second=55 -> margin 45
    assert_eq!(step(&[10, 100, 55]), 45);

    // two candidates tied at the max (65535,65535,0): top=65535, second=65535 -> margin 0;
    // also exercises u16 sum overflow (65535+65535+0 wraps past 65535) staying exact
    // because the internal a+b+c-lo-top computation is modular and second always fits u16
    assert_eq!(step(&[65535, 65535, 0]), 0);

    // a=1,b=2,c=65535: top=65535, lo=1, second=2 -> margin 65533
    // exercises overflow in the a+b+c sum before subtracting lo/top
    assert_eq!(step(&[1, 2, 65535]), 65533);
}
