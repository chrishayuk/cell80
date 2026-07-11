//! Host-oracle tests for the vector pack (`cell80/cells/vector/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::{cell_src, run_cell};
use cell80::{StateCell, DEFAULT_CYCLES};

#[test]
fn vector_state_cells_match_defined_behaviour() {
    // dot2 (wave 3, pilot batch): a 4-field state cell purely for arg count (2 vectors),
    // not width — mirrors the manhattan/chebyshev shape.
    let mut cell = StateCell::bind(&cell_src("dot2"), "Dot2", None).unwrap();
    for (f, v) in [("ax", 3u64), ("ay", 4), ("bx", 2), ("by", 1)] {
        cell.set(f, v).unwrap();
    }
    assert_eq!(cell.run(DEFAULT_CYCLES).unwrap().result, 10); // 3*2 + 4*1
}

#[test]
fn first_wave_vector_cells_match_defined_behaviour() {
    let cases: &[(&str, &[u16], u16)] = &[("norm2_sq", &[3, 4], 25), ("norm2_sq", &[0, 0], 0)];

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
fn wave11_3d_vector_cells_match_defined_behaviour() {
    // Wave 11 (docs/math-server-map.md's vector category): cross_product and
    // vectors_parallel both track each signed component as a (magnitude, sign) pair
    // rather than forming a raw i16 arithmetic result, since the dialect has no
    // signed-32-bit width for an intermediate product. Cross-checked against an
    // independent Python reference implementation, including a 2,000-case random
    // sweep of cross_product against the true integer cross product, before
    // transcribing any test row here.
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
    fn cross(a: (i16, i16, i16), b: (i16, i16, i16)) -> (cell80::Report, StateCell) {
        step(
            "cross_product",
            "CrossProduct",
            &[
                ("ax", i16_bits(a.0)),
                ("ay", i16_bits(a.1)),
                ("az", i16_bits(a.2)),
                ("bx", i16_bits(b.0)),
                ("by", i16_bits(b.1)),
                ("bz", i16_bits(b.2)),
            ],
        )
    }

    // cross_product: unit basis vectors, i x j = k.
    let (_, cell) = cross((1, 0, 0), (0, 1, 0));
    assert_eq!(cell.get("rx_mag"), Some(0));
    assert_eq!(cell.get("ry_mag"), Some(0));
    assert_eq!(cell.get("rz_mag"), Some(1));
    assert_eq!(cell.get("rz_neg"), Some(0));

    // cross_product: known case (2,3,4) x (5,6,7) = (-3, 6, -3).
    let (_, cell) = cross((2, 3, 4), (5, 6, 7));
    assert_eq!((cell.get("rx_mag"), cell.get("rx_neg")), (Some(3), Some(1)));
    assert_eq!((cell.get("ry_mag"), cell.get("ry_neg")), (Some(6), Some(0)));
    assert_eq!((cell.get("rz_mag"), cell.get("rz_neg")), (Some(3), Some(1)));

    // vectors_parallel: same direction, anti-parallel (negative scalar), and a larger
    // positive scalar — all parallel. A non-parallel pair returns 0.
    fn parallel(a: (i16, i16, i16), b: (i16, i16, i16)) -> u16 {
        let (_, cell) = step(
            "vectors_parallel",
            "VectorsParallel",
            &[
                ("ax", i16_bits(a.0)),
                ("ay", i16_bits(a.1)),
                ("az", i16_bits(a.2)),
                ("bx", i16_bits(b.0)),
                ("by", i16_bits(b.1)),
                ("bz", i16_bits(b.2)),
            ],
        );
        cell.get("result").unwrap() as u16
    }
    assert_eq!(parallel((3, 4, 5), (6, 8, 10)), 1);
    assert_eq!(parallel((3, 4, 5), (-9, -12, -15)), 1);
    assert_eq!(parallel((3, 4, 5), (15, 20, 25)), 1);
    assert_eq!(parallel((1, 0, 0), (0, 1, 0)), 0);
    assert_eq!(parallel((0, 0, 0), (0, 0, 0)), 1); // the zero vector is trivially parallel
}

#[test]
fn wave12_triple_product_cells_match_defined_behaviour() {
    // Wave 12: the vector pack's deferred triple products (docs/math-server-map.md),
    // built on wave 11's sign-magnitude technique. Cross-checked against an independent
    // Python reference implementation, including a 2,000-case random sweep for each
    // cell against the true (non-sign-magnitude) formula, before transcribing any row.
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
    fn vec9_fields(
        a: (i16, i16, i16),
        b: (i16, i16, i16),
        c: (i16, i16, i16),
    ) -> Vec<(&'static str, u64)> {
        vec![
            ("ax", i16_bits(a.0)),
            ("ay", i16_bits(a.1)),
            ("az", i16_bits(a.2)),
            ("bx", i16_bits(b.0)),
            ("by", i16_bits(b.1)),
            ("bz", i16_bits(b.2)),
            ("cx", i16_bits(c.0)),
            ("cy", i16_bits(c.1)),
            ("cz", i16_bits(c.2)),
        ]
    }

    // triple_scalar_product: a . (b x c), the signed volume of the parallelepiped.
    let (_, cell) = step(
        "triple_scalar_product",
        "TripleScalarProduct",
        &vec9_fields((1, 0, 0), (0, 1, 0), (0, 0, 1)),
    );
    assert_eq!(
        (cell.get("result_mag"), cell.get("result_neg")),
        (Some(1), Some(0))
    ); // unit cube, right-handed

    let (_, cell) = step(
        "triple_scalar_product",
        "TripleScalarProduct",
        &vec9_fields((1, 2, 3), (4, 5, 6), (7, 8, 10)),
    );
    assert_eq!(
        (cell.get("result_mag"), cell.get("result_neg")),
        (Some(3), Some(1))
    );

    let (_, cell) = step(
        "triple_scalar_product",
        "TripleScalarProduct",
        &vec9_fields((3, -1, 2), (1, 4, -2), (-1, 1, 3)),
    );
    assert_eq!(
        (cell.get("result_mag"), cell.get("result_neg")),
        (Some(53), Some(0))
    );

    // Coplanar vectors: the scalar triple product vanishes.
    let (_, cell) = step(
        "triple_scalar_product",
        "TripleScalarProduct",
        &vec9_fields((1, 0, 0), (0, 1, 0), (1, 1, 0)),
    );
    assert_eq!(
        (cell.get("result_mag"), cell.get("result_neg")),
        (Some(0), Some(0))
    );

    // triple_vector_product: a x (b x c), via the BAC-CAB identity.
    let (_, cell) = step(
        "triple_vector_product",
        "TripleVectorProduct",
        &vec9_fields((1, 0, 0), (0, 1, 0), (0, 0, 1)),
    );
    assert_eq!(
        (cell.get("rx_mag"), cell.get("ry_mag"), cell.get("rz_mag")),
        (Some(0), Some(0), Some(0))
    ); // b x c is parallel to a here, so a x (b x c) vanishes

    let (_, cell) = step(
        "triple_vector_product",
        "TripleVectorProduct",
        &vec9_fields((1, 2, 3), (2, 0, 1), (1, 1, 1)),
    );
    assert_eq!((cell.get("rx_mag"), cell.get("rx_neg")), (Some(7), Some(0)));
    assert_eq!((cell.get("ry_mag"), cell.get("ry_neg")), (Some(5), Some(1)));
    assert_eq!((cell.get("rz_mag"), cell.get("rz_neg")), (Some(1), Some(0)));

    // Overflow: triple_vector_product's scaling step (a dot product times a vector
    // component) can overflow for inputs well within i16's own range.
    let (report, _) = step(
        "triple_vector_product",
        "TripleVectorProduct",
        &vec9_fields(
            (30000, 30000, 30000),
            (30000, 30000, 30000),
            (30000, 30000, 30000),
        ),
    );
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn dot3_hand_computed_cases() {
    // Dot product of two signed 3D vectors, tracked as (dot_mag, dot_neg). Cross-checked
    // by hand: sum of the three pairwise products ax*bx + ay*by + az*bz.
    fn i16_bits(v: i16) -> u64 {
        (v as u16) as u64
    }
    fn dot3(a: (i16, i16, i16), b: (i16, i16, i16)) -> (u64, u64) {
        let mut cell = StateCell::bind(&cell_src("dot3"), "Dot3", None).unwrap();
        for (f, v) in [
            ("ax", i16_bits(a.0)),
            ("ay", i16_bits(a.1)),
            ("az", i16_bits(a.2)),
            ("bx", i16_bits(b.0)),
            ("by", i16_bits(b.1)),
            ("bz", i16_bits(b.2)),
        ] {
            cell.set(f, v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap();
        (cell.get("dot_mag").unwrap(), cell.get("dot_neg").unwrap())
    }

    // 1*4 + 2*5 + 3*6 = 32, positive.
    assert_eq!(dot3((1, 2, 3), (4, 5, 6)), (32, 0));
    // 3*1 + (-1)*4 + 2*(-2) = 3 - 4 - 4 = -5, negative.
    assert_eq!(dot3((3, -1, 2), (1, 4, -2)), (5, 1));
    // zero vector dotted with anything is 0, must not carry a spurious sign bit.
    assert_eq!(dot3((0, 0, 0), (5, -3, 7)), (0, 0));
    // (-2)*(-5) + (-3)*(-6) + (-4)*(-7) = 10 + 18 + 28 = 56, positive.
    assert_eq!(dot3((-2, -3, -4), (-5, -6, -7)), (56, 0));
    // 7*(-7) + (-8)*8 + 9*(-9) = -49 - 64 - 81 = -194, negative.
    assert_eq!(dot3((7, -8, 9), (-7, 8, -9)), (194, 1));
    // orthogonal unit basis vectors: dot is exactly zero, sign must be 0 not spuriously 1.
    assert_eq!(dot3((1, 0, 0), (0, 1, 0)), (0, 0));
}

#[test]
fn norm3_sq_matches_hand_computed_cases() {
    // norm3_sq: signed 3D squared magnitude, widened to u32 (u32 forces a state cell
    // even though there are only 3 inputs). Since every term is a square, only the
    // magnitude branch of the sign-magnitude pattern is ever exercised -- no
    // sign-combining step is needed, unlike cross_product/triple_scalar_product.
    fn i16_bits(v: i16) -> u64 {
        (v as u16) as u64
    }
    fn run(x: i16, y: i16, z: i16) -> u64 {
        let mut cell = StateCell::bind(&cell_src("norm3_sq"), "Norm3Sq", None).unwrap();
        cell.set("x", i16_bits(x)).unwrap();
        cell.set("y", i16_bits(y)).unwrap();
        cell.set("z", i16_bits(z)).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap();
        cell.get("mag_sq").unwrap()
    }

    // (3,4,0): 3*3 + 4*4 + 0*0 = 9 + 16 + 0 = 25
    assert_eq!(run(3, 4, 0), 25);
    // (-3,-4,12): 9 + 16 + 144 = 169, both negative components square positive
    assert_eq!(run(-3, -4, 12), 169);
    // (0,0,0): the zero vector
    assert_eq!(run(0, 0, 0), 0);
    // (-5,12,0): 25 + 144 + 0 = 169, mixed sign
    assert_eq!(run(-5, 12, 0), 169);
    // (-32768,0,0): i16::MIN alone -- magnitude 32768, squared = 1,073,741,824
    assert_eq!(run(-32768, 0, 0), 1_073_741_824);
    // (32767,32767,32767): i16::MAX in all three lanes -- 3 * 32767*32767 =
    // 3,221,028,867, comfortably inside u32 range, confirming the wide sum never
    // wraps or spuriously halts for any legal i16 input.
    assert_eq!(run(32767, 32767, 32767), 3_221_028_867);
}

#[test]
fn cosine_score_approx_matches_hand_computed_expectations() {
    // The long-blocked vector-pack candidate, closed once isqrt_u32 existed: norm_a and
    // norm_b are each at most u16::MAX, so their u32 product always fits u32 (65535*65535
    // = 4,294,836,225 < u32::MAX), sidestepping the sqrt-of-a-product overflow this cell
    // was parked behind for many checkpoints.
    fn score(ax: u16, ay: u16, bx: u16, by: u16) -> u16 {
        let mut cell =
            StateCell::bind(&cell_src("cosine_score_approx"), "CosineScoreApprox", None).unwrap();
        cell.set("ax", ax as u64).unwrap();
        cell.set("ay", ay as u64).unwrap();
        cell.set("bx", bx as u64).unwrap();
        cell.set("by", by as u64).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // (3,4) vs (4,3): dot=24, norm_a=norm_b=25, cos = 24/25 = 0.96 -> 245/256 (floor).
    assert_eq!(score(3, 4, 4, 3), 245);
    // Parallel vectors: cosine exactly 1.0 -> Q8.8 256.
    assert_eq!(score(1, 0, 1, 0), 256);
    // Perpendicular vectors: dot = 0 -> score 0.
    assert_eq!(score(1, 0, 0, 1), 0);
    // Zero-magnitude input vector: guarded to 0, never a divide-by-zero panic.
    assert_eq!(score(0, 0, 1, 1), 0);
}

#[test]
fn cross2d_state_cell_matches_defined_behaviour() {
    // cross2d: signed scalar 2D cross product ax*by - ay*bx of (ax,ay) and (bx,by),
    // returned as an exact (cross_mag, cross_neg) pair (neg 0=nonnegative, 1=negative) --
    // the same combining-subtract technique cross_product uses for one component, but
    // simplified since ax/ay/bx/by are plain u16 magnitudes (no i16 sign-tracking on inputs).
    fn cross2d(a: (u16, u16), b: (u16, u16)) -> (u64, u64) {
        let mut cell = StateCell::bind(&cell_src("cross2d"), "Cross2d", None).unwrap();
        for (f, v) in [
            ("ax", a.0 as u64),
            ("ay", a.1 as u64),
            ("bx", b.0 as u64),
            ("by", b.1 as u64),
        ] {
            cell.set(f, v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap();
        (
            cell.get("cross_mag").unwrap(),
            cell.get("cross_neg").unwrap(),
        )
    }

    // Unit basis vectors: i x j = +1 (counter-clockwise sense).
    assert_eq!(cross2d((1, 0), (0, 1)), (1, 0));
    // Swapped order flips the sign: j x i = -1.
    assert_eq!(cross2d((0, 1), (1, 0)), (1, 1));
    // Known case: (2,3) x (5,6) = 2*6 - 3*5 = -3 -> mag 3, neg 1.
    assert_eq!(cross2d((2, 3), (5, 6)), (3, 1));
    // Collinear vectors: cross is exactly zero, forced to neg 0.
    assert_eq!(cross2d((2, 4), (1, 2)), (0, 0));
    // Near the u16 boundary: 65535*65535 fits exactly in u32 with room to spare.
    assert_eq!(cross2d((65535, 0), (0, 65535)), (4294836225, 0));
}

#[test]
fn vec3_length_matches_hand_computed_cases() {
    // vec3_length: floor(sqrt(x*x + y*y + z*z)) for a signed 3D vector -- the sqrt sibling
    // of norm3_sq, closed by isqrt_u32's wide integer sqrt. Reuses norm3_sq's exact
    // i16_mag/mul_checked_u32/add_checked_u32 chain to build mag_sq internally, then runs
    // isqrt_u32's branch-free bitwise loop on it before returning the u16 length.
    fn i16_bits(v: i16) -> u64 {
        (v as u16) as u64
    }
    fn length(x: i16, y: i16, z: i16) -> u16 {
        let mut cell = StateCell::bind(&cell_src("vec3_length"), "Vec3Length", None).unwrap();
        cell.set("x", i16_bits(x)).unwrap();
        cell.set("y", i16_bits(y)).unwrap();
        cell.set("z", i16_bits(z)).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap().result
    }

    // (3,4,0): 9+16+0=25, sqrt(25)=5 exactly.
    assert_eq!(length(3, 4, 0), 5);
    // (0,0,0): the zero vector.
    assert_eq!(length(0, 0, 0), 0);
    // (2,3,6): 4+9+36=49, sqrt(49)=7 exactly -- the classic 2-3-6-7 Pythagorean quadruple.
    assert_eq!(length(2, 3, 6), 7);
    // (1,1,1): mag_sq=3, sqrt(3)=1.732..., floor = 1 -- confirms truncation, not rounding.
    assert_eq!(length(1, 1, 1), 1);
    // (-1,-2,2): mixed sign, mag_sq = 1+4+4=9, sqrt(9)=3 exactly.
    assert_eq!(length(-1, -2, 2), 3);
    // Extreme: (i16::MIN, i16::MIN, i16::MIN). mag_sq = 3 * 32768^2 = 3,221,225,472,
    // comfortably inside u32 range (never trips the checked-add/mul overflow halt).
    // 56755^2 = 3,221,130,025 <= mag_sq < 56756^2 = 3,221,243,536, so floor sqrt = 56755.
    assert_eq!(length(-32768, -32768, -32768), 56755);
}

#[test]
fn vectors_orthogonal_hand_computed_cases() {
    // vectors_orthogonal: dot3(a,b) == 0, reusing dot3's sign-magnitude product/sum
    // chain internally and testing the final magnitude for zero. Distinct from
    // vectors_parallel (cross-product-zero); most pairs are neither. Cross-checked by
    // hand: dot = ax*bx + ay*by + az*bz.
    fn i16_bits(v: i16) -> u64 {
        (v as u16) as u64
    }
    fn orthogonal(a: (i16, i16, i16), b: (i16, i16, i16)) -> u16 {
        let mut cell = StateCell::bind(&cell_src("vectors_orthogonal"), "VectorsOrthogonal", None)
            .unwrap_or_else(|e| panic!("bind vectors_orthogonal: {e}"));
        for (f, v) in [
            ("ax", i16_bits(a.0)),
            ("ay", i16_bits(a.1)),
            ("az", i16_bits(a.2)),
            ("bx", i16_bits(b.0)),
            ("by", i16_bits(b.1)),
            ("bz", i16_bits(b.2)),
        ] {
            cell.set(f, v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap();
        cell.get("result").unwrap() as u16
    }

    // (1,0,0).(0,1,0) = 0 -- basis vectors, orthogonal.
    assert_eq!(orthogonal((1, 0, 0), (0, 1, 0)), 1);
    // (1,2,3).(2,-1,0) = 2 - 2 + 0 = 0 -- non-axis-aligned orthogonal pair.
    assert_eq!(orthogonal((1, 2, 3), (2, -1, 0)), 1);
    // (3,4,5).(6,8,10) = 18+32+50 = 100 -- parallel (scalar multiple), not orthogonal.
    assert_eq!(orthogonal((3, 4, 5), (6, 8, 10)), 0);
    // (3,-1,2).(1,4,-2) = 3-4-4 = -5 -- neither parallel nor orthogonal.
    assert_eq!(orthogonal((3, -1, 2), (1, 4, -2)), 0);
    // (0,0,0).(5,-3,7) = 0 -- the zero vector is trivially orthogonal to anything.
    assert_eq!(orthogonal((0, 0, 0), (5, -3, 7)), 1);
}
