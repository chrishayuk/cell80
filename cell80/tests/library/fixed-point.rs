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

#[test]
fn q_div_i16_signed_q8_8_divide_matches_hand_computed_cases() {
    // q_div_i16: signed Q8.8 divide via sign-magnitude (i16_mag/i16_neg decompose each input,
    // magnitudes combine as (mag_a << 8) / mag_b at wide u32 width mirroring q_div's own
    // (a<<8)/b, sign is the XOR of the input signs) -- q_div's signed counterpart, since q_div
    // is unsigned-only.

    // Both positive: 10 / 4 in raw Q8.8 form -> (10<<8)/4 = 2560/4 = 640.
    assert_eq!(run_cell("q_div_i16", &[10, 4]), 640);

    // Negative / positive: -10 / 4 -> magnitude 640, sign negative -> -640 (64896 as u16 bits).
    // -10 as u16 bits: 65536-10 = 65526.
    assert_eq!(run_cell("q_div_i16", &[65526, 4]), 64896);

    // Positive / negative: 10 / -4 -> magnitude 640, sign negative -> -640 (64896).
    // -4 as u16 bits: 65536-4 = 65532.
    assert_eq!(run_cell("q_div_i16", &[10, 65532]), 64896);

    // Negative / negative: -10 / -4 -> signs cancel -> +640.
    assert_eq!(run_cell("q_div_i16", &[65526, 65532]), 640);

    // Zero divisor: b == 0 -> 0, matching q_div's own zero-divisor convention (no halt).
    assert_eq!(run_cell("q_div_i16", &[100, 0]), 0);

    // Boundary that fits exactly: 32767 / 256 -> (32767<<8)/256 = 32767 (i16::MAX), no halt.
    assert_eq!(run_cell("q_div_i16", &[32767, 256]), 32767);

    // Overflow: a=20000, b=1 -> scaled magnitude 20000<<8 = 5,120,000, far past the
    // post-shift i16 limit (32767 positive / 32768 negative) -> escalates instead of
    // silently truncating.
    let mut r = Runner::compile(&cell_src("q_div_i16")).unwrap();
    let report = r.run(None, &[20000, 1], DEFAULT_CYCLES).unwrap();
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

// int_to_q8_i16: signed counterpart of int_to_q8 -- encodes x into Q8.8 via x << 8 for x in
// [-128, 127], escalating (halt 0xFF05, needs_wider_math) outside that range since Q8.8's 8
// signed integer bits can't hold a larger whole-number part without losing high bits. This is
// the missing encode step for the pack's signed cells (q_sigmoid, q_mul_i16, q_div_i16,
// clamp_i16), mirroring how int_to_q8 already serves the unsigned ones.
#[test]
fn int_to_q8_i16_encodes_signed_and_escalates_outside_i8_range() {
    fn report(id: &str, args: &[u16]) -> cell80::Report {
        let mut r = Runner::compile(&cell_src(id)).unwrap_or_else(|e| panic!("compile {id}: {e}"));
        r.run(None, args, DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run {id}: {e}"))
    }

    // 0 -> 0.0
    assert_eq!(run_cell("int_to_q8_i16", &[0]), 0);
    // 1 -> 1.0 = 256
    assert_eq!(run_cell("int_to_q8_i16", &[1]), 256);
    // boundary high: 127 -> 127 << 8 = 32512, still fits in i16
    assert_eq!(run_cell("int_to_q8_i16", &[127]), 32512);
    // -1 -> -256, as u16 bit pattern: 65536-256 = 65280
    assert_eq!(run_cell("int_to_q8_i16", &[65535]), 65280);
    // boundary low: -128 -> -32768 exactly (i16::MIN), as u16 bit pattern: 65536-32768 = 32768.
    // input -128 as u16 bits is 65536-128 = 65408.
    assert_eq!(run_cell("int_to_q8_i16", &[65408]), 32768);

    // Escalation: just past either boundary must halt 0xFF05 rather than silently wrap.
    assert_eq!(
        report("int_to_q8_i16", &[128]).halt,
        cell80::Halt::Escalate(0xFF05)
    ); // 128 is out of range high
    assert_eq!(
        report("int_to_q8_i16", &[65407]).halt, // -129 as u16 bits: 65536-129 = 65407
        cell80::Halt::Escalate(0xFF05)
    );
}

#[test]
fn q_mul_checked_q8_8_multiply_matches_hand_computed_cases() {
    // q_mul_checked: (a*b)>>8 in Q8.8, but escalates (halt 0xFF05, needs_wider_math) instead
    // of silently truncating when the scaled product doesn't fit u16 -- q_mul's checked
    // counterpart (q_mul's own doc comment documents the unguarded shift, no escalation).

    // 1.5 * 2.0 = 3.0 in Q8.8: (384*512)>>8 = 768. No halt.
    assert_eq!(run_cell("q_mul_checked", &[384, 512]), 768);

    // 1.0 * 1.0 = 1.0: (256*256)>>8 = 256. No halt.
    assert_eq!(run_cell("q_mul_checked", &[256, 256]), 256);

    // 0 * anything = 0.
    assert_eq!(run_cell("q_mul_checked", &[0, 12345]), 0);

    // Exact boundary: 65535 * 256 -> 16776960 >> 8 = 65535 == u16::MAX exactly, no halt.
    assert_eq!(run_cell("q_mul_checked", &[65535, 256]), 65535);

    // One past the boundary: 65535 * 257 -> 16842495 >> 8 = 65790 > 65535 -> escalate.
    let mut r = cell80::Runner::compile(&cell_src("q_mul_checked")).unwrap();
    let report = r.run(None, &[65535, 257], DEFAULT_CYCLES).unwrap();
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // Domain extreme: 65535 * 65535 -> 4294836225 >> 8 = 16776704, far past u16::MAX -> escalate.
    let mut r = cell80::Runner::compile(&cell_src("q_mul_checked")).unwrap();
    let report = r.run(None, &[65535, 65535], DEFAULT_CYCLES).unwrap();
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn q_div_checked_matches_hand_computed_cases() {
    // q_div_checked: checked Q8.8 divide, (a<<8)/b at wide u32 width like q_div itself, but
    // escalates instead of silently truncating when the scaled quotient overflows u16.

    // Normal division, no overflow: 3.0 / 2.0 = 1.5 in Q8.8 -> (768<<8)/512 = 196608/512 = 384.
    // Same values as q_div's own existing test case.
    assert_eq!(run_cell("q_div_checked", &[768, 512]), 384);

    // Zero divisor: b == 0 -> 0, matching q_div's own zero-divisor convention (no halt).
    assert_eq!(run_cell("q_div_checked", &[768, 0]), 0);

    // Boundary that fits exactly: a=65535, b=256 -> (65535<<8)/256 = 16776960/256 = 65535
    // (u16::MAX exactly) -> no halt.
    assert_eq!(run_cell("q_div_checked", &[65535, 256]), 65535);

    // Overflow just past the boundary: a=256, b=1 -> (256<<8)/1 = 65536 > 0xFFFF -> halt
    // (0xFF05, needs_wider_math) instead of silently truncating to 0.
    let mut r = Runner::compile(&cell_src("q_div_checked")).unwrap();
    let report = r.run(None, &[256, 1], DEFAULT_CYCLES).unwrap();
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));

    // Overflow further past the boundary: a=65535, b=255 -> (65535<<8)/255 = 16776960/255 =
    // 65792 > 0xFFFF -> halt.
    let mut r2 = Runner::compile(&cell_src("q_div_checked")).unwrap();
    let report2 = r2.run(None, &[65535, 255], DEFAULT_CYCLES).unwrap();
    assert_eq!(report2.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn q_bilerp_bilinear_interpolation_matches_hand_computed_cases() {
    // q_bilerp: bilinear interpolation of four Q8.8 corners (q00,q10,q01,q11) by two Q0.8
    // fractions (tx, ty) -- lerp(lerp(q00,q10,tx), lerp(q01,q11,tx), ty), each 1D step using
    // q_lerp's own a+/-diff*t>>8 technique inlined three times (top edge, bottom edge, across).
    fn q_bilerp(q00: u16, q10: u16, q01: u16, q11: u16, tx: u16, ty: u16) -> u16 {
        let mut cell = StateCell::bind(&cell_src("q_bilerp"), "QBilerp", None).unwrap();
        cell.set("q00", q00 as u64).unwrap();
        cell.set("q10", q10 as u64).unwrap();
        cell.set("q01", q01 as u64).unwrap();
        cell.set("q11", q11 as u64).unwrap();
        cell.set("tx", tx as u64).unwrap();
        cell.set("ty", ty as u64).unwrap();
        cell.run(DEFAULT_CYCLES).unwrap();
        cell.get("out").unwrap() as u16
    }

    // Flat in y (q00==q01==0, q10==q11==256), tx=0.5, ty=0.0: both edges lerp to 128,
    // then ty=0 keeps the 'a' side -> 128.
    assert_eq!(q_bilerp(0, 256, 0, 256, 128, 0), 128);

    // tx=0 (left edge): top=q00=0, bottom=q01=256; ty=1.0 selects bottom -> 256.
    assert_eq!(q_bilerp(0, 256, 256, 256, 0, 256), 256);

    // General midpoint: top=lerp(100,200,0.5)=150, bottom=lerp(300,500,0.5)=400,
    // out=lerp(150,400,0.5)=150+((250*128)>>8)=150+125=275.
    assert_eq!(q_bilerp(100, 200, 300, 500, 128, 128), 275);

    // Reverse branches (b < a on both edges), tx=0.25, ty=0.75: top=200-25=175,
    // bottom=500-50=450, out=175+((275*192)>>8)=175+206=381 (52800>>8 truncates to 206).
    assert_eq!(q_bilerp(200, 100, 500, 300, 64, 192), 381);

    // tx=256 (1.0), ty=256 (1.0): both fractions saturate to the 'b' side -> exactly q11.
    assert_eq!(q_bilerp(10, 20, 30, 40, 256, 256), 40);
}

#[test]
fn q_to_int_i16_signed_q8_8_decode_matches_hand_computed_cases() {
    // q_to_int_i16: decode a signed Q8.8 value back to a plain integer via arithmetic
    // (sign-propagating) right shift by 8 -- int_to_q8_i16's missing decode counterpart.
    // Args/results are passed as raw u16 bit patterns of the underlying i16, per this
    // pack's existing signed-cell test convention (see q_mul_i16 / q_div_i16 above).

    // 0.0 -> 0
    assert_eq!(run_cell("q_to_int_i16", &[0]), 0);
    // 1.0 (256 in Q8.8) -> 1
    assert_eq!(run_cell("q_to_int_i16", &[256]), 1);
    // 127.0 (32512 in Q8.8, the int_to_q8_i16 boundary) -> 127
    assert_eq!(run_cell("q_to_int_i16", &[32512]), 127);
    // -1.0 in Q8.8 (u16 bits 65536-256=65280) -> -1, as u16 bits 65535 (0xFFFF).
    // A logical shift (like high_byte) would instead give 65280>>8 = 255 -- wrong.
    assert_eq!(run_cell("q_to_int_i16", &[65280]), 65535);
    // i16::MIN (32768 as u16 bits, -32768 in Q8.8 = -128.0 exactly) -> -128, u16 bits 65408
    assert_eq!(run_cell("q_to_int_i16", &[32768]), 65408);
    // -300 in Q8.8 (u16 bits 65536-300=65236), not a multiple of 256: must floor toward
    // -infinity, -300/256 = -1.171875 -> floor is -2, as u16 bits 65534.
    assert_eq!(run_cell("q_to_int_i16", &[65236]), 65534);
}

#[test]
fn q_mul3_triple_q8_8_multiply_matches_hand_computed_cases() {
    // q_mul3: chains two q_mul-style widen-shift steps -- step1 = (a*b)>>8, then
    // result = (step1*c)>>8 -- the 3-arg generalization of q_mul, which has no such sibling.

    // 1.0 * 1.0 * 1.0 = 1.0: step1 = (256*256)>>8 = 256; result = (256*256)>>8 = 256.
    assert_eq!(run_cell("q_mul3", &[256, 256, 256]), 256);

    // 1.5 * 2.0 * 2.0 = 6.0: step1 = (384*512)>>8 = 768 (== q_mul's own 1.5*2.0 case);
    // result = (768*512)>>8 = 1536 (6.0 in Q8.8).
    assert_eq!(run_cell("q_mul3", &[384, 512, 512]), 1536);

    // zero propagates through both stages: 0 * 500 * 700 = 0.
    assert_eq!(run_cell("q_mul3", &[0, 500, 700]), 0);

    // multiplying by 1.0 twice is a passthrough: step1 = (256*300)>>8 = 300;
    // result = (300*256)>>8 = 300.
    assert_eq!(run_cell("q_mul3", &[256, 300, 256]), 300);

    // general case, cross-checked against chaining q_mul by hand:
    // step1 = (200*300)>>8 = 60000>>8 = 234; result = (234*400)>>8 = 93600>>8 = 365.
    assert_eq!(run_cell("q_mul3", &[200, 300, 400]), 365);
}
