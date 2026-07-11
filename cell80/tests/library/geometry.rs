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

#[test]
fn wave11_geom_distance_3d_matches_defined_behaviour() {
    // Wave 11 (docs/math-server-map.md's vector/geometry categories). Cross-checked
    // against an independent Python reference implementation (including a 2,000-case
    // random sweep against the true integer cross product) before transcription.
    fn i16_bits(v: i16) -> u64 {
        (v as u16) as u64
    }
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // geom_distance_3d: squared 3D Euclidean distance, the missing sibling of euclid_sq.
    let (_, cell) = step(
        "geom_distance_3d",
        "GeomDistance3d",
        &[
            ("ax", 0),
            ("ay", 0),
            ("az", 0),
            ("bx", i16_bits(3)),
            ("by", i16_bits(4)),
            ("bz", i16_bits(12)),
        ],
    );
    assert_eq!(cell.get("result"), Some(169)); // 3^2+4^2+12^2 = 9+16+144

    // Negative coordinates, both sides — confirms the excess-32768 shift handles signed
    // differences correctly, not just the a=0 case above.
    let (_, cell) = step(
        "geom_distance_3d",
        "GeomDistance3d",
        &[
            ("ax", i16_bits(-1)),
            ("ay", i16_bits(-2)),
            ("az", i16_bits(-3)),
            ("bx", i16_bits(2)),
            ("by", i16_bits(2)),
            ("bz", i16_bits(1)),
        ],
    );
    assert_eq!(cell.get("result"), Some(9 + 16 + 16)); // dx=3,dy=4,dz=4

    // Extreme coordinates: the summed squared distance overflows u32.
    let (report, _) = step(
        "geom_distance_3d",
        "GeomDistance3d",
        &[
            ("ax", i16_bits(i16::MIN)),
            ("ay", i16_bits(i16::MIN)),
            ("az", i16_bits(i16::MIN)),
            ("bx", i16_bits(i16::MAX)),
            ("by", i16_bits(i16::MAX)),
            ("bz", i16_bits(i16::MAX)),
        ],
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn orientation2d_matches_defined_behaviour() {
    // orientation2d: sign of the 2D cross product (x2-x1)*(y3-y1) - (y2-y1)*(x3-x1) for
    // three points p1->p2->p3 -- -1 clockwise, 0 collinear, 1 counter-clockwise. The four
    // difference terms are derived from raw i16 coordinates via a sign-magnitude subtract
    // (a plain i16 subtract could overflow i16's range, e.g. 32767 - (-32768)), then
    // combined as (magnitude, sign) products the same way matrix_det_2x2/cross_product do.
    fn i16_bits(v: i16) -> u64 {
        (v as u16) as u64
    }
    fn orientation(p1: (i16, i16), p2: (i16, i16), p3: (i16, i16)) -> i16 {
        let mut cell = StateCell::bind(&cell_src("orientation2d"), "Orientation2d", None)
            .unwrap_or_else(|e| panic!("bind orientation2d: {e}"));
        cell.set("x1", i16_bits(p1.0)).unwrap();
        cell.set("y1", i16_bits(p1.1)).unwrap();
        cell.set("x2", i16_bits(p2.0)).unwrap();
        cell.set("y2", i16_bits(p2.1)).unwrap();
        cell.set("x3", i16_bits(p3.0)).unwrap();
        cell.set("y3", i16_bits(p3.1)).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap();
        cell.get("sign").map(|b| b as u16 as i16).unwrap()
    }

    // (0,0)->(1,0)->(0,1): cross = 1*1 - 0*0 = 1 -> counter-clockwise.
    assert_eq!(orientation((0, 0), (1, 0), (0, 1)), 1);
    // (0,0)->(0,1)->(1,0): cross = 0*0 - 1*1 = -1 -> clockwise.
    assert_eq!(orientation((0, 0), (0, 1), (1, 0)), -1);
    // (0,0)->(1,1)->(2,2): cross = 1*2 - 1*2 = 0 -> collinear.
    assert_eq!(orientation((0, 0), (1, 1), (2, 2)), 0);
    // (-5,-5)->(5,-5)->(5,5): dx1=10,dy1=10,dy2=0,dx2=10 -> cross = 100 -> ccw.
    assert_eq!(orientation((-5, -5), (5, -5), (5, 5)), 1);
    // Extreme magnitudes stressing the sign-magnitude coordinate-difference subtract:
    // (-32768,0)->(32767,0)->(0,100): dx1 = 65535, dy1 = 100, dy2 = 0, dx2 = 32768
    // -> cross = 6553500 -> ccw.
    assert_eq!(orientation((-32768, 0), (32767, 0), (0, 100)), 1);
}

#[test]
fn segments_intersect_int_matches_defined_behaviour() {
    // segments_intersect_int: the standard four-orientation-sign-test segment-intersection
    // predicate (including the collinear-overlap edge case). Hand-verified against the
    // orientation math directly before shipping.
    fn i16_bits(v: i16) -> u64 {
        (v as u16) as u64
    }
    // Eight coordinates is the cell's own state shape (two segments) — the
    // helper mirrors it one-to-one.
    #[allow(clippy::too_many_arguments)]
    fn seg(x1: i16, y1: i16, x2: i16, y2: i16, x3: i16, y3: i16, x4: i16, y4: i16) -> u16 {
        let mut cell = StateCell::bind(
            &cell_src("segments_intersect_int"),
            "SegmentsIntersect",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("x1", i16_bits(x1)).unwrap();
        cell.set("y1", i16_bits(y1)).unwrap();
        cell.set("x2", i16_bits(x2)).unwrap();
        cell.set("y2", i16_bits(y2)).unwrap();
        cell.set("x3", i16_bits(x3)).unwrap();
        cell.set("y3", i16_bits(y3)).unwrap();
        cell.set("x4", i16_bits(x4)).unwrap();
        cell.set("y4", i16_bits(y4)).unwrap();
        let report = cell
            .run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "unexpected halt: {:?}",
            report.halt
        );
        cell.get("result").unwrap() as u16
    }

    // 1) Classic X-crossing: (0,0)-(4,4) and (0,4)-(4,0) cross at (2,2). d1=-16, d2=+16
    //    (opposite, nonzero); d3=+16, d4=-16 (opposite, nonzero) -> proper hit.
    assert_eq!(seg(0, 0, 4, 4, 0, 4, 4, 0), 1);

    // 2) Parallel, non-touching: (0,0)-(1,0) and (0,1)-(1,1). d1=d2=-1 (same sign, no
    //    orientation flip), no orientation is zero -> no intersection.
    assert_eq!(seg(0, 0, 1, 0, 0, 1, 1, 1), 0);

    // 3) Collinear overlap: (0,0)-(4,0) and (2,0)-(6,0) share [2,4] on the x-axis. d1=d2=0
    //    and P2=(4,0) falls inside [2,6]x[0,0] -> the collinear on-segment branch fires.
    assert_eq!(seg(0, 0, 4, 0, 2, 0, 6, 0), 1);

    // 4) Collinear, disjoint: (0,0)-(2,0) and (3,0)-(5,0), same line, no overlap. All four
    //    orientations are zero but no point falls in the opposite segment's bounding box.
    assert_eq!(seg(0, 0, 2, 0, 3, 0, 5, 0), 0);

    // 5) Shared endpoint (T-touch): (0,0)-(2,2) and (2,2)-(4,0); P2 == P3 = (2,2), so
    //    d2 = 0 and P2 trivially lies within P3P4's own bounding box -> touching at an
    //    endpoint counts as intersecting.
    assert_eq!(seg(0, 0, 2, 2, 2, 2, 4, 0), 1);
}

#[test]
fn slope_fraction_two_point_slope_matches_defined_behaviour() {
    // slope_fraction: exact (y2-y1)/(x2-x1) between two points as a sign-magnitude
    // fraction (num_mag, num_neg) over a positive den -- the two-point sibling of
    // linear_regression_slope's aggregated-sums fit. Neither reduces to lowest terms.
    fn i16_bits(v: i16) -> u64 {
        (v as u16) as u64
    }
    fn step(fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src("slope_fraction"), "SlopeFraction", None)
            .unwrap_or_else(|e| panic!("bind slope_fraction: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // (0,0)->(4,8): slope = 8/4, positive.
    let (_, cell) = step(&[
        ("x1", i16_bits(0)),
        ("y1", i16_bits(0)),
        ("x2", i16_bits(4)),
        ("y2", i16_bits(8)),
    ]);
    assert_eq!(cell.get("num_mag"), Some(8));
    assert_eq!(cell.get("num_neg"), Some(0));
    assert_eq!(cell.get("den"), Some(4));

    // (0,0)->(4,-8): slope = -8/4, negative numerator, positive den.
    let (_, cell) = step(&[
        ("x1", i16_bits(0)),
        ("y1", i16_bits(0)),
        ("x2", i16_bits(4)),
        ("y2", i16_bits(-8)),
    ]);
    assert_eq!(cell.get("num_mag"), Some(8));
    assert_eq!(cell.get("num_neg"), Some(1));
    assert_eq!(cell.get("den"), Some(4));

    // (5,3)->(2,3): horizontal line with a negative dx; slope 0, sign forced to 0.
    let (_, cell) = step(&[
        ("x1", i16_bits(5)),
        ("y1", i16_bits(3)),
        ("x2", i16_bits(2)),
        ("y2", i16_bits(3)),
    ]);
    assert_eq!(cell.get("num_mag"), Some(0));
    assert_eq!(cell.get("num_neg"), Some(0));
    assert_eq!(cell.get("den"), Some(3));

    // (2,2)->(-3,-3): both dx and dy negative -> positive slope (5/5, unreduced).
    let (_, cell) = step(&[
        ("x1", i16_bits(2)),
        ("y1", i16_bits(2)),
        ("x2", i16_bits(-3)),
        ("y2", i16_bits(-3)),
    ]);
    assert_eq!(cell.get("num_mag"), Some(5));
    assert_eq!(cell.get("num_neg"), Some(0));
    assert_eq!(cell.get("den"), Some(5));

    // Vertical line (x1 == x2): escalates, undefined slope.
    let (report, _) = step(&[
        ("x1", i16_bits(7)),
        ("y1", i16_bits(1)),
        ("x2", i16_bits(7)),
        ("y2", i16_bits(99)),
    ]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
}
