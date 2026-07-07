//! Host-oracle tests for the stateful-rng pack (`cell80/cells/stateful-rng/*.rs`). Split from the
//! former monolithic `cell80/tests/library.rs` (2026-07-07) to mirror the cells' own
//! pack-directory structure; see `cell80/tests/library/common.rs` for the shared
//! `cell_src`/`run_cell` helpers every pack file uses.

use crate::common::cell_src;
use cell80::{StateCell, DEFAULT_CYCLES};

#[test]
fn stateful_rng_cells_match_defined_behaviour() {
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> StateCell {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap();
        cell
    }

    // lcg_next: seed = seed * 1664525 + 1013904223 (mod 2^32), top 16 bits returned —
    // matches the classic Numerical Recipes LCG exactly (checked against a reference
    // computation), not just "changes each step."
    let mut seed = 42u64;
    let expect = [1083814273u64, 378494188, 2479403867, 955863294];
    for want_seed in expect {
        let cell = step("lcg_next", "Lcg", &[("seed", seed)]);
        seed = cell.get("seed").unwrap();
        assert_eq!(seed, want_seed);
    }

    // xorshift16: a distinct recurrence from lcg_next (shift/xor, no multiply); a zero
    // seed is a documented fixed point (stays 0 forever).
    let cell = step("xorshift16", "Xorshift16", &[("x", 1)]);
    let x1 = cell.get("x").unwrap();
    assert_ne!(x1, 0);
    let cell = step("xorshift16", "Xorshift16", &[("x", x1)]);
    let x2 = cell.get("x").unwrap();
    assert_ne!(x2, x1); // genuinely advances, not a fixed point at a nonzero seed
    let cell = step("xorshift16", "Xorshift16", &[("x", 0)]);
    assert_eq!(cell.get("x"), Some(0)); // documented zero-seed degeneracy

    // counter_step: wraps to 0 the instant it would reach `limit`; limit 0 disables
    // wrapping (up to the native u16 boundary, which is out of the cell's control).
    let mut count = 0u64;
    for want in [1u64, 2, 0, 1, 2, 0] {
        let cell = step(
            "counter_step",
            "CounterStep",
            &[("count", count), ("limit", 3)],
        );
        count = cell.get("count").unwrap();
        assert_eq!(count, want);
    }
    let cell = step(
        "counter_step",
        "CounterStep",
        &[("count", 65534), ("limit", 0)],
    );
    assert_eq!(cell.get("count"), Some(65535)); // no wrap when limit == 0
    // The stateful/RNG pack (library-growth.md "Next waves") — deterministic pseudo-random
    // steps. `StateCell::run` zeros memory the previous run touched (Runner::run's own
    // doc), so the carried field must be re-`set` from the prior `get` before every call —
    // there's no implicit persistence across separate `.run()` invocations.

}
