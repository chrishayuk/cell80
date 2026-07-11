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

#[test]
fn triangle_area_x4_approx_hand_verified() {
    // triangle_area_x4_approx: floor(4*Area) of a triangle with integer sides (a, b, c) --
    // inlines heron_16a2's own valid-triangle check + 16*Area^2 formula, then extracts a
    // real magnitude via isqrt_u32's branch-free bitwise sqrt loop, since
    // isqrt(16*Area^2) = floor(4*Area). Hand-verified against exact integer Heron
    // arithmetic (not floating-point) before shipping.
    fn step(a: u64, b: u64, c: u64) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("triangle_area_x4_approx"),
            "TriangleAreaX4Approx",
            None,
        )
        .unwrap_or_else(|e| panic!("bind triangle_area_x4_approx: {e}"));
        cell.set("a", a).unwrap();
        cell.set("b", b).unwrap();
        cell.set("c", c).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // 3-4-5 right triangle: area = 6, floor(4*6) = 24. 16*Area^2 = 576 = 24^2 exactly.
    let (report, cell) = step(3, 4, 5);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("area_x4"), Some(24));
    assert_eq!(report.result, 24);

    // Equilateral side 2: 16*Area^2 = 48 (matches heron_16a2's own test case). isqrt(48) = 6
    // (6^2=36 <= 48 < 49=7^2), i.e. floor(4*sqrt(3)) = floor(6.928..) = 6.
    let (report, cell) = step(2, 2, 2);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("area_x4"), Some(6));
    assert_eq!(report.result, 6);

    // Equilateral side 10: s1=30, s2=s3=s4=10 -> 16*Area^2 = 300*100 = 30000.
    // isqrt(30000) = 173 (173^2=29929 <= 30000 < 30276=174^2).
    let (report, cell) = step(10, 10, 10);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("area_x4"), Some(173));
    assert_eq!(report.result, 173);

    // 5-12-13 right triangle: area = 30, floor(4*30) = 120. 16*Area^2 = 14400 = 120^2 exactly.
    let (report, cell) = step(5, 12, 13);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("area_x4"), Some(120));
    assert_eq!(report.result, 120);

    // Not a triangle (1 + 1 <= 5): out_of_domain, same convention as heron_16a2.
    let (report, _) = step(1, 1, 5);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // Large equilateral triangle: the final factor-pair product overflows u32 (the same
    // inputs heron_16a2's own overflow test uses).
    let (report, _) = step(30000, 30000, 30000);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn geom_circle_area_approx_matches_defined_behaviour() {
    // geom_circle_area_approx: Floor(pi*r^2) via Q8.8 fixed pi (804/256 = 3.140625).
    // area = (r*r * 804) >> 8, both multiplies checked -- escalates (0xFF05) once
    // r*r*804 would overflow u32 (r*r itself never overflows u32 for any u16 r,
    // since 65535^2 fits comfortably under u32::MAX).
    fn step(r: u64) -> (cell80::Report, cell80::StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("geom_circle_area_approx"),
            "GeomCircleAreaApprox",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("r", r).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // r=1 -> 1*804 = 804, 804>>8 = 3 (pi*1 = 3.14159...)
    let (_, cell) = step(1);
    assert_eq!(cell.get("area"), Some(3));
    // r=5 -> 25*804 = 20100, 20100>>8 = 78 (pi*25 = 78.5398...)
    let (_, cell) = step(5);
    assert_eq!(cell.get("area"), Some(78));
    // r=10 -> 100*804 = 80400, 80400>>8 = 314 (pi*100 = 314.159...)
    let (_, cell) = step(10);
    assert_eq!(cell.get("area"), Some(314));
    // r=2312 -> r*r = 5,345,344 (fits u32), *804 = 4,297,656,576 > u32::MAX
    // -> escalates (halt 0xFF05, needs_wider_math)
    let (report, _) = step(2312);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn geom_circle_circumference_approx_matches_hand_computed_values() {
    // geom_circle_circumference_approx: floor(2*pi*r) via the pack's Q8.8 fixed-point pi
    // constant (804/256), computed as (r*1608)>>8 -- the non-squaring sibling of
    // geom_circle_area_approx's r^2 formula. Small r values line up closely with the true
    // circumference (slightly under, since 804/256 = 3.140625 is a hair below real pi);
    // r >= 10434 pushes the shifted result past u16::MAX and must escalate, not truncate.
    assert_eq!(run_cell("geom_circle_circumference_approx", &[0]), 0);
    assert_eq!(run_cell("geom_circle_circumference_approx", &[1]), 6); // 1608 >> 8 = 6
    assert_eq!(run_cell("geom_circle_circumference_approx", &[10]), 62); // 16080 >> 8 = 62
    assert_eq!(run_cell("geom_circle_circumference_approx", &[100]), 628); // 160800 >> 8 = 628
    assert_eq!(
        run_cell("geom_circle_circumference_approx", &[10433]),
        65532
    ); // last radius before overflow

    // r = 10434 pushes (r*1608)>>8 to 65538, past u16::MAX -- must escalate, not wrap.
    let mut r = cell80::Runner::compile(&cell_src("geom_circle_circumference_approx")).unwrap();
    let report = r.run(None, &[10434], DEFAULT_CYCLES).unwrap();
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn shoelace_area_x2_i16_matches_hand_computed_values() {
    // shoelace_area_x2_i16: twice a triangle's area over the full signed i16 plane (the
    // signed sibling of shoelace_area_x2, which is unsigned-only). Every term is a signed
    // coordinate times a signed y-difference, combined via sign-magnitude tracking.
    fn i16_bits(v: i16) -> u64 {
        (v as u16) as u64
    }
    fn area_x2(x1: i16, y1: i16, x2: i16, y2: i16, x3: i16, y3: i16) -> u64 {
        let mut cell =
            StateCell::bind(&cell_src("shoelace_area_x2_i16"), "ShoelaceAreaX2I16", None)
                .unwrap_or_else(|e| panic!("bind: {e}"));
        cell.set("x1", i16_bits(x1)).unwrap();
        cell.set("y1", i16_bits(y1)).unwrap();
        cell.set("x2", i16_bits(x2)).unwrap();
        cell.set("y2", i16_bits(y2)).unwrap();
        cell.set("x3", i16_bits(x3)).unwrap();
        cell.set("y3", i16_bits(y3)).unwrap();
        let report = cell
            .run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "unexpected halt: {:?}",
            report.halt
        );
        cell.get("result").unwrap()
    }

    // 1) Right triangle, all-nonnegative coords (parity check against shoelace_area_x2's
    //    own unsigned test): legs 4 and 3, area 6, x2 = 12.
    assert_eq!(area_x2(0, 0, 4, 0, 0, 3), 12);
    // 2) Same triangle, reversed winding order: |.| is winding-independent, still 12.
    assert_eq!(area_x2(0, 0, 0, 3, 4, 0), 12);
    // 3) Degenerate triangle (all three vertices coincide): area 0.
    assert_eq!(area_x2(1, 1, 1, 1, 1, 1), 0);
    // 4) Straddling all four quadrant signs: (-2,-2),(2,-2),(2,2) is a right triangle
    //    with legs 4 and 4 -> area 8, x2 = 16.
    assert_eq!(area_x2(-2, -2, 2, -2, 2, 2), 16);
    // 5) General negative/positive mix, hand-solved via the raw shoelace sum:
    //    x1*(y2-y3) + x2*(y3-y1) + x3*(y1-y2) = -5*(-4-6) + 2*(6-3) + 6*(3-(-4))
    //    = 50 + 6 + 42 = 98.
    assert_eq!(area_x2(-5, 3, 2, -4, 6, 6), 98);
    // 6) Extreme-magnitude stress test (i16::MIN/MAX corners), still within u32 -- no
    //    overflow halt expected. y2-y3=65535, y3-y1=0, y1-y2=-65535, so:
    //    term1 = 32767*65535 = 2_147_385_345, term2 = 0,
    //    term3 = (-32768)*(-65535) = 32768*65535 = 2_147_450_880,
    //    sum = 4_294_836_225 (< u32::MAX = 4_294_967_295).
    assert_eq!(
        area_x2(32767, -32768, 32767, 32767, -32768, -32768),
        4_294_836_225
    );
}

#[test]
fn shoelace_area_x2_quad_i16_matches_defined_behaviour() {
    // shoelace_area_x2_quad_i16: same shoelace formula as shoelace_area_x2_quad but over
    // signed i16 vertices, tracked internally as sign-magnitude pairs. Hand-computed cases
    // below cover an all-positive square, a square straddling the origin (negative coords
    // on both axes), a degenerate (coincident-vertex) quad, a rectangle spanning the sign
    // boundary, and a magnitude-overflow escalation at the extremes of i16's range.
    fn i16_bits(v: i16) -> u64 {
        (v as u16) as u64
    }
    #[allow(clippy::too_many_arguments)]
    fn shoelace(
        x1: i16,
        y1: i16,
        x2: i16,
        y2: i16,
        x3: i16,
        y3: i16,
        x4: i16,
        y4: i16,
    ) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("shoelace_area_x2_quad_i16"),
            "ShoelaceAreaX2QuadI16",
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
        (report, cell)
    }

    // 2x2 square at the origin, all-positive coords -> area 4, x2 = 8.
    let (report, cell) = shoelace(0, 0, 2, 0, 2, 2, 0, 2);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(8));

    // 4x4 square straddling the origin (-2,-2)..(2,2) -> area 16, x2 = 32.
    let (report, cell) = shoelace(-2, -2, 2, -2, 2, 2, -2, 2);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(32));

    // Degenerate quadrilateral, all four vertices coincide at a negative point -> 0.
    let (report, cell) = shoelace(-5, -5, -5, -5, -5, -5, -5, -5);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(0));

    // 7x3 rectangle spanning the sign boundary on both axes -> area 21, x2 = 42.
    let (report, cell) = shoelace(-3, 1, 4, 1, 4, -2, -3, -2);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("result"), Some(42));

    // Extreme i16 coordinates: the sign-magnitude running sum overflows u32 -> escalate.
    let (report, _cell) = shoelace(-32768, -32768, -32768, 32767, 32767, 32767, 0, -32768);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn point_line_dist_sq_matches_defined_behaviour() {
    // point_line_dist_sq: exact squared perpendicular distance from a point (px,py) to
    // the infinite line through (x1,y1)-(x2,y2), returned as an unreduced fraction
    // num/den = cross^2 / (dx^2+dy^2). Hand-verified via the standard point-line distance
    // formula (perp_dist = |cross| / |segment|, so perp_dist^2 = cross^2 / segment_len_sq)
    // before shipping.
    fn i16_bits(v: i16) -> u64 {
        (v as u16) as u64
    }
    fn step(fields: &[(&str, i16)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(&cell_src("point_line_dist_sq"), "PointLineDistSq", None)
            .unwrap_or_else(|e| panic!("bind point_line_dist_sq: {e}"));
        for (f, v) in fields {
            cell.set(f, i16_bits(*v)).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // Line is the x-axis (0,0)-(4,0), point (2,3): perpendicular distance 3, squared 9 =
    // 144/16 (unreduced: cross = 4*3 - 0*2 = 12, 12^2 = 144; segment len^2 = 4^2 = 16).
    let (report, cell) = step(&[
        ("x1", 0),
        ("y1", 0),
        ("x2", 4),
        ("y2", 0),
        ("px", 2),
        ("py", 3),
    ]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("num"), Some(144));
    assert_eq!(cell.get("den"), Some(16));

    // Line is the y-axis (0,0)-(0,5), point (3,2): perpendicular distance 3, squared 9 =
    // 225/25.
    let (report, cell) = step(&[
        ("x1", 0),
        ("y1", 0),
        ("x2", 0),
        ("y2", 5),
        ("px", 3),
        ("py", 2),
    ]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("num"), Some(225));
    assert_eq!(cell.get("den"), Some(25));

    // Point exactly on the line (0,0)-(4,4): distance 0, so num is 0 even though den (32)
    // is not.
    let (report, cell) = step(&[
        ("x1", 0),
        ("y1", 0),
        ("x2", 4),
        ("y2", 4),
        ("px", 2),
        ("py", 2),
    ]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("num"), Some(0));
    assert_eq!(cell.get("den"), Some(32));

    // Negative coordinates: horizontal line y=-5 from (-5,-5) to (5,-5), point (0,0).
    // Distance 5, squared 25 = 2500/100.
    let (report, cell) = step(&[
        ("x1", -5),
        ("y1", -5),
        ("x2", 5),
        ("y2", -5),
        ("px", 0),
        ("py", 0),
    ]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("num"), Some(2500));
    assert_eq!(cell.get("den"), Some(100));

    // The two line-defining points coincide (7,7)-(7,7): den would be 0, the line is
    // undefined -- must escalate 0xFF06 (out_of_domain) rather than return a bogus fraction.
    let (report, _) = step(&[
        ("x1", 7),
        ("y1", 7),
        ("x2", 7),
        ("y2", 7),
        ("px", 10),
        ("py", 10),
    ]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
}

#[test]
fn triangle_inradius_x2_approx_hand_verified() {
    // triangle_inradius_x2_approx: twice a triangle's inradius, floor(floor(4*Area)/(a+b+c)) --
    // reuses heron_16a2's exact 16*Area^2 rearrangement and triangle_area_x4_approx's own
    // inline isqrt loop to recover floor(4*Area), then divides by the perimeter since
    // 2r = 4*Area/(a+b+c). The pack's first triangle metric that is a length (not an area).
    // Hand-verified against exact integer Heron arithmetic before shipping.
    fn step(a: u64, b: u64, c: u64) -> cell80::Report {
        let mut r = cell80::Runner::compile(&cell_src("triangle_inradius_x2_approx"))
            .unwrap_or_else(|e| panic!("compile: {e}"));
        r.run(None, &[a as u16, b as u16, c as u16], DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run: {e}"))
    }

    // 3-4-5 right triangle: Area=6, 4*Area=24, perimeter=12, 2r=24/12=2.
    let report = step(3, 4, 5);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(report.result, 2);

    // 5-5-6 isosceles: 16*Area^2 = 16*6*6*4 = 2304, isqrt=48=4*Area, perimeter=16, 2r=48/16=3.
    let report = step(5, 5, 6);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(report.result, 3);

    // 6-8-10 (2x 3-4-5 scaled): Area=24, 4*Area=96, perimeter=24, 2r=96/24=4.
    let report = step(6, 8, 10);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(report.result, 4);

    // 1-1-1 equilateral: 16*Area^2=3, isqrt(3)=1=floor(4*Area), perimeter=3, floor(1/3)=0
    // (the floor of a genuinely fractional 2r, not a rounding bug).
    let report = step(1, 1, 1);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(report.result, 0);

    // Not a triangle (1+1<=5): out_of_domain, same convention as heron_16a2.
    let report = step(1, 1, 5);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // Large equilateral triangle: the same overflow inputs heron_16a2/triangle_area_x4_approx
    // already document -> needs_wider_math.
    let report = step(30000, 30000, 30000);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn point_segment_dist_sq_matches_hand_computed() {
    // Exact squared distance from a point to the closest point on a FINITE segment,
    // clamping to an endpoint (den=1) when the perpendicular foot falls outside
    // [0,1] along the segment, otherwise falling back to point_line_dist_sq's own
    // cross^2/den fraction. All five expected values are hand-derived below.
    fn i16_bits(v: i16) -> u64 {
        (v as u16) as u64
    }
    fn step(fields: &[(&str, i16)]) -> StateCell {
        let mut cell = StateCell::bind(
            &cell_src("point_segment_dist_sq"),
            "PointSegmentDistSq",
            None,
        )
        .unwrap_or_else(|e| panic!("bind point_segment_dist_sq: {e}"));
        for (f, v) in fields {
            cell.set(f, i16_bits(*v)).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        assert_eq!(report.halt, cell80::Halt::Returned);
        cell
    }

    // 1) Segment (0,0)-(4,0), point (2,3): foot of perpendicular at (2,0), inside the
    // segment. cross = 4*3 - 0*2 = 12, 12^2 = 144; t_den = 4^2 = 16 -- matches
    // point_line_dist_sq's own num/den for the same inputs (144/16 = 9).
    let cell = step(&[
        ("x1", 0),
        ("y1", 0),
        ("x2", 4),
        ("y2", 0),
        ("px", 2),
        ("py", 3),
    ]);
    assert_eq!(cell.get("num"), Some(144));
    assert_eq!(cell.get("den"), Some(16));

    // 2) Same segment, point (-2,3): foot falls before A (t_num = (-2)*4+3*0 = -8 < 0)
    // -> clamp to (0,0). Squared distance = (-2)^2+3^2 = 13, den forced to 1.
    let cell = step(&[
        ("x1", 0),
        ("y1", 0),
        ("x2", 4),
        ("y2", 0),
        ("px", -2),
        ("py", 3),
    ]);
    assert_eq!(cell.get("num"), Some(13));
    assert_eq!(cell.get("den"), Some(1));

    // 3) Same segment, point (6,3): foot falls beyond B (t_num = 6*4+3*0 = 24 >=
    // t_den=16) -> clamp to (4,0). Squared distance = (6-4)^2+3^2 = 13, den forced to 1.
    let cell = step(&[
        ("x1", 0),
        ("y1", 0),
        ("x2", 4),
        ("y2", 0),
        ("px", 6),
        ("py", 3),
    ]);
    assert_eq!(cell.get("num"), Some(13));
    assert_eq!(cell.get("den"), Some(1));

    // 4) Degenerate segment (5,5)-(5,5), point (8,9): t_den = 0, no halt -- returns the
    // exact squared distance to the single point (5,5): (8-5)^2+(9-5)^2 = 9+16 = 25.
    let cell = step(&[
        ("x1", 5),
        ("y1", 5),
        ("x2", 5),
        ("y2", 5),
        ("px", 8),
        ("py", 9),
    ]);
    assert_eq!(cell.get("num"), Some(25));
    assert_eq!(cell.get("den"), Some(1));

    // 5) Segment (-4,-2)-(4,2) (dx=8,dy=4,t_den=80), point (-2,4): t_num =
    // (-2-(-4))*8 + (4-(-2))*4 = 2*8+6*4 = 40, strictly between 0 and 80 -> the foot
    // is inside the segment. cross = dx*dpy - dy*dpx = 8*6 - 4*2 = 40, num = 1600,
    // den = 80 (1600/80 = 20, matching the Euclidean check: foot = (0,0), squared
    // distance from (-2,4) is (-2)^2+4^2 = 20).
    let cell = step(&[
        ("x1", -4),
        ("y1", -2),
        ("x2", 4),
        ("y2", 2),
        ("px", -2),
        ("py", 4),
    ]);
    assert_eq!(cell.get("num"), Some(1600));
    assert_eq!(cell.get("den"), Some(80));
}

#[test]
fn line_intersect_params_frac_matches_defined_behaviour() {
    // line_intersect_params_frac: exact parametric-fraction intersection of the two infinite
    // lines through (x1,y1)-(x2,y2) and (x3,y3)-(x4,y4): t=t_num/den is how far along line 1
    // (P1+t*(P2-P1)), u=u_num/den is how far along line 2, sharing den=cross(d1,d2). Both
    // numerators and the denominator are left unreduced (no gcd), only sign-normalized so
    // den is always positive. Hand-verified against the standard direction-vector formula
    // (t=cross(w,d2)/cross(d1,d2), u=cross(w,d1)/cross(d1,d2), w=P3-P1) before shipping.
    fn i16_bits(v: i16) -> u64 {
        (v as u16) as u64
    }
    fn step(fields: &[(&str, i16)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("line_intersect_params_frac"),
            "LineIntersectParamsFrac",
            None,
        )
        .unwrap_or_else(|e| panic!("bind line_intersect_params_frac: {e}"));
        for (f, v) in fields {
            cell.set(f, i16_bits(*v)).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // Two diagonals (0,0)-(4,4) and (0,4)-(4,0) cross at (2,2): den = 4*-4 - 4*4 = -32 (negative,
    // so both numerators get sign-flipped to normalize den positive); t_num = u_num = 16, den = 32
    // (t = u = 1/2, matching the midpoint (2,2)).
    let (report, cell) = step(&[
        ("x1", 0),
        ("y1", 0),
        ("x2", 4),
        ("y2", 4),
        ("x3", 0),
        ("y3", 4),
        ("x4", 4),
        ("y4", 0),
    ]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("t_num_mag"), Some(16));
    assert_eq!(cell.get("t_num_neg"), Some(0));
    assert_eq!(cell.get("u_num_mag"), Some(16));
    assert_eq!(cell.get("u_num_neg"), Some(0));
    assert_eq!(cell.get("den"), Some(32));

    // Parallel lines (0,0)-(2,0) and (0,1)-(2,1) share direction (2,0): den = 2*0 - 0*2 = 0,
    // so the cell escalates rather than divide by a zero denominator.
    let (report, _cell) = step(&[
        ("x1", 0),
        ("y1", 0),
        ("x2", 2),
        ("y2", 0),
        ("x3", 0),
        ("y3", 1),
        ("x4", 2),
        ("y4", 1),
    ]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));

    // Negative coordinates and a non-half fraction: (1,1)-(5,3) and (1,5)-(5,1) cross at
    // (11/3, 7/3). den = 4*-4 - 2*4 = -24 (negative, flip); t_num = u_num = 16, den = 24
    // (t = u = 2/3).
    let (report, cell) = step(&[
        ("x1", 1),
        ("y1", 1),
        ("x2", 5),
        ("y2", 3),
        ("x3", 1),
        ("y3", 5),
        ("x4", 5),
        ("y4", 1),
    ]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("t_num_mag"), Some(16));
    assert_eq!(cell.get("t_num_neg"), Some(0));
    assert_eq!(cell.get("u_num_mag"), Some(16));
    assert_eq!(cell.get("u_num_neg"), Some(0));
    assert_eq!(cell.get("den"), Some(24));

    // Horizontal line (0,0)-(4,0) crossed by vertical (2,-2)-(2,2) at (2,0): den = 4*4 - 0*0 =
    // 16 is already positive (no flip), t_num = u_num = 8 (t = u = 1/2).
    let (report, cell) = step(&[
        ("x1", 0),
        ("y1", 0),
        ("x2", 4),
        ("y2", 0),
        ("x3", 2),
        ("y3", -2),
        ("x4", 2),
        ("y4", 2),
    ]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("t_num_mag"), Some(8));
    assert_eq!(cell.get("t_num_neg"), Some(0));
    assert_eq!(cell.get("u_num_mag"), Some(8));
    assert_eq!(cell.get("u_num_neg"), Some(0));
    assert_eq!(cell.get("den"), Some(16));

    // Same horizontal line (0,0)-(4,0), but the second line (2,2)-(2,6) points "away" from the
    // crossing: u = -1/2 is a legitimate negative parameter on the *infinite* line (the crossing
    // point (2,0) is still exact), so u_num_neg must come back 1 while t_num stays positive.
    let (report, cell) = step(&[
        ("x1", 0),
        ("y1", 0),
        ("x2", 4),
        ("y2", 0),
        ("x3", 2),
        ("y3", 2),
        ("x4", 2),
        ("y4", 6),
    ]);
    assert_eq!(report.halt, cell80::Halt::Returned);
    assert_eq!(cell.get("t_num_mag"), Some(8));
    assert_eq!(cell.get("t_num_neg"), Some(0));
    assert_eq!(cell.get("u_num_mag"), Some(8));
    assert_eq!(cell.get("u_num_neg"), Some(1));
    assert_eq!(cell.get("den"), Some(16));
}

// quad_is_valid: 1 if four side lengths (a,b,c,d) satisfy the quadrilateral
// inequality (each side strictly less than the sum of the other three), else 0.
// The polygon-inequality generalization of triangle_is_valid's three-side check.
#[test]
fn geometry_pack_quad_is_valid() {
    fn quad_is_valid(a: u16, b: u16, c: u16, d: u16) -> u16 {
        let mut cell = StateCell::bind(&cell_src("quad_is_valid"), "QuadIsValid", None)
            .unwrap_or_else(|e| panic!("bind quad_is_valid: {e}"));
        cell.set("a", a as u64).unwrap();
        cell.set("b", b as u64).unwrap();
        cell.set("c", c as u64).unwrap();
        cell.set("d", d as u64).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap();
        cell.get("valid").unwrap() as u16
    }

    // A normal valid quadrilateral: 3<15, 4<14, 5<13, 6<12 all hold.
    assert_eq!(quad_is_valid(3, 4, 5, 6), 1);
    // One side dominates the rest (10 vs 1+1+1=3) -- cannot close -> invalid.
    assert_eq!(quad_is_valid(1, 1, 1, 10), 0);
    // Boundary: d exactly equals the sum of the others (3 == 1+1+1); the
    // inequality is strict, so equality must fail -> invalid.
    assert_eq!(quad_is_valid(1, 1, 1, 3), 0);
    // Square-like, all sides equal -> trivially valid.
    assert_eq!(quad_is_valid(5, 5, 5, 5), 1);
    // Large sides near u16's top end: any three summed (180000) overflow u16
    // (wraps to 48928) unless widened to u32 first. Widened correctly, all
    // four inequalities hold -> valid; a non-widened implementation would
    // wrongly report invalid here.
    assert_eq!(quad_is_valid(60000, 60000, 60000, 60000), 1);
}

#[test]
fn geom_distance_3d_exact_matches_defined_behaviour() {
    // geom_distance_3d_exact: true (rooted) 3D Euclidean distance -- the isqrt-closed
    // sibling of geom_distance_3d, which stays squared. Hand-computed expectations below,
    // not taken from the compiled cell's own output.
    fn i16_bits(v: i16) -> u64 {
        (v as u16) as u64
    }
    fn step(fields: &[(&str, u64)]) -> (cell80::Report, StateCell) {
        let mut cell = StateCell::bind(
            &cell_src("geom_distance_3d_exact"),
            "GeomDistance3dExact",
            None,
        )
        .unwrap_or_else(|e| panic!("bind geom_distance_3d_exact: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        (report, cell)
    }

    // (0,0,0) -> (3,4,0): a 3-4-5 triangle embedded in 3D (dz=0). 9+16+0=25, isqrt(25)=5.
    let (_, cell) = step(&[
        ("ax", 0),
        ("ay", 0),
        ("az", 0),
        ("bx", i16_bits(3)),
        ("by", i16_bits(4)),
        ("bz", 0),
    ]);
    assert_eq!(cell.get("dist"), Some(5));

    // Negative coordinates on the a side, same 3-4-5 magnitude -- confirms the
    // excess-32768 shift handles signed differences symmetrically.
    let (_, cell) = step(&[
        ("ax", i16_bits(-3)),
        ("ay", i16_bits(-4)),
        ("az", 0),
        ("bx", 0),
        ("by", 0),
        ("bz", 0),
    ]);
    assert_eq!(cell.get("dist"), Some(5));

    // Non-perfect-square case: dx=10,dy=20,dz=40 -> sum=2100; isqrt(2100)=45 since
    // 45^2=2025 <= 2100 < 2116=46^2 (floors, doesn't round).
    let (_, cell) = step(&[
        ("ax", 100),
        ("ay", 200),
        ("az", 300),
        ("bx", 110),
        ("by", 220),
        ("bz", 340),
    ]);
    assert_eq!(cell.get("dist"), Some(45));

    // Coincident points -> distance 0.
    let (_, cell) = step(&[
        ("ax", 0),
        ("ay", 0),
        ("az", 0),
        ("bx", 0),
        ("by", 0),
        ("bz", 0),
    ]);
    assert_eq!(cell.get("dist"), Some(0));

    // Extreme coordinates: the summed squared distance overflows u32 -- escalates
    // rather than silently wrapping (the same guard geom_distance_3d documents).
    let (report, _) = step(&[
        ("ax", i16_bits(i16::MIN)),
        ("ay", i16_bits(i16::MIN)),
        ("az", i16_bits(i16::MIN)),
        ("bx", i16_bits(i16::MAX)),
        ("by", i16_bits(i16::MAX)),
        ("bz", i16_bits(i16::MAX)),
    ]);
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}
