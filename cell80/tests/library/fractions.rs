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

#[test]
fn linear_solve_1var_matches_defined_behaviour() {
    // Mined from chuk-math-gym's linear_equations domain: the single-unknown sibling of
    // matrix_solve_2x2's two-unknown Cramer's-rule solve, ax + b = cx + d for x, exact
    // via sign-magnitude subtraction + gcd_u32 reduction (no i32 in the dialect yet).
    fn solve(id: &str, a: u64, b: u64, c: u64, d: u64) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), "LinearSolve1Var", None).unwrap();
        cell.set("a", a).unwrap();
        cell.set("b", b).unwrap();
        cell.set("c", c).unwrap();
        cell.set("d", d).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }
    fn i16_bits(v: i16) -> u64 {
        (v as u16) as u64
    }

    // 2x + 3 = 5x - 6 -> x = 3 (as 3/1).
    let (_, cell) = solve(
        "linear_solve_1var",
        i16_bits(2),
        i16_bits(3),
        i16_bits(5),
        i16_bits(-6),
    );
    assert_eq!(cell.get("num_mag"), Some(3));
    assert_eq!(cell.get("num_neg"), Some(0));
    assert_eq!(cell.get("den"), Some(1));
    // x = 2x + 3 -> x = -3 (as -3/1: num_neg = 1).
    let (_, cell) = solve(
        "linear_solve_1var",
        i16_bits(1),
        i16_bits(0),
        i16_bits(2),
        i16_bits(3),
    );
    assert_eq!(cell.get("num_mag"), Some(3));
    assert_eq!(cell.get("num_neg"), Some(1));
    assert_eq!(cell.get("den"), Some(1));
    // 4x + 2 = 4x + 9 -> a == c, no unique solution -> escalate.
    let (report, _) = solve(
        "linear_solve_1var",
        i16_bits(4),
        i16_bits(2),
        i16_bits(4),
        i16_bits(9),
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
}

#[test]
fn frac_of_whole_ceil_hand_computed_cases() {
    // Checks frac_of_whole_ceil: ceil(n/d * whole), never escalating on inexactness
    // (unlike frac_of_whole), only on d == 0 or an n*whole overflow.
    fn frac_of_whole_ceil(n: u32, d: u32, whole: u32) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src("frac_of_whole_ceil"), "FracOfWholeCeil", None)
            .unwrap_or_else(|e| panic!("bind frac_of_whole_ceil: {e}"));
        for (f, v) in [("n", n as u64), ("d", d as u64), ("whole", whole as u64)] {
            cell.set(f, v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // 90/100 of 23 = 20.7 -> ceil 21 (never escalates on inexactness, unlike frac_of_whole).
    let (report, cell) = frac_of_whole_ceil(90, 100, 23);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(21));

    // 3/4 of 20 = 15 exactly -> ceil of an exact value is itself.
    let (report, cell) = frac_of_whole_ceil(3, 4, 20);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(15));

    // 1/3 of 10 = 3.33... -> ceil 4.
    let (report, cell) = frac_of_whole_ceil(1, 3, 10);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(4));

    // zero divisor halts with out_of_domain (0xFF06).
    let (report, _cell) = frac_of_whole_ceil(1, 0, 5);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // n * whole overflows u32 (4_294_967_295 * 2) -> halts needs_wider_math (0xFF05).
    let (report, _cell) = frac_of_whole_ceil(4_294_967_295, 1, 2);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn frac_sub_whole_matches_hand_computed() {
    // frac_sub_whole: n/d - whole, reduced to lowest terms; escalates (0xFF05) rather than
    // go negative, mirroring frac_sub's own escalate-rather-than-go-negative convention.
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // 7/2 - 1 = 5/2 (already lowest terms)
    let (_, c) = step(
        "frac_sub_whole",
        "FracSubWhole",
        &[("n", 7), ("d", 2), ("whole", 1)],
    );
    assert_eq!((c.get("num"), c.get("den")), (Some(5), Some(2)));

    // 5/2 - 2 = 1/2
    let (_, c) = step(
        "frac_sub_whole",
        "FracSubWhole",
        &[("n", 5), ("d", 2), ("whole", 2)],
    );
    assert_eq!((c.get("num"), c.get("den")), (Some(1), Some(2)));

    // 6/4 - 1 = 2/4 = 1/2 (tests reduction after subtraction, not just before)
    let (_, c) = step(
        "frac_sub_whole",
        "FracSubWhole",
        &[("n", 6), ("d", 4), ("whole", 1)],
    );
    assert_eq!((c.get("num"), c.get("den")), (Some(1), Some(2)));

    // 4/2 - 2 = 0 exactly -> num=0, den=1 (the exact-zero-result path)
    let (_, c) = step(
        "frac_sub_whole",
        "FracSubWhole",
        &[("n", 4), ("d", 2), ("whole", 2)],
    );
    assert_eq!((c.get("num"), c.get("den")), (Some(0), Some(1)));

    // 1/4 - 1 would be negative (whole*d=4 > n=1) -> escalate 0xFF05, needs_wider_math
    let (report, _) = step(
        "frac_sub_whole",
        "FracSubWhole",
        &[("n", 1), ("d", 4), ("whole", 1)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // d == 0 -> escalate 0xFF06, out_of_domain
    let (report, _) = step(
        "frac_sub_whole",
        "FracSubWhole",
        &[("n", 1), ("d", 0), ("whole", 0)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
}

#[test]
fn frac_div_whole_matches_hand_computed() {
    // Divide a fraction by a whole number, staying a fraction: (n/d)/k = n/(d*k), reduced via gcd_u32.
    fn verify(fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src("frac_div_whole"), "FracDivWhole", None)
            .unwrap_or_else(|e| panic!("bind frac_div_whole: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // (2/3) / 4 = 2/12 = 1/6
    let (_, c) = verify(&[("n", 2), ("d", 3), ("k", 4)]);
    assert_eq!((c.get("num"), c.get("den")), (Some(1), Some(6)));

    // (6/4) / 3 = 6/12 = 1/2 (reduces through both n and d*k)
    let (_, c) = verify(&[("n", 6), ("d", 4), ("k", 3)]);
    assert_eq!((c.get("num"), c.get("den")), (Some(1), Some(2)));

    // (5/3) / 2 = 5/6 (already lowest terms)
    let (_, c) = verify(&[("n", 5), ("d", 3), ("k", 2)]);
    assert_eq!((c.get("num"), c.get("den")), (Some(5), Some(6)));

    // n == 0 short-circuits to 0/1 regardless of d, k
    let (_, c) = verify(&[("n", 0), ("d", 5), ("k", 3)]);
    assert_eq!((c.get("num"), c.get("den")), (Some(0), Some(1)));

    // d == 0 halts out_of_domain
    let (report, _) = verify(&[("n", 1), ("d", 0), ("k", 2)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // k == 0 halts out_of_domain
    let (report, _) = verify(&[("n", 1), ("d", 2), ("k", 0)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // d * k overflows u32 (3_000_000_000 * 2 > u32::MAX) halts needs_wider_math
    let (report, _) = verify(&[("n", 1), ("d", 3_000_000_000), ("k", 2)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn frac_is_improper_cases() {
    // Host-oracle check for frac_is_improper: the explicit complement of frac_is_proper
    // (n >= d rather than n < d), same escalation on zero denominator.
    fn verify(id: &str, strct: &str, fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // 3/4 : n < d -> proper, not improper -> 0
    let (report, _) = verify("frac_is_improper", "FracIsImproper", &[("n", 3), ("d", 4)]);
    assert_eq!(report.result, 0);

    // 4/4 : n == d, exactly one whole -> improper -> 1
    let (report, _) = verify("frac_is_improper", "FracIsImproper", &[("n", 4), ("d", 4)]);
    assert_eq!(report.result, 1);

    // 5/4 : n > d -> improper -> 1
    let (report, _) = verify("frac_is_improper", "FracIsImproper", &[("n", 5), ("d", 4)]);
    assert_eq!(report.result, 1);

    // 0/5 : n < d -> proper, not improper -> 0
    let (report, _) = verify("frac_is_improper", "FracIsImproper", &[("n", 0), ("d", 5)]);
    assert_eq!(report.result, 0);

    // 7/0 : zero denominator -> escalate out_of_domain, same halt code as frac_is_proper
    let (report, _) = verify("frac_is_improper", "FracIsImproper", &[("n", 7), ("d", 0)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
}
