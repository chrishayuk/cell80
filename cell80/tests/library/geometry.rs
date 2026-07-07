//! Host-oracle tests for the geometry pack (`cell80/cells/geometry/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::{cell_src, run_cell};
use cell80::{StateCell, DEFAULT_CYCLES};

#[test]
fn math_aime_pack_second_slice_geometry_slice() {
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

    // shoelace_area_x2: twice a triangle's area; winding order doesn't change the |.|;
    // a degenerate (all-coincident-points) triangle is 0.
    let (_, _, cell) = step(
        "shoelace_area_x2",
        "ShoelaceAreaX2",
        &[
            ("x1", 0),
            ("y1", 0),
            ("x2", 4),
            ("y2", 0),
            ("x3", 0),
            ("y3", 3),
        ],
    );
    assert_eq!(cell.get("result"), Some(12)); // right triangle, legs 4 and 3, area 6, x2 = 12
    let (_, _, cell) = step(
        "shoelace_area_x2",
        "ShoelaceAreaX2",
        &[
            ("x1", 0),
            ("y1", 0),
            ("x2", 0),
            ("y2", 3),
            ("x3", 4),
            ("y3", 0),
        ],
    );
    assert_eq!(cell.get("result"), Some(12)); // reversed winding, same |.|
    let (_, _, cell) = step(
        "shoelace_area_x2",
        "ShoelaceAreaX2",
        &[
            ("x1", 1),
            ("y1", 1),
            ("x2", 1),
            ("y2", 1),
            ("x3", 1),
            ("y3", 1),
        ],
    );
    assert_eq!(cell.get("result"), Some(0));
}

#[test]
fn geometry_combinatorics_sequences_geometry_slice() {
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

    // shoelace_area_x2_quad: unit square -> 2; degenerate (all points coincide) -> 0.
    let (_, _, cell) = step(
        "shoelace_area_x2_quad",
        "ShoelaceAreaX2Quad",
        &[
            ("x1", 0),
            ("y1", 0),
            ("x2", 1),
            ("y2", 0),
            ("x3", 1),
            ("y3", 1),
            ("x4", 0),
            ("y4", 1),
        ],
    );
    assert_eq!(cell.get("result"), Some(2));
    let (_, _, cell) = step(
        "shoelace_area_x2_quad",
        "ShoelaceAreaX2Quad",
        &[
            ("x1", 5),
            ("y1", 5),
            ("x2", 5),
            ("y2", 5),
            ("x3", 5),
            ("y3", 5),
            ("x4", 5),
            ("y4", 5),
        ],
    );
    assert_eq!(cell.get("result"), Some(0));

    // triangle_is_valid: 3-4-5 is valid; 1-1-3 fails the inequality; 1-2-3 is degenerate
    // (collinear, fails strictly).
    assert_eq!(run_cell("triangle_is_valid", &[3, 4, 5]), 1);
    assert_eq!(run_cell("triangle_is_valid", &[1, 1, 3]), 0);
    assert_eq!(run_cell("triangle_is_valid", &[1, 2, 3]), 0);

    // Geometry (shoelace_area_x2_quad, triangle_is_valid), combinatorics
    // (fibonacci_checked_u32, catalan_number, derangement_count), and sequences
    // (arithmetic_series_sum, geometric_series_sum) — requested as a broad next batch after
    // the MATH/AIME and backlog packs (sort3, the batch's one "algorithm", was scoped but
    // refused by the admission gate — see the note near the end of this test). Deliberately
    // NOT built (compose from existing cells instead, per this session's own rule):
    // Pythagorean-triple check (mul/add/eq), rectangle area/perimeter (mul/add),
    // collinearity (shoelace_area_x2 == 0), subset count (pow(2,n)), permutations with
    // repetition (pow(n,k)), stars-and-bars (choose(n-1,k-1)), multinomial coefficients
    // (two choose calls). Still blocked: Stirling numbers, ISBN/IBAN/UPC, and
    // percentile-from-histogram all need array/bytes[N] state fields, never yet exercised.

    // sort3 was scoped (min, mid, max) as a 3-tuple) but never shipped: the admission gate
    // refused it as a behavioural duplicate of min3, agreement 1.00 — correctly, since the
    // fingerprint only digests the primary (HL) register for a free fn with no state
    // (cell80/src/fingerprint.rs), and sort3's first tuple slot is, by construction, always
    // exactly min3's entire output. No reordering of the tuple escapes this: whichever of
    // min/mid/max lands first will always exactly match min3/median3/max3's own output for
    // every input, since a sort's outputs are definitionally those three statistics. Not a
    // false positive to work around — the extra capability (getting mid and max too) lives
    // entirely in registers the gate doesn't currently compare for duplicate-detection
    // purposes, a real gap worth someone revisiting in fingerprint.rs itself, not by hacking
    // around it here.
}

#[test]
fn aime_geometry_cos_and_heron_cells_match_defined_behaviour() {
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // cos_frac_from_sides: 3-4-5 right triangle, angle opposite the hypotenuse (5) is
    // 90 degrees -> cos = 0/1.
    let (_, cell) = step(
        "cos_frac_from_sides",
        "CosFracFromSides",
        &[("a", 3), ("b", 4), ("c", 5)],
    );
    assert_eq!(cell.get("mag_num"), Some(0));
    assert_eq!(cell.get("neg_num"), Some(0));
    assert_eq!(cell.get("den"), Some(1));
    // Equilateral: every angle is 60 degrees, cos 60 = 1/2.
    let (_, cell) = step(
        "cos_frac_from_sides",
        "CosFracFromSides",
        &[("a", 2), ("b", 2), ("c", 2)],
    );
    assert_eq!(cell.get("mag_num"), Some(1));
    assert_eq!(cell.get("neg_num"), Some(0));
    assert_eq!(cell.get("den"), Some(2));
    // Obtuse: a=2,b=2,c=3 -> cos C = (4+4-9)/8 = -1/8, sign-magnitude negative.
    let (_, cell) = step(
        "cos_frac_from_sides",
        "CosFracFromSides",
        &[("a", 2), ("b", 2), ("c", 3)],
    );
    assert_eq!(cell.get("mag_num"), Some(1));
    assert_eq!(cell.get("neg_num"), Some(1));
    assert_eq!(cell.get("den"), Some(8));
    // Not a triangle (1 + 1 <= 5): out_of_domain.
    let (report, _) = step(
        "cos_frac_from_sides",
        "CosFracFromSides",
        &[("a", 1), ("b", 1), ("c", 5)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // heron_16a2: 3-4-5 -> area 6, 16*6^2 = 576.
    let (_, cell) = step("heron_16a2", "Heron16A2", &[("a", 3), ("b", 4), ("c", 5)]);
    assert_eq!(cell.get("result"), Some(576));
    // Equilateral side 2: area = sqrt(3), 16*3 = 48.
    let (_, cell) = step("heron_16a2", "Heron16A2", &[("a", 2), ("b", 2), ("c", 2)]);
    assert_eq!(cell.get("result"), Some(48));
    // Not a triangle: out_of_domain.
    let (report, _) = step("heron_16a2", "Heron16A2", &[("a", 1), ("b", 1), ("c", 5)]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
    // Large equilateral triangle: the final factor-pair product overflows u32.
    let (report, _) = step(
        "heron_16a2",
        "Heron16A2",
        &[("a", 30000), ("b", 30000), ("c", 30000)],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
    // cos_frac_from_sides + heron_16a2: the AIME geometry pair that trades a real
    // square root for exact fraction/integer arithmetic (law of cosines and Heron's
    // formula rearranged to avoid one). Both escalate (0xFF06) on an invalid triangle.
}
