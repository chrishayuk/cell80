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
