//! Phase 1.4 — signed `i16`: two's-complement add/sub/mul share the unsigned bit
//! patterns; comparisons order by sign (S ⊕ V), divide truncates toward zero with the
//! remainder taking the dividend's sign, and `>>` is arithmetic. Every case runs the
//! sign boundary (−1, i16::MIN, mixed signs) against the rustc oracle on both targets.
//! Results return as bits (`as u16`) — the register convention is width-agnostic.

use crate::harness::*;

#[test]
fn i16_arithmetic_and_negation() {
    check!({
        let a = -5i16;
        let b = 12i16;
        (a + b) as u16
    }); // 7
    check!({
        let a = -5i16;
        let b = 12i16;
        (a - b) as u16
    }); // -17 → 0xFFEF
    check!({
        let a = -3i16;
        let b = -7i16;
        (a * b) as u16
    }); // 21 — product of negatives
    check!({
        let a = 1000i16;
        let b = -3i16;
        a.wrapping_mul(b) as u16
    }); // -3000
    check!({
        let x = 42i16;
        (-x) as u16
    }); // runtime negation
    check!({
        let m = -32768i16;
        m.wrapping_sub(1i16) as u16
    }); // i16::MIN - 1 wraps to i16::MAX
}

#[test]
fn i16_comparisons_order_by_sign() {
    // The whole point: unsigned compare says 0xFFFF > 1; signed must say -1 < 1.
    check!({
        let a = -1i16;
        let b = 1i16;
        (a < b) as u16
    }); // 1
    check!({
        let a = -1i16;
        let b = 1i16;
        (a > b) as u16
    }); // 0
    check!({
        let m = -32768i16;
        let p = 32767i16;
        (m < p) as u16
    }); // MIN < MAX — the overflow (V=1) compare case
    check!({
        let a = -100i16;
        let b = -7i16;
        (a <= b) as u16
    }); // both negative
    check!({
        let a = -5i16;
        let b = -5i16;
        (a >= b) as u16
    }); // equality boundary on >=
        // …and in branch position (gen_cond_skip's signed path).
    check!({
        let t = -10i16;
        let mut r = 0u16;
        if t < 0i16 {
            r = 1u16;
        } else {
            r = 2u16;
        }
        r
    });
    check!({
        let mut s = 0i16;
        let mut i = -3i16;
        while i < 3i16 {
            s = s + i;
            i = i + 1i16;
        }
        s as u16
    }); // a signed loop crossing zero
}

#[test]
fn i16_div_rem_truncate_toward_zero() {
    // rustc semantics: -7/2 = -3 (toward zero), -7%2 = -1 (dividend's sign).
    check!({
        let a = -7i16;
        let b = 2i16;
        (a / b) as u16
    });
    check!({
        let a = -7i16;
        let b = 2i16;
        (a % b) as u16
    });
    check!({
        let a = 7i16;
        let b = -2i16;
        (a / b) as u16
    }); // -3
    check!({
        let a = 7i16;
        let b = -2i16;
        (a % b) as u16
    }); // +1
    check!({
        let a = -7i16;
        let b = -2i16;
        (a / b) as u16
    }); // +3
    check!({
        let m = -32768i16;
        let b = -1i16;
        m.wrapping_div(b) as u16
    }); // MIN / -1 wraps back to MIN
    check!({
        let a = -30000i16;
        let b = 7i16;
        ((a / b) * b + a % b) as u16
    }); // == a — the division identity, signed
}

#[test]
fn i16_arithmetic_shift_right() {
    check!({
        let a = -16i16;
        (a >> 2i16) as u16
    }); // -4 — the sign propagates
    check!({
        let a = -1i16;
        (a >> 8i16) as u16
    }); // still -1
    check!({
        let a = 1000i16;
        (a >> 3i16) as u16
    }); // positive: same as logical
    check!({
        let a = -16i16;
        (a << 2i16) as u16
    }); // shifts left are sign-agnostic bits
}

#[test]
fn i16_casts_are_bit_preserving() {
    check!({
        let u = 0xFFFEu16;
        let s = u as i16; // -2
        (s + 1i16) as u16
    }); // -1 → 0xFFFF
    check!({
        let b = 200u8;
        (b as i16) as u16
    }); // u8 zero-extends: 200
    check!({
        let s = -200i16;
        (s as u8) as u16
    }); // low byte of 0xFF38 = 0x38
}

#[test]
fn i16_params_locals_and_fields() {
    // i16 crosses calls, lives in arrays, and persists in struct fields as bits.
    fn host() -> u16 {
        struct P {
            dx: i16,
            dy: i16,
        }
        let p = P { dx: -3, dy: 4 };
        let deltas = [-2i16, 5i16, -1i16];
        let mut s = p.dx + p.dy; // 1
        for i in 0..3 {
            s = s + deltas[i];
        }
        s as u16 // 1 + 2 = 3
    }
    let src = "
        struct P { dx: i16, dy: i16 }
        fn bump(v: i16) -> i16 { v + 1i16 }
        fn run() -> u16 {
            let p = P { dx: -3i16, dy: 4i16 };
            let deltas = [-2i16, 5i16, -1i16];
            let mut s = p.dx + p.dy;
            let mut i = 0u16;
            while i < 3u16 { s = s + deltas[i as usize]; i = i + 1u16; }
            s = bump(s) - 1i16;
            s as u16
        }
    ";
    let prog_result = run_program(src, "run");
    assert_eq!(prog_result, host());
}

#[test]
fn i16_if_expression() {
    // The Phase-1 features compose: a signed conditional value.
    check!({
        let d = -8i16;
        let mag = if d < 0i16 { -d } else { d };
        mag as u16
    }); // |−8| = 8
}
