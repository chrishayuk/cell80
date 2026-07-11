//! Host-oracle tests for the matrix pack (`cell80/cells/matrix/*.rs`). Mirrors the cells'
//! own pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::cell_src;
use cell80::{StateCell, DEFAULT_CYCLES};

#[test]
fn wave13_matrix_2x2_cells_match_defined_behaviour() {
    // Wave 13 (docs/math-server-map.md's linear_algebra.matrices category). Every
    // expected value was cross-checked against an independent Python reference
    // implementation before transcription.
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

    // matrix_det_2x2: a*d - b*c.
    let (_, cell) = step(
        "matrix_det_2x2",
        "MatrixDet2x2",
        &[
            ("a", i16_bits(3)),
            ("b", i16_bits(4)),
            ("c", i16_bits(1)),
            ("d", i16_bits(2)),
        ],
    );
    assert_eq!(
        (cell.get("result_mag"), cell.get("result_neg")),
        (Some(2), Some(0))
    );
    let (_, cell) = step(
        "matrix_det_2x2",
        "MatrixDet2x2",
        &[
            ("a", i16_bits(1)),
            ("b", i16_bits(2)),
            ("c", i16_bits(3)),
            ("d", i16_bits(4)),
        ],
    );
    assert_eq!(
        (cell.get("result_mag"), cell.get("result_neg")),
        (Some(2), Some(1))
    ); // 1*4-2*3 = -2
    let (_, cell) = step(
        "matrix_det_2x2",
        "MatrixDet2x2",
        &[
            ("a", i16_bits(0)),
            ("b", i16_bits(0)),
            ("c", i16_bits(0)),
            ("d", i16_bits(0)),
        ],
    );
    assert_eq!(
        (cell.get("result_mag"), cell.get("result_neg")),
        (Some(0), Some(0))
    );

    // matrix_solve_2x2: Cramer's rule, x and y sharing one positive denominator.
    fn solve(m: (i16, i16, i16, i16, i16, i16)) -> (cell80::Report, StateCell) {
        step(
            "matrix_solve_2x2",
            "MatrixSolve2x2",
            &[
                ("a", i16_bits(m.0)),
                ("b", i16_bits(m.1)),
                ("c", i16_bits(m.2)),
                ("d", i16_bits(m.3)),
                ("e", i16_bits(m.4)),
                ("f", i16_bits(m.5)),
            ],
        )
    }

    // x + y = 3, x - y = 1  ->  x=2, y=1.
    let (_, cell) = solve((1, 1, 1, -1, 3, 1));
    assert_eq!(
        (cell.get("x_num_mag"), cell.get("x_num_neg")),
        (Some(4), Some(0))
    );
    assert_eq!(cell.get("den"), Some(2)); // x = 4/2 = 2
    assert_eq!(
        (cell.get("y_num_mag"), cell.get("y_num_neg")),
        (Some(2), Some(0))
    );
    assert_eq!(cell.get("den"), Some(2)); // y = 2/2 = 1

    // 2x + y = 5, x + 3y = 10  ->  x=1, y=3.
    let (_, cell) = solve((2, 1, 1, 3, 5, 10));
    assert_eq!(
        (cell.get("x_num_mag"), cell.get("x_num_neg")),
        (Some(5), Some(0))
    );
    assert_eq!(cell.get("den"), Some(5));
    assert_eq!(
        (cell.get("y_num_mag"), cell.get("y_num_neg")),
        (Some(15), Some(0))
    );

    // Negative determinant: a=1,b=2,c=3,d=4,e=5,f=6 -> det=-2, x=-4, y=4.5 (9/2).
    let (_, cell) = solve((1, 2, 3, 4, 5, 6));
    assert_eq!(
        (cell.get("x_num_mag"), cell.get("x_num_neg")),
        (Some(8), Some(1))
    );
    assert_eq!(cell.get("den"), Some(2)); // x = -8/2 = -4
    assert_eq!(
        (cell.get("y_num_mag"), cell.get("y_num_neg")),
        (Some(9), Some(0))
    );
    assert_eq!(cell.get("den"), Some(2)); // y = 9/2 = 4.5

    // Zero determinant: no unique solution.
    let (report, _) = solve((1, 2, 2, 4, 5, 6));
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
}

#[test]
fn matrix_apply_2x2_matches_defined_behaviour() {
    // matrix_apply_2x2: (rx, ry) = (a*x + b*y, c*x + d*y), each output a sign-magnitude
    // combine of two signed products (the forward counterpart of matrix_solve_2x2's
    // reverse Cramer's-rule solve). Every expected value hand-computed before running.
    fn i16_bits(v: i16) -> u64 {
        (v as u16) as u64
    }
    fn apply(a: i16, b: i16, c: i16, d: i16, x: i16, y: i16) -> (u64, u64, u64, u64) {
        let mut cell = StateCell::bind(&cell_src("matrix_apply_2x2"), "MatrixApply2x2", None)
            .unwrap_or_else(|e| panic!("bind matrix_apply_2x2: {e}"));
        cell.set("a", i16_bits(a)).unwrap();
        cell.set("b", i16_bits(b)).unwrap();
        cell.set("c", i16_bits(c)).unwrap();
        cell.set("d", i16_bits(d)).unwrap();
        cell.set("x", i16_bits(x)).unwrap();
        cell.set("y", i16_bits(y)).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        assert_eq!(report.halt, cell80::Halt::Returned);
        (
            cell.get("rx_mag").unwrap(),
            cell.get("rx_neg").unwrap(),
            cell.get("ry_mag").unwrap(),
            cell.get("ry_neg").unwrap(),
        )
    }

    // Identity matrix leaves the vector unchanged.
    assert_eq!(apply(1, 0, 0, 1, 5, 7), (5, 0, 7, 0));

    // All-positive coefficients and vector.
    // rx = 2*1 + 3*1 = 5, ry = 4*1 + 5*1 = 9.
    assert_eq!(apply(2, 3, 4, 5, 1, 1), (5, 0, 9, 0));

    // Mixed-sign coefficients, positive result.
    // rx = 2*5 + (-3)*2 = 10 - 6 = 4; ry = (-1)*5 + 4*2 = -5 + 8 = 3.
    assert_eq!(apply(2, -3, -1, 4, 5, 2), (4, 0, 3, 0));

    // Negative result on both components.
    // rx = 1*(-3) + 1*1 = -2; ry = 1*(-3) + 1*1 = -2.
    assert_eq!(apply(1, 1, 1, 1, -3, 1), (2, 1, 2, 1));

    // Exact cancellation to zero (zero magnitude forces neg=0).
    // rx = 1*5 + (-1)*5 = 0; ry = 2*5 + (-2)*5 = 0.
    assert_eq!(apply(1, -1, 2, -2, 5, 5), (0, 0, 0, 0));

    // Negative matrix and vector.
    // rx = (-1)*(-4) + (-1)*6 = 4 - 6 = -2; ry = (-1)*(-4) + (-1)*6 = -2.
    assert_eq!(apply(-1, -1, -1, -1, -4, 6), (2, 1, 2, 1));
}

#[test]
fn matrix_mul_2x2_composes_two_2x2_matrices() {
    // Sign-magnitude sum-of-products check for C = A*B, mirroring the wave13-style
    // helpers already in this file: i16_bits packs a signed field, step binds+sets+runs.
    fn i16_bits(v: i16) -> u64 {
        (v as u16) as u64
    }
    fn mul(m: (i16, i16, i16, i16, i16, i16, i16, i16)) -> StateCell {
        let mut cell = StateCell::bind(&cell_src("matrix_mul_2x2"), "MatrixMul2x2", None)
            .unwrap_or_else(|e| panic!("bind matrix_mul_2x2: {e}"));
        for (f, v) in [
            ("a", m.0),
            ("b", m.1),
            ("c", m.2),
            ("d", m.3),
            ("e", m.4),
            ("f", m.5),
            ("g", m.6),
            ("h", m.7),
        ] {
            cell.set(f, i16_bits(v)).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap();
        cell
    }

    // A=[[1,2],[3,4]], B=[[5,6],[7,8]] -> C=[[19,22],[43,50]] (all positive).
    let cell = mul((1, 2, 3, 4, 5, 6, 7, 8));
    assert_eq!(
        (cell.get("r11_mag"), cell.get("r11_neg")),
        (Some(19), Some(0))
    );
    assert_eq!(
        (cell.get("r12_mag"), cell.get("r12_neg")),
        (Some(22), Some(0))
    );
    assert_eq!(
        (cell.get("r21_mag"), cell.get("r21_neg")),
        (Some(43), Some(0))
    );
    assert_eq!(
        (cell.get("r22_mag"), cell.get("r22_neg")),
        (Some(50), Some(0))
    );

    // A=[[1,-2],[3,4]], B=[[-5,6],[7,-8]] -> C=[[-19,22],[13,-14]] (mixed signs).
    let cell = mul((1, -2, 3, 4, -5, 6, 7, -8));
    assert_eq!(
        (cell.get("r11_mag"), cell.get("r11_neg")),
        (Some(19), Some(1))
    );
    assert_eq!(
        (cell.get("r12_mag"), cell.get("r12_neg")),
        (Some(22), Some(0))
    );
    assert_eq!(
        (cell.get("r21_mag"), cell.get("r21_neg")),
        (Some(13), Some(0))
    );
    assert_eq!(
        (cell.get("r22_mag"), cell.get("r22_neg")),
        (Some(14), Some(1))
    );

    // A=[[2,1],[3,4]], B=[[3,5],[-6,2]]: c11 = 2*3 + 1*-6 = 0 exactly (cancellation ->
    // neg must be forced back to 0, not left dangling from whichever operand "lost").
    let cell = mul((2, 1, 3, 4, 3, 5, -6, 2));
    assert_eq!(
        (cell.get("r11_mag"), cell.get("r11_neg")),
        (Some(0), Some(0))
    );
    assert_eq!(
        (cell.get("r12_mag"), cell.get("r12_neg")),
        (Some(12), Some(0))
    );
    assert_eq!(
        (cell.get("r21_mag"), cell.get("r21_neg")),
        (Some(15), Some(1))
    );
    assert_eq!(
        (cell.get("r22_mag"), cell.get("r22_neg")),
        (Some(23), Some(0))
    );
}

#[test]
fn matrix_trace_2x2_hand_computed_cases() {
    // Trace of [[a,_],[_,d]] = a + d, tracked as an exact (magnitude, sign) pair since a
    // native i16 add can overflow i16's own range even for individually-valid i16 inputs
    // (e.g. a=d=i16::MAX). Every case below is hand-computed, not taken from the compiled
    // output.
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
    fn trace(a: i16, d: i16) -> (cell80::Report, StateCell) {
        step(
            "matrix_trace_2x2",
            "MatrixTrace2x2",
            &[("a", i16_bits(a)), ("d", i16_bits(d))],
        )
    }

    // Same-sign positive: 3 + 2 = 5.
    let (_, cell) = trace(3, 2);
    assert_eq!(
        (cell.get("trace_mag"), cell.get("trace_neg")),
        (Some(5), Some(0))
    );

    // Same-sign negative: -3 + -5 = -8.
    let (_, cell) = trace(-3, -5);
    assert_eq!(
        (cell.get("trace_mag"), cell.get("trace_neg")),
        (Some(8), Some(1))
    );

    // Opposite sign, equal magnitude: 5 + -5 = 0, forced neg=0 (no negative zero).
    let (_, cell) = trace(5, -5);
    assert_eq!(
        (cell.get("trace_mag"), cell.get("trace_neg")),
        (Some(0), Some(0))
    );

    // Opposite sign, d's magnitude wins: 4 + -6 = -2.
    let (_, cell) = trace(4, -6);
    assert_eq!(
        (cell.get("trace_mag"), cell.get("trace_neg")),
        (Some(2), Some(1))
    );

    // i16::MAX + i16::MAX = 65534, which overflows i16's own representable range
    // (max 32767) -- proves the sign-magnitude widening to u32 is load-bearing.
    let (_, cell) = trace(32767, 32767);
    assert_eq!(
        (cell.get("trace_mag"), cell.get("trace_neg")),
        (Some(65534), Some(0))
    );

    // i16::MIN + i16::MIN: magnitude 32768 + 32768 = 65536, exercises add_checked_u32
    // near the top of i16::MIN's own magnitude.
    let (_, cell) = trace(-32768, -32768);
    assert_eq!(
        (cell.get("trace_mag"), cell.get("trace_neg")),
        (Some(65536), Some(1))
    );
}

#[test]
fn matrix_real_eigenvalues_2x2_matches_defined_behaviour() {
    // matrix_real_eigenvalues_2x2: predicate -- does [[a,b],[c,d]] have two real eigenvalues
    // (discriminant = trace^2 - 4*det >= 0) or a complex-conjugate pair (discriminant < 0)?
    // Every expected value was hand-computed from the characteristic polynomial before
    // transcription.
    fn i16_bits(v: i16) -> u64 {
        (v as u16) as u64
    }
    fn eigen(a: i16, b: i16, c: i16, d: i16) -> Option<u64> {
        let mut cell = StateCell::bind(
            &cell_src("matrix_real_eigenvalues_2x2"),
            "MatrixRealEigenvalues2x2",
            None,
        )
        .unwrap_or_else(|e| panic!("bind matrix_real_eigenvalues_2x2: {e}"));
        cell.set("a", i16_bits(a)).unwrap();
        cell.set("b", i16_bits(b)).unwrap();
        cell.set("c", i16_bits(c)).unwrap();
        cell.set("d", i16_bits(d)).unwrap();
        cell.run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run matrix_real_eigenvalues_2x2: {e}"));
        cell.get("result")
    }

    // [[1,0],[0,1]]: trace=2, det=1, discriminant=4-4=0 -> real (repeated eigenvalue 1,1).
    assert_eq!(eigen(1, 0, 0, 1), Some(1));

    // [[0,1],[-1,0]]: 90-degree rotation. trace=0, det=1, discriminant=0-4=-4 -> complex pair.
    assert_eq!(eigen(0, 1, -1, 0), Some(0));

    // [[3,4],[1,2]]: trace=5, det=2, discriminant=25-8=17 -> real (matches matrix_det_2x2's
    // own worked example).
    assert_eq!(eigen(3, 4, 1, 2), Some(1));

    // [[2,-1],[1,2]]: trace=4, det=5, discriminant=16-20=-4 -> complex.
    assert_eq!(eigen(2, -1, 1, 2), Some(0));

    // [[5,0],[0,-3]]: diagonal with distinct real eigenvalues 5 and -3. trace=2, det=-15,
    // discriminant=4-4*(-15)=64 -> real (also exercises the det<0 branch, where the
    // predicate is true unconditionally regardless of trace).
    assert_eq!(eigen(5, 0, 0, -3), Some(1));
}

#[test]
fn matrix_frobenius_norm_sq_2x2_matches_hand_computed_values() {
    // Checks matrix_frobenius_norm_sq_2x2: a*a + b*b + c*c + d*d (Frobenius norm squared,
    // no sqrt), the matrix pack's flat-vector-of-4 sibling to vector::norm2_sq/norm3_sq.
    use cell80::{StateCell, DEFAULT_CYCLES};

    fn i16_bits(v: i16) -> u64 {
        (v as u16) as u64
    }
    fn step(fields: &[(&str, u64)]) -> StateCell {
        let mut cell = StateCell::bind(
            &cell_src("matrix_frobenius_norm_sq_2x2"),
            "MatrixFrobeniusNormSq2x2",
            None,
        )
        .unwrap_or_else(|e| panic!("bind: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap();
        cell
    }

    // a=3,b=4,c=0,d=0 -> 9+16+0+0 = 25
    let cell = step(&[
        ("a", i16_bits(3)),
        ("b", i16_bits(4)),
        ("c", i16_bits(0)),
        ("d", i16_bits(0)),
    ]);
    assert_eq!(cell.get("result"), Some(25));

    // a=1,b=2,c=3,d=4 -> 1+4+9+16 = 30
    let cell = step(&[
        ("a", i16_bits(1)),
        ("b", i16_bits(2)),
        ("c", i16_bits(3)),
        ("d", i16_bits(4)),
    ]);
    assert_eq!(cell.get("result"), Some(30));

    // Negatives: squares are sign-independent. a=-3,b=4,c=-5,d=12 -> 9+16+25+144 = 194
    let cell = step(&[
        ("a", i16_bits(-3)),
        ("b", i16_bits(4)),
        ("c", i16_bits(-5)),
        ("d", i16_bits(12)),
    ]);
    assert_eq!(cell.get("result"), Some(194));

    // Larger magnitudes, mixed signs. a=100,b=-200,c=300,d=-400 -> 10000+40000+90000+160000 = 300000
    let cell = step(&[
        ("a", i16_bits(100)),
        ("b", i16_bits(-200)),
        ("c", i16_bits(300)),
        ("d", i16_bits(-400)),
    ]);
    assert_eq!(cell.get("result"), Some(300000));
}

#[test]
fn matrix_det_3x3_matches_test_cases() {
    // Excel's MDETERM, landed in the matrix pack (not excel-mathstat) per this wave's
    // naming convention: matrix_det_3x3 / MatrixDet3x3, matching matrix_det_2x2's own
    // sibling naming. First-row cofactor expansion, f32 throughout (matrix_solve_3x3's
    // own tier) -- unlike matrix_solve_3x3, a zero determinant is a legitimate answer,
    // not an escalation.
    fn det(m: [f32; 9]) -> f32 {
        let mut cell = StateCell::bind(&cell_src("matrix_det_3x3"), "MatrixDet3x3", None)
            .unwrap_or_else(|e| panic!("bind matrix_det_3x3: {e}"));
        for (f, v) in [
            "a11", "a12", "a13", "a21", "a22", "a23", "a31", "a32", "a33",
        ]
        .iter()
        .zip(m.iter())
        {
            cell.set(f, v.to_bits() as u64).unwrap();
        }
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        assert_eq!(report.halt, cell80::Halt::Returned);
        f32::from_bits(cell.get("det").unwrap() as u32)
    }
    fn approx(got: f32, want: f32) {
        let tol = (want.abs() * 1e-3).max(1e-3);
        assert!((got - want).abs() < tol, "got {got} want {want}");
    }

    // General 3x3, det = -3 (hand-worked via first-row cofactor expansion).
    approx(det([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 10.0]), -3.0);

    // Identity matrix -> det = 1.
    approx(det([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]), 1.0);

    // Singular matrix (row 2 = 2 * row 1) -> det = 0 exactly, NOT an escalation.
    approx(det([1.0, 2.0, 3.0, 2.0, 4.0, 6.0, 7.0, 8.0, 9.0]), 0.0);

    // Negative coefficients, exercising sign flips through the minors and the
    // middle-term subtraction: det = 4.
    approx(det([2.0, -1.0, 0.0, -1.0, 2.0, -1.0, 0.0, -1.0, 2.0]), 4.0);
}

#[test]
fn matrix_inverse_2x2_matches_test_cases() {
    // MINVERSE: (1/det)*[[d,-b],[-c,a]], all four entries sharing one determinant
    // division computed once and reused. f32 throughout (matrix_solve_3x3's own tier),
    // unlike matrix_solve_2x2's exact signed fraction.
    fn inv(a: f32, b: f32, c: f32, d: f32) -> (f32, f32, f32, f32) {
        let mut cell = StateCell::bind(&cell_src("matrix_inverse_2x2"), "MatrixInverse2x2", None)
            .unwrap_or_else(|e| panic!("bind matrix_inverse_2x2: {e}"));
        cell.set("a", a.to_bits() as u64).unwrap();
        cell.set("b", b.to_bits() as u64).unwrap();
        cell.set("c", c.to_bits() as u64).unwrap();
        cell.set("d", d.to_bits() as u64).unwrap();
        let report = cell.run(DEFAULT_CYCLES).unwrap();
        assert_eq!(report.halt, cell80::Halt::Returned);
        (
            f32::from_bits(cell.get("inv_a").unwrap() as u32),
            f32::from_bits(cell.get("inv_b").unwrap() as u32),
            f32::from_bits(cell.get("inv_c").unwrap() as u32),
            f32::from_bits(cell.get("inv_d").unwrap() as u32),
        )
    }
    fn approx(got: f32, want: f32) {
        let tol = (want.abs() * 1e-3).max(1e-3);
        assert!((got - want).abs() < tol, "got {got} want {want}");
    }

    // [[4,7],[2,6]]: det=10. inv = [[0.6,-0.7],[-0.2,0.4]].
    let (ia, ib, ic, id) = inv(4.0, 7.0, 2.0, 6.0);
    approx(ia, 0.6);
    approx(ib, -0.7);
    approx(ic, -0.2);
    approx(id, 0.4);

    // [[2,0],[0,2]]: det=4. inv = 0.5*I.
    let (ia, ib, ic, id) = inv(2.0, 0.0, 0.0, 2.0);
    approx(ia, 0.5);
    approx(ib, 0.0);
    approx(ic, 0.0);
    approx(id, 0.5);

    // [[1,2],[3,4]]: det=-2 (negative determinant, exercising the sign path).
    let (ia, ib, ic, id) = inv(1.0, 2.0, 3.0, 4.0);
    approx(ia, -2.0);
    approx(ib, 1.0);
    approx(ic, 1.5);
    approx(id, -0.5);

    // Singular (det=0): must escalate out_of_domain, not return a matrix.
    let mut cell = StateCell::bind(&cell_src("matrix_inverse_2x2"), "MatrixInverse2x2", None)
        .unwrap_or_else(|e| panic!("bind matrix_inverse_2x2: {e}"));
    cell.set("a", 1.0f32.to_bits() as u64).unwrap();
    cell.set("b", 2.0f32.to_bits() as u64).unwrap();
    cell.set("c", 2.0f32.to_bits() as u64).unwrap();
    cell.set("d", 4.0f32.to_bits() as u64).unwrap();
    let report = cell.run(DEFAULT_CYCLES).unwrap();
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF06));
}
