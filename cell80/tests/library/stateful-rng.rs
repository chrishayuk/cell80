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

// xorshift32: a full 32-bit-state xorshift recurrence (x ^= x<<13; x ^= x>>17; x ^= x<<5,
// Marsaglia's classic constants) — distinct from xorshift16 (different shifts, a wider
// 2^32-1 period) and from lcg_next (no multiply, pure shift/xor). Values below were hand-
// computed independently in Python against the exact same recurrence, mod 2^32.
#[test]
fn xorshift32_matches_hand_computed_recurrence() {
    fn step(id: &str, strct: &str, x: u64) -> (u16, u64) {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        cell.set("x", x).unwrap();
        let report = cell
            .run(DEFAULT_CYCLES)
            .unwrap_or_else(|e| panic!("run {id}: {e}"));
        assert_eq!(
            report.halt,
            cell80::Halt::Returned,
            "unexpected halt: {:?}",
            report.halt
        );
        (report.result, cell.get("x").unwrap())
    }

    // seed=1 -> full state 0x00042021, top16 = 4
    let (top, full) = step("xorshift32", "Xorshift32", 1);
    assert_eq!(full, 0x00042021);
    assert_eq!(top, 4);

    // seed=2463534242 (0x92d68ca2, Marsaglia's usual xorshift32 demo seed) -> 0x2b1f4d63, top16 = 11039
    let (top, full) = step("xorshift32", "Xorshift32", 2463534242);
    assert_eq!(full, 0x2b1f4d63);
    assert_eq!(top, 11039);

    // seed=0xFFFFFFFF (all-ones) -> 0x0003e01f, top16 = 3
    let (top, full) = step("xorshift32", "Xorshift32", 0xFFFFFFFF);
    assert_eq!(full, 0x0003e01f);
    assert_eq!(top, 3);

    // two-step chain from seed=1 genuinely advances state across calls (not a fixed point):
    // step1 -> 0x00042021 (top16=4), step2 -> 0x04080601 (top16=1032).
    let (top1, x1) = step("xorshift32", "Xorshift32", 1);
    assert_eq!((top1, x1), (4, 0x00042021));
    let (top2, x2) = step("xorshift32", "Xorshift32", x1);
    assert_eq!((top2, x2), (1032, 0x04080601));

    // documented zero-seed fixed point: 0 stays 0 forever.
    let (top, full) = step("xorshift32", "Xorshift32", 0);
    assert_eq!((top, full), (0, 0));
}

#[test]
fn counter_step_u32_matches_hand_computed_values() {
    // Checks CounterStepU32 against hand-computed expectations: (A) the same wrap cadence
    // as counter_step but through u32 fields, (B)/(C) values that sail straight past
    // u16::MAX and up toward u32::MAX with limit=0 (never wrap) -- the exact range
    // counter_step's u16 fields cannot represent, proving the "wide sibling" claim, and
    // (D) wrapping exactly at a limit that itself sits beyond the u16 ceiling.
    fn step(fields: &[(&str, u64)]) -> (u16, StateCell) {
        let mut cell = StateCell::bind(&cell_src("counter_step_u32"), "CounterStepU32", None)
            .unwrap_or_else(|e| panic!("bind: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        let result = cell.run(DEFAULT_CYCLES).unwrap().result;
        (result, cell)
    }

    // Case A: small limit=3, mirrors counter_step's own wrap cadence (1,2,0,1,...) but
    // through the u32 struct/entry point.
    let mut count = 0u64;
    let mut got = Vec::new();
    for _ in 0..4 {
        let (flag, cell) = step(&[("count", count), ("limit", 3)]);
        assert_eq!(flag, 1, "run() always signals success via the 1u16 flag");
        count = cell.get("count").unwrap();
        got.push(count);
    }
    assert_eq!(got, vec![1, 2, 0, 1]);

    // Case B: limit=0 disables wrapping, and count sails straight past 65535 -- the exact
    // ceiling counter_step (u16) cannot cross.
    let (flag, cell) = step(&[("count", 65535u64), ("limit", 0u64)]);
    assert_eq!(flag, 1);
    assert_eq!(
        cell.get("count"),
        Some(65536),
        "count exceeds u16::MAX with limit=0"
    );
    let (flag, cell) = step(&[("count", 65536u64), ("limit", 0u64)]);
    assert_eq!(flag, 1);
    assert_eq!(cell.get("count"), Some(65537));

    // Case C: near u32::MAX, still limit=0 (never wrap).
    let (flag, cell) = step(&[("count", 4294967294u64), ("limit", 0u64)]);
    assert_eq!(flag, 1);
    assert_eq!(
        cell.get("count"),
        Some(4294967295),
        "reaches u32::MAX with limit=0"
    );

    // Case D: wraps exactly at a limit that itself sits beyond the u16 ceiling.
    let (flag, cell) = step(&[("count", 99999u64), ("limit", 100001u64)]);
    assert_eq!(flag, 1);
    assert_eq!(
        cell.get("count"),
        Some(100000),
        "no wrap yet, one below limit"
    );
    let (flag, cell) = step(&[("count", 100000u64), ("limit", 100001u64)]);
    assert_eq!(flag, 1);
    assert_eq!(
        cell.get("count"),
        Some(0),
        "wraps to 0 the instant it reaches limit"
    );
}

// pingpong_step: bounces `pos` between 0 and `limit`, reversing direction the instant a
// bound is touched (checked *before* the step, so no u16 underflow at the bottom), instead
// of wrapping like counter_step. `dir` 0=increasing, 1=decreasing.
#[test]
fn pingpong_step_bounces_between_zero_and_limit() {
    fn step(id: &str, strct: &str, fields: &[(&str, u64)]) -> StateCell {
        let mut cell = StateCell::bind(&cell_src(id), strct, None)
            .unwrap_or_else(|e| panic!("bind {id}: {e}"));
        for (f, v) in fields {
            cell.set(f, *v).unwrap();
        }
        cell.run(DEFAULT_CYCLES).unwrap();
        cell
    }

    // limit=3 from pos=0/dir=0 (increasing): classic triangle wave up to the top bound,
    // reverse, back down to the bottom bound, reverse again.
    let mut pos = 0u64;
    let mut dir = 0u64;
    let expect = [
        (1u64, 0u64), // 0 -> 1
        (2, 0),       // 1 -> 2
        (3, 0),       // 2 -> 3 (now at the top bound)
        (2, 1),       // reversal seen on this call: flip to decreasing, then step down
        (1, 1),       // 2 -> 1
        (0, 1),       // 1 -> 0 (now at the bottom bound)
        (1, 0),       // reversal seen: flip to increasing, then step up
    ];
    for (want_pos, want_dir) in expect {
        let cell = step(
            "pingpong_step",
            "PingPong",
            &[("pos", pos), ("dir", dir), ("limit", 3)],
        );
        pos = cell.get("pos").unwrap();
        dir = cell.get("dir").unwrap();
        assert_eq!((pos, dir), (want_pos, want_dir));
    }

    // limit=0 is degenerate (a zero-width range): pos must stay pinned at 0 forever, never
    // underflowing u16, even though dir keeps flipping internally each call.
    let cell = step(
        "pingpong_step",
        "PingPong",
        &[("pos", 0), ("dir", 0), ("limit", 0)],
    );
    assert_eq!(cell.get("pos"), Some(0));
    assert_eq!(cell.get("dir"), Some(1));
    let cell = step(
        "pingpong_step",
        "PingPong",
        &[("pos", 0), ("dir", 1), ("limit", 0)],
    );
    assert_eq!(cell.get("pos"), Some(0));
    assert_eq!(cell.get("dir"), Some(0));
}
