//! Host-oracle tests for the fractions pack (`cell80/cells/fractions/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::cell_src;
use cell80::{StateCell, DEFAULT_CYCLES};

#[test]
fn fractions_cells_match_defined_behaviour() {
    fn verify(id: &str, strct: &str, fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // The GSM8K math-campaign fractions pack (Phase 2.3, M1 5/5 — the last authored pack)
    // — u32 numerator/denominator, eager reduction via an inline Euclidean GCD in every
    // cell that needs one (no shared gcd_u32 helper: M0's Tier 2 allows at most one u32
    // param per call, still not the two a general gcd_u32(a, b) needs — see
    // docs/library-growth.md). frac_floor/frac_ceil were skipped: they're exact duplicates
    // of the already-shipped div_floor_u32/div_ceil_u32.

    let (_, c) = verify("frac_reduce", "FracReduce", &[("n", 6), ("d", 8)]);
    assert_eq!((c.get("num"), c.get("den")), (Some(3), Some(4)));
    let (_, c) = verify("frac_reduce", "FracReduce", &[("n", 0), ("d", 5)]);
    assert_eq!((c.get("num"), c.get("den")), (Some(0), Some(1)));
    let (report, _) = verify("frac_reduce", "FracReduce", &[("n", 5), ("d", 0)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    let (_, c) = verify(
        "frac_add",
        "FracAdd",
        &[("na", 1), ("da", 2), ("nb", 1), ("db", 3)],
    );
    assert_eq!((c.get("num"), c.get("den")), (Some(5), Some(6))); // 1/2 + 1/3 = 5/6
    let (_, c) = verify(
        "frac_add",
        "FracAdd",
        &[("na", 1), ("da", 2), ("nb", 1), ("db", 2)],
    );
    assert_eq!((c.get("num"), c.get("den")), (Some(1), Some(1))); // 1/2 + 1/2 = 1
    let (report, _) = verify(
        "frac_add",
        "FracAdd",
        &[("na", 1), ("da", 0), ("nb", 1), ("db", 2)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    let (_, c) = verify(
        "frac_sub",
        "FracSub",
        &[("na", 3), ("da", 4), ("nb", 1), ("db", 4)],
    );
    assert_eq!((c.get("num"), c.get("den")), (Some(1), Some(2))); // 3/4 - 1/4 = 1/2
    let (report, _) = verify(
        "frac_sub",
        "FracSub",
        &[("na", 1), ("da", 4), ("nb", 3), ("db", 4)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05)); // 1/4 - 3/4 is negative

    let (_, c) = verify(
        "frac_mul",
        "FracMul",
        &[("na", 2), ("da", 3), ("nb", 3), ("db", 4)],
    );
    assert_eq!((c.get("num"), c.get("den")), (Some(1), Some(2))); // 2/3 * 3/4 = 1/2

    let (_, c) = verify(
        "frac_div",
        "FracDiv",
        &[("na", 1), ("da", 2), ("nb", 1), ("db", 3)],
    );
    assert_eq!((c.get("num"), c.get("den")), (Some(3), Some(2))); // (1/2) / (1/3) = 3/2
    let (report, _) = verify(
        "frac_div",
        "FracDiv",
        &[("na", 1), ("da", 2), ("nb", 0), ("db", 3)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06)); // dividing by a zero fraction

    let (report, _) = verify(
        "frac_cmp",
        "FracCmp",
        &[("na", 1), ("da", 2), ("nb", 1), ("db", 3)],
    );
    assert_eq!(report.result, 2); // 1/2 > 1/3
    let (report, _) = verify(
        "frac_cmp",
        "FracCmp",
        &[("na", 1), ("da", 2), ("nb", 2), ("db", 4)],
    );
    assert_eq!(report.result, 1); // 1/2 == 2/4
    let (report, _) = verify(
        "frac_cmp",
        "FracCmp",
        &[("na", 1), ("da", 3), ("nb", 1), ("db", 2)],
    );
    assert_eq!(report.result, 0); // 1/3 < 1/2

    let (report, _) = verify(
        "frac_eq",
        "FracEq",
        &[("na", 1), ("da", 2), ("nb", 2), ("db", 4)],
    );
    assert_eq!(report.result, 1); // equal despite unreduced 2/4
    let (report, _) = verify(
        "frac_eq",
        "FracEq",
        &[("na", 1), ("da", 2), ("nb", 1), ("db", 3)],
    );
    assert_eq!(report.result, 0);

    let (report, _) = verify("is_integer", "IsInteger", &[("n", 10), ("d", 5)]);
    assert_eq!(report.result, 1);
    let (report, _) = verify("is_integer", "IsInteger", &[("n", 10), ("d", 3)]);
    assert_eq!(report.result, 0);
    let (report, _) = verify("is_integer", "IsInteger", &[("n", 5), ("d", 0)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    let (_, c) = verify("frac_to_mixed", "FracToMixed", &[("n", 10), ("d", 4)]);
    assert_eq!(
        (c.get("whole"), c.get("num"), c.get("den")),
        (Some(2), Some(1), Some(2))
    ); // 10/4 = 2 1/2
    let (_, c) = verify("frac_to_mixed", "FracToMixed", &[("n", 9), ("d", 3)]);
    assert_eq!(
        (c.get("whole"), c.get("num"), c.get("den")),
        (Some(3), Some(0), Some(1))
    ); // 9/3 = 3 exactly

    let (_, c) = verify(
        "ratio_split2",
        "RatioSplit2",
        &[("total", 100), ("ratio_a", 3), ("ratio_b", 2)],
    );
    assert_eq!((c.get("part_a"), c.get("part_b")), (Some(60), Some(40)));
    let (_, c) = verify(
        "ratio_split2",
        "RatioSplit2",
        &[("total", 10), ("ratio_a", 1), ("ratio_b", 3)],
    );
    // truncated split, but the two parts always sum exactly to total.
    assert_eq!((c.get("part_a"), c.get("part_b")), (Some(2), Some(8)));
}

#[test]
fn fractions_wave2_cells_match_defined_behaviour() {
    fn verify(id: &str, strct: &str, fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // GSM8K math-campaign fractions pack, second slice — closing the gap against
    // docs/math-campaign-spec.md's ~20-cell estimate: a reciprocal, applying a fraction to
    // a whole (exact) vs scaling a fraction by an integer (stays a fraction), picking the
    // smaller/larger of two fractions (distinct from frac_cmp's ordering code), a 3-way
    // ratio split, a proper-fraction predicate, and the mixed-number pair
    // (frac_add_whole / mixed_to_frac, the latter the exact inverse of frac_to_mixed).

    let (_, c) = verify("frac_reciprocal", "FracReciprocal", &[("n", 3), ("d", 4)]);
    assert_eq!((c.get("num"), c.get("den")), (Some(4), Some(3)));
    let (report, _) = verify("frac_reciprocal", "FracReciprocal", &[("n", 0), ("d", 4)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    let (_, c) = verify(
        "frac_of_whole",
        "FracOfWhole",
        &[("n", 3), ("d", 4), ("whole", 20)],
    );
    assert_eq!(c.get("result"), Some(15)); // 3/4 of 20 = 15
    let (report, _) = verify(
        "frac_of_whole",
        "FracOfWhole",
        &[("n", 3), ("d", 4), ("whole", 10)],
    ); // 30/4 isn't a whole number: wrong-plan signal
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    let (_, c) = verify("frac_scale", "FracScale", &[("n", 2), ("d", 3), ("k", 4)]);
    assert_eq!((c.get("num"), c.get("den")), (Some(8), Some(3))); // (2/3)*4 = 8/3

    let (_, c) = verify(
        "frac_min",
        "FracMin",
        &[("na", 1), ("da", 2), ("nb", 1), ("db", 3)],
    );
    assert_eq!((c.get("num"), c.get("den")), (Some(1), Some(3))); // 1/3 < 1/2
    let (_, c) = verify(
        "frac_max",
        "FracMax",
        &[("na", 1), ("da", 2), ("nb", 1), ("db", 3)],
    );
    assert_eq!((c.get("num"), c.get("den")), (Some(1), Some(2))); // 1/2 > 1/3

    let (_, c) = verify(
        "ratio_split3",
        "RatioSplit3",
        &[
            ("total", 100),
            ("ratio_a", 1),
            ("ratio_b", 1),
            ("ratio_c", 2),
        ],
    );
    assert_eq!(
        (c.get("part_a"), c.get("part_b"), c.get("part_c")),
        (Some(25), Some(25), Some(50))
    );

    let (report, _) = verify("frac_is_proper", "FracIsProper", &[("n", 3), ("d", 4)]);
    assert_eq!(report.result, 1);
    let (report, _) = verify("frac_is_proper", "FracIsProper", &[("n", 4), ("d", 4)]);
    assert_eq!(report.result, 0);

    let (_, c) = verify(
        "frac_add_whole",
        "FracAddWhole",
        &[("n", 1), ("d", 3), ("whole", 2)],
    );
    assert_eq!((c.get("num"), c.get("den")), (Some(7), Some(3))); // 1/3 + 2 = 7/3

    let (_, c) = verify(
        "mixed_to_frac",
        "MixedToFrac",
        &[("whole", 2), ("num", 1), ("den", 2)],
    );
    assert_eq!((c.get("n"), c.get("d")), (Some(5), Some(2))); // 2 1/2 = 5/2
}

#[test]
fn math_wave3_fractions_slice() {
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

    // frac_avg2: (1/2 + 1/3)/2 = 5/12.
    let (_, _, cell) = step(
        "frac_avg2",
        "FracAvg2",
        &[("na", 1), ("da", 2), ("nb", 1), ("db", 3)],
    );
    assert_eq!((cell.get("num"), cell.get("den")), (Some(5), Some(12)));

    // frac_sub_from_whole: 3 - 1/4 = 11/4; 2 - 1/2 = 3/2; going negative escalates.
    let (_, _, cell) = step(
        "frac_sub_from_whole",
        "FracSubFromWhole",
        &[("whole", 3), ("n", 1), ("d", 4)],
    );
    assert_eq!((cell.get("num"), cell.get("den")), (Some(11), Some(4)));
    let (_, _, cell) = step(
        "frac_sub_from_whole",
        "FracSubFromWhole",
        &[("whole", 2), ("n", 1), ("d", 2)],
    );
    assert_eq!((cell.get("num"), cell.get("den")), (Some(3), Some(2)));
    let (_, report, _) = step(
        "frac_sub_from_whole",
        "FracSubFromWhole",
        &[("whole", 0), ("n", 1), ("d", 4)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

}

#[test]
fn wave4_width_precision_fractions_slice() {
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

    // frac_of_whole_floor: 90% of 23 = 20.7 -> floors to 20, never escalates (unlike
    // frac_of_whole, which would escalate on this exact input since it doesn't divide
    // evenly). Exact-dividing input still works identically to frac_of_whole.
    let (_, report, cell) = step(
        "frac_of_whole_floor",
        "FracOfWholeFloor",
        &[("n", 90), ("d", 100), ("whole", 23)],
    );
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(20));
    let (_, _, cell) = step(
        "frac_of_whole_floor",
        "FracOfWholeFloor",
        &[("n", 3), ("d", 4), ("whole", 20)],
    );
    assert_eq!(cell.get("result"), Some(15));
    let (_, report, _) = step(
        "frac_of_whole_floor",
        "FracOfWholeFloor",
        &[("n", 1), ("d", 0), ("whole", 5)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
    let (_, report, _) = step(
        "frac_of_whole_floor",
        "FracOfWholeFloor",
        &[("n", 4_294_967_295), ("d", 1), ("whole", 2)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}
