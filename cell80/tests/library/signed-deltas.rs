//! Host-oracle tests for the signed-deltas pack (`cell80/cells/signed-deltas/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::run_cell;

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
