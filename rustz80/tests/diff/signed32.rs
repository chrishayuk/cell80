//! The i32 corpus (Phase 5 A3, `docs/13-multi-target-spec.md` §3 WS-A): signed-32
//! semantics under the rustc oracle **on the reference IR interpreter** — the
//! pre-registered acceptance shape, "before any backend emits it". Every signed-32
//! op is gated out of Z80 codegen with an instructive error (pinned below);
//! bit-identical signed ops (add/sub/mul/bitwise) compile everywhere as before.

#[test]
fn i32_locals_literals_and_arithmetic() {
    check_ir!({
        let a = -100_000i32;
        let b = 3i32;
        ((a / b) >> 8) as u16
    });
    check_ir!({
        let a = 2_000_000_000i32;
        let b = 2_000_000_000i32;
        let s = a.wrapping_add(b); // wraps negative (bare ops wrap in the dialect,
                                   // but the debug-built oracle needs the spelling)
        (s < 0) as u16
    });
    check_ir!({
        let x = -7i32;
        let y = 2i32;
        ((x % y) == -1i32) as u16
    });
}

#[test]
fn i32_division_truncates_toward_zero() {
    // The four sign quadrants + the MIN/1 corner (MIN/-1 panics in rustc — out of
    // the oracle's domain, like every divide-by-zero).
    check_ir!({
        let a = -7i32 / 2i32;
        let b = 7i32 / -2i32;
        let c = -7i32 / -2i32;
        let d = 7i32 / 2i32;
        ((a == -3i32) as u16)
            + ((b == -3i32) as u16) * 10
            + ((c == 3i32) as u16) * 100
            + ((d == 3i32) as u16) * 1000
    });
    check_ir!({
        let m = -2_147_483_648i32;
        let one = 1i32;
        (m / one == m) as u16
    });
    check_ir!({
        let a = -7i32 % 2i32;
        let b = 7i32 % -2i32;
        ((a == -1i32) as u16) + ((b == 1i32) as u16) * 10
    });
}

#[test]
fn i32_comparisons_order_by_twos_complement() {
    check_ir!({
        let a = -1i32;
        let b = 1i32;
        ((a < b) as u16)
            + ((a <= b) as u16) * 10
            + ((b > a) as u16) * 100
            + ((a >= b) as u16) * 1000
            + ((a == a) as u16) * 10000
    });
    check_ir!({
        // The unsigned reading would invert these: 0xFFFF_FFFF > 1 as u32.
        let big_neg = -2_147_483_648i32;
        let small_pos = 1i32;
        (big_neg < small_pos) as u16
    });
    check_ir!({
        let x = -5i32;
        if x < 0i32 {
            111
        } else {
            222
        }
    });
}

#[test]
fn i32_arithmetic_shift_right() {
    check_ir!({
        let x = -256i32;
        ((x >> 4) == -16i32) as u16
    });
    check_ir!({
        let x = -1i32;
        ((x >> 31) == -1i32) as u16 // sign fill, not zero
    });
    check_ir!({
        let x = -8i32;
        ((x << 2) == -32i32) as u16
    });
}

#[test]
fn i32_casts_and_bridges() {
    check_ir!({
        let x = -3i16;
        let w = x as i32; // sign-extend (the A2 bridge, signed target)
        (w == -3i32) as u16
    });
    check_ir!({
        let x = 40_000u16;
        let w = x as i32; // zero-extend: a u16 is non-negative
        (w == 40_000i32) as u16
    });
    check_ir!({
        let x = -1i32;
        (x as u32 == 4_294_967_295u32) as u16 // bit-identity, unsigned reading
    });
    check_ir!({
        let x = 4_294_967_295u32;
        (x as i32 == -1i32) as u16 // and back
    });
    check_ir!({
        let x = -70_000i32;
        x as u16 // truncate to the low word
    });
    check_ir!({
        let x = -1i32;
        (x as u8) as u16
    });
}

#[test]
fn i32_params_returns_and_calls() {
    // The wide call convention carries i32 like u32 (same two-slot storage).
    let src = "
        fn scale(x: i32, k: u16) -> i32 { x * (k as i32) }
        fn run() -> u16 {
            let a = scale(-300i32, 4);
            (a == -1200i32) as u16
        }
    ";
    let out = rustz80::interp_program(src, "run").unwrap();
    assert_eq!(out[0], 1);
    // Two i32 params (the gcd shape).
    let src = "
        fn diff(a: i32, b: i32) -> i32 { a - b }
        fn run() -> u16 { (diff(-5i32, 3i32) == -8i32) as u16 }
    ";
    let out = rustz80::interp_program(src, "run").unwrap();
    assert_eq!(out[0], 1);
}

#[test]
fn signed32_is_gated_out_of_codegen_instructively() {
    // Ops whose signedness changes the bits: refused with the WS-B pointer.
    for src in [
        "fn f() -> u16 { let x = -5i32; (x < 0i32) as u16 }",
        "fn f() -> u16 { (-7i32 / 2i32) as u16 }",
        "fn f() -> u16 { (-256i32 >> 4) as u16 }",
    ] {
        for target in crate::harness::TARGETS {
            let e = rustz80::compile_fn_for(src, target).unwrap_err();
            assert!(
                e.contains("reference interpreter") && e.contains("WS-B"),
                "gate message drifted: {e}"
            );
        }
    }
    // Bit-identical signed ops (add/sub/mul/bitwise/left-shift/casts) still
    // compile and agree everywhere — the unsigned patterns carry them.
    check!({
        let a = -5i32;
        let b = 3i32;
        ((a + b) as u16) ^ ((a * b) as u16) ^ ((a - b) as u16) ^ ((a << 2) as u16)
    });
}
