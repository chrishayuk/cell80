//! Host-oracle tests for the fixed-point pack (`cell80/cells/fixed-point/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::{cell_src, run_cell};
use cell80::{StateCell, DEFAULT_CYCLES};

#[test]
fn library_growth_backlog_fixed_point_slice() {
    fn variance_step(fields: &[(&str, u64)]) -> StateCell {
        let mut cell =
            StateCell::bind(&cell_src("running_variance_step"), "RunningVariance", None).unwrap();
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap();
        cell
    }

    // q_sqrt: sqrt(x/256)*256 via a branch-free bitwise integer sqrt.
    assert_eq!(run_cell("q_sqrt", &[0]), 0);
    assert_eq!(run_cell("q_sqrt", &[256]), 256); // sqrt(1.0) = 1.0
    assert_eq!(run_cell("q_sqrt", &[1024]), 512); // sqrt(4.0) = 2.0
    assert_eq!(run_cell("q_sqrt", &[65535]), 4095); // domain extreme, ~15.9998

    // q_sigmoid: hard sigmoid, clamp(x/4 + 0.5, 0, 1) in Q8.8; saturates outside [-4, 4].
    assert_eq!(run_cell("q_sigmoid", &[0]), 128); // sigmoid(0) = 0.5
    assert_eq!(run_cell("q_sigmoid", &[400]), 228); // 400/4 + 128 = 228, unclamped
    assert_eq!(run_cell("q_sigmoid", &[1024]), 256); // saturates high (x = 4.0)
    assert_eq!(
        run_cell("q_sigmoid", &[65536u32.wrapping_sub(1024) as u16]),
        0
    ); // -4.0, saturates low

    // The "straightforward deferred set" from docs/library-growth.md's Next waves /
    // pack-note backlog: q_sqrt, q_sigmoid, running_variance_step, morton_encode/decode,
    // bresenham_step, rate_window_update. q_tanh was deliberately not built — it reduces
    // exactly to clamp_i16(x, -256, 256), now tagged on that cell instead.

    // running_variance_step: [10, 20, 30] -> population variance 200/3, exact match to a
    // hand-derived reference (mean recomputed fresh each side of the update, not compounded).
    let (mut count, mut sum, mut m2) = (0u64, 0u64, 0u64);
    for value in [10u64, 20, 30] {
        let cell = variance_step(&[("value", value), ("count", count), ("sum", sum), ("m2", m2)]);
        count = cell.get("count").unwrap();
        sum = cell.get("sum").unwrap();
        m2 = cell.get("m2").unwrap();
    }
    assert_eq!((count, sum, m2), (3, 60, 200)); // variance = 200/3 ~= 66.67

}

#[test]
fn first_wave_fixed_point_cells_match_defined_behaviour() {
    let cases: &[(&str, &[u16], u16)] = &[
        ("q_mul", &[384, 512], 768),       // 1.5 * 2.0 = 3.0
        ("q_mul", &[256, 256], 256),       // 1.0 * 1.0 = 1.0 (identity)
        ("q_div", &[768, 512], 384),       // 3.0 / 2.0 = 1.5
        ("q_div", &[768, 0], 0),           // divide by zero — safe
        ("q_lerp", &[0, 256, 128], 128),   // halfway, forward
        ("q_lerp", &[200, 100, 64], 175),  // t=0.25, b < a (reverse branch)
        ("q_lerp", &[100, 200, 0], 100),   // t=0 → a
        ("q_lerp", &[100, 200, 256], 200), // t=1.0 → b
    ];

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
