//! Host-oracle tests for the signed-deltas pack (`cell80/cells/signed-deltas/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::{cell_src, run_cell};

#[test]
fn signed_delta_free_fn_cells_match_defined_behaviour() {
    // The signed-deltas pack (library-growth.md "Next waves") — the library's first cells
    // over `i16`, now that the dialect supports it. Negative arguments/results are passed
    // and read as their two's-complement `u16` bit pattern (`-5` <-> `65531`), the same
    // convention `run_cell`'s raw-register interface uses throughout this file.
    assert_eq!(run_cell("sign_i16", &[5]), 1);
    assert_eq!(run_cell("sign_i16", &[65531]), 65535); // -5 -> -1
    assert_eq!(run_cell("sign_i16", &[0]), 0);

    assert_eq!(run_cell("abs_i16", &[5]), 5);
    assert_eq!(run_cell("abs_i16", &[65531]), 5); // -5 -> 5
    assert_eq!(run_cell("abs_i16", &[32768]), 32768); // i16::MIN -> 32768 (doesn't fit i16)

    // clamp_i16(x, lo, hi): lo=-10 (65526), hi=10.
    assert_eq!(run_cell("clamp_i16", &[5, 65526, 10]), 5); // within range, unchanged
    assert_eq!(run_cell("clamp_i16", &[65516, 65526, 10]), 65526); // -20 clamped up to -10
    assert_eq!(run_cell("clamp_i16", &[20, 65526, 10]), 10); // 20 clamped down to 10

    // apply_delta_clamped(value, delta, cap): a bounded resource/health adjustment.
    assert_eq!(run_cell("apply_delta_clamped", &[50, 20, 100]), 70);
    assert_eq!(run_cell("apply_delta_clamped", &[90, 20, 100]), 100); // clamped at cap
    assert_eq!(run_cell("apply_delta_clamped", &[50, 65516, 100]), 30); // delta -20
    assert_eq!(run_cell("apply_delta_clamped", &[10, 65516, 100]), 0); // clamped at 0
    assert_eq!(run_cell("apply_delta_clamped", &[100, 0, 100]), 100);
}

#[test]
fn negate_i16_matches_defined_behaviour() {
    // negate_i16(x): arithmetic negation -x, escalating (needs_wider_math) at i16::MIN
    // since 32768 (its magnitude) has no representation in i16. Args/results are read
    // as their two's-complement u16 bit pattern (-5 <-> 65531), matching this file's
    // other signed-deltas cases.
    assert_eq!(run_cell("negate_i16", &[5]), 65531); // -5 -> -5 (65531)
    assert_eq!(run_cell("negate_i16", &[65531]), 5); // -(-5) -> 5
    assert_eq!(run_cell("negate_i16", &[0]), 0); // -0 -> 0
    assert_eq!(run_cell("negate_i16", &[32767]), 32769); // -(i16::MAX) -> -32767 (32769)

    // i16::MIN (bits 32768) must escalate: needs_wider_math (halt 0xFF05).
    let mut r = cell80::Runner::compile(&cell_src("negate_i16")).unwrap();
    let report = r.run(None, &[32768], cell80::DEFAULT_CYCLES).unwrap();
    assert_eq!(report.halt, cell80::Halt::Escalate(0xFF05));
}

#[test]
fn abs_diff_i16_matches_hand_computed_expectations() {
    // abs_diff_i16(a, b): |a - b| for two signed i16 inputs, returned as u16. Negative
    // arguments are passed/read as their two's-complement u16 bit pattern, the same
    // convention run_cell's raw-register interface uses throughout this file.

    // The overflow case this cell exists for: a=i16::MAX (32767), b=i16::MIN (-32768,
    // 65536 as u16 bits). |32767 - (-32768)| = 65535, exactly u16::MAX -- a raw i16
    // subtract would overflow i16 by one before abs() could be taken.
    assert_eq!(run_cell("abs_diff_i16", &[32767, 32768]), 65535);

    // Both positive: |5 - 3| = 2, and order-independent: |3 - 5| = 2.
    assert_eq!(run_cell("abs_diff_i16", &[5, 3]), 2);
    assert_eq!(run_cell("abs_diff_i16", &[3, 5]), 2);

    // Both negative: |-5 - (-3)| = |-2| = 2. -5 -> 65531, -3 -> 65533.
    assert_eq!(run_cell("abs_diff_i16", &[65531, 65533]), 2);

    // Mixed sign: |-5 - 3| = 8. -5 -> 65531.
    assert_eq!(run_cell("abs_diff_i16", &[65531, 3]), 8);

    // Zero vs negative: |0 - (-5)| = 5. -5 -> 65531.
    assert_eq!(run_cell("abs_diff_i16", &[0, 65531]), 5);

    // Equal values: |7 - 7| = 0.
    assert_eq!(run_cell("abs_diff_i16", &[7, 7]), 0);
}

#[test]
fn min_i16_matches_hand_computed_expectations() {
    // Both positive: min(5, 3) = 3, ordinary case.
    assert_eq!(run_cell("min_i16", &[5, 3]), 3);

    // Both negative: min(-5, -3) = -5 (65531 as u16 bits) -- true signed ordering,
    // not a bit-magnitude compare (which would wrongly call -5's bits the larger one).
    assert_eq!(run_cell("min_i16", &[65531, 65533]), 65531); // -5, -3 -> -5

    // Mixed sign: min(-1, 0) = -1 (65535) -- the case a naive u16 compare gets
    // backwards, since -1's bit pattern (65535) looks larger than 0's.
    assert_eq!(run_cell("min_i16", &[65535, 0]), 65535);

    // Mixed sign, operands swapped: min(2, -2) = -2 (65534).
    assert_eq!(run_cell("min_i16", &[2, 65534]), 65534);

    // Equal values: min(7, 7) = 7 (either operand, tie is a no-op).
    assert_eq!(run_cell("min_i16", &[7, 7]), 7);
}

#[test]
fn max_i16_matches_hand_computed_expectations() {
    // The signed sibling of max (u16)/max_u32 and the direct complement of min_i16:
    // true signed ordering, not unsigned bit-pattern ordering.
    assert_eq!(run_cell("max_i16", &[5, 3]), 5); // both positive: 5 > 3
    assert_eq!(run_cell("max_i16", &[65531, 65533]), 65533); // -5 vs -3 -> -3 is larger
    assert_eq!(run_cell("max_i16", &[65535, 1]), 1); // -1 vs 1 -> 1 wins (unsigned would wrongly pick 65535)
    assert_eq!(run_cell("max_i16", &[42, 42]), 42); // tie
    assert_eq!(run_cell("max_i16", &[32768, 32767]), 32767); // i16::MIN vs i16::MAX -> MAX wins
}

#[test]
fn lerp_i16_matches_hand_computed_expectations() {
    // q_lerp's signed sibling, the long-open "overflow safety not yet worked out"
    // blocker: b - a can exceed i16's own representable range even when a and b are
    // both valid i16 values, so it's computed via sign-magnitude throughout, never a
    // native i16 subtraction.
    assert_eq!(run_cell("lerp_i16", &[65526, 10, 128]), 0); // a=-10, b=10, t=0.5 -> 0
    assert_eq!(run_cell("lerp_i16", &[100, 65486, 64]), 63); // a=100, b=-50, t=0.25 -> 63 (truncated toward zero)
    assert_eq!(run_cell("lerp_i16", &[50, 65486, 0]), 50); // t=0 -> a unchanged
    assert_eq!(run_cell("lerp_i16", &[50, 65486, 256]), 65486); // t=256 -> b exactly
                                                                 // a=i16::MAX, b=i16::MIN: diff magnitude 65535, itself not representable as i16.
    assert_eq!(run_cell("lerp_i16", &[32767, 32768, 128]), 0);
}
