//! Host-oracle tests for the fixed-point pack (`cell80/cells/fixed-point/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::{cell_src, run_cell};
use cell80::{Runner, StateCell, DEFAULT_CYCLES};

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

// int_to_q8: encodes a plain integer into Q8.8 (x << 8); escalates past the 8-bit
// integer-part ceiling (x > 255) rather than silently truncating the high bits.
#[test]
fn int_to_q8_encodes_and_escalates_past_255() {
    fn report(id: &str, args: &[u16]) -> cell80::Report {
        let mut r = Runner::compile(&cell_src(id)).unwrap_or_else(|e| panic!("compile {id}: {e}"));
        r.run(None, args, DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run {id}: {e}"))
    }

    assert_eq!(report("int_to_q8", &[0]).result, 0); // 0 -> 0.0
    assert_eq!(report("int_to_q8", &[1]).result, 256); // 1 -> 1.0
    assert_eq!(report("int_to_q8", &[4]).result, 1024); // 4 -> 4.0
    assert_eq!(report("int_to_q8", &[255]).result, 65280); // boundary, still fits
    assert_eq!(
        report("int_to_q8", &[256]).halt,
        cell80::Halt::Escalate(0xFF05)
    ); // just past the boundary -> needs_wider_math
    assert_eq!(
        report("int_to_q8", &[65535]).halt,
        cell80::Halt::Escalate(0xFF05)
    );
}

#[test]
fn q_mul_i16_signed_q8_8_multiply_matches_hand_computed_cases() {
    // q_mul_i16: signed Q8.8 multiply via sign-magnitude (i16_mag/i16_neg decompose each input,
    // magnitudes multiply and shift right 8 mirroring q_mul's own (a*b)>>8, sign is the XOR of
    // the input signs) -- q_mul's signed counterpart, since q_mul is unsigned-only.

    // 1.5 * 2.0 = 3.0, both positive. Q8.8: 384 * 512, (384*512)>>8 = 768.
    assert_eq!(run_cell("q_mul_i16", &[384, 512]), 768);

    // -1.5 * 2.0 = -3.0, mixed sign. -1.5 as u16 bits: 65536-384 = 65152; -3.0: 65536-768 = 64768.
    assert_eq!(run_cell("q_mul_i16", &[65152, 512]), 64768);

    // -1.5 * -2.0 = 3.0, both negative -> positive result. -2.0 as u16 bits: 65536-512 = 65024.
    assert_eq!(run_cell("q_mul_i16", &[65152, 65024]), 768);

    // 0 * -1.953125 = 0: a zero magnitude with a "negative" XOR sign flag must still collapse to
    // plain 0 (no -0 in i16). -500 as u16 bits: 65536-500 = 65036.
    assert_eq!(run_cell("q_mul_i16", &[0, 65036]), 0);

    // Overflow: i16::MAX * i16::MAX (~127.996 * ~127.996 in real terms) multiplies out to a real
    // product (~16383) with no representation in Q8.8 i16 (max ~127.996) -> escalates
    // (halt 0xFF05, needs_wider_math) instead of silently truncating.
    let mut r = cell80::Runner::compile(&cell_src("q_mul_i16")).unwrap();
    let report = r.run(None, &[32767, 32767], DEFAULT_CYCLES).unwrap();
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}
