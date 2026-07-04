//! The 32-bit lane: widen/narrow casts, bitwise + shifts, and the full
//! add/sub/mul/div/rem arithmetic (software carry-chain + runtime routines).

use crate::harness::*;

#[test]
fn widen_to_u32() {
    // `x as u32` zero-extends into the high word, so a widened `u16` can feed the `u32` bit/shift
    // ops and truncate back. The `>>` cases would drag garbage down if the high word weren't 0.
    check!({
        let x = 43981u16; // 0xABCD
        ((x as u32) >> 8) as u16
    });
    check!({
        let x = 50000u16;
        ((x as u32) >> 4) as u16
    });
    check!({
        let x = 60000u16;
        ((x as u32) & 65535u32) as u16
    });
    check!({
        // widen → left shift fills the high word → shift back → truncate: round-trips to x.
        let x = 700u16;
        (((x as u32) << 8) >> 8) as u16
    });
}

#[test]
fn u32_xorshift() {
    // A real 32-bit xorshift step (the SDK `Rng` core) — `u32` locals, `^`, and
    // constant `<<` / `>>` (including a shift past the word boundary, 17). The low 16
    // bits are returned. Same source under rustc.
    fn host() -> u16 {
        let mut x: u32 = 2463534242;
        x = x ^ (x << 13);
        x = x ^ (x >> 17);
        x = x ^ (x << 5);
        x as u16
    }
    let src = "
        fn run() -> u16 {
            let mut x: u32 = 2463534242u32;
            x = x ^ (x << 13u32);
            x = x ^ (x >> 17u32);
            x = x ^ (x << 5u32);
            x as u16
        }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn u32_bitwise_and_truncate() {
    // u32 `& | ^` across both words, with `as u16` / `as u8` truncation.
    fn host() -> u16 {
        let a: u32 = 0xDEADBEEF;
        let b: u32 = 0x0F0F0F0F;
        let c = (a & b) | (a ^ b);
        (c as u16) ^ ((c >> 16) as u16) ^ ((a as u8) as u16)
    }
    let src = "
        fn run() -> u16 {
            let a: u32 = 0xDEADBEEFu32;
            let b: u32 = 0x0F0F0F0Fu32;
            let c = (a & b) | (a ^ b);
            (c as u16) ^ ((c >> 16u32) as u16) ^ ((a as u8) as u16)
        }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn u32_add_sub() {
    // 32-bit add/sub: the carry/borrow must chain across the word boundary.
    check!({
        let a = 100000u32;
        let b = 45000u32;
        ((a + b) >> 16) as u16
    });
    check!({
        let a = 100000u32;
        let b = 65535u32;
        (a - b) as u16
    }); // low-word borrow into the high word
    check!({
        let a = 0xFFFFu32;
        let b = 1u32;
        ((a + b) >> 16) as u16
    }); // carry exactly at bit 16
    check!({
        let a = 5u32;
        let b = 7u32;
        (a.wrapping_sub(b) >> 16) as u16
    }); // wrap below zero
}

#[test]
fn u32_mul() {
    // Full 32-bit product (mod 2^32): cross partial products land in the high word.
    check!({
        let a = 70000u32;
        let b = 3u32;
        ((a * b) >> 16) as u16
    });
    check!({
        let a = 0x1234u32;
        let b = 0x5678u32;
        ((a * b) >> 16) as u16
    });
    check!({
        let a = 0x1234u32;
        let b = 0x5678u32;
        (a * b) as u16
    });
    check!({
        let a = 0x0002_0003u32;
        let b = 0x0004_0005u32;
        (a.wrapping_mul(b) >> 16) as u16
    }); // both high words set — wraps mod 2^32
    check!({
        let x = 300u16;
        ((x as u32 * x as u32) >> 8) as u16
    }); // the `square` shape: a widened u16 squared past 65535
}

#[test]
fn u32_div_rem() {
    check!({
        let a = 100000u32;
        let b = 7u32;
        (a / b) as u16
    });
    check!({
        let a = 100000u32;
        let b = 7u32;
        (a % b) as u16
    });
    check!({
        let a = 0xFFFF_FFFFu32;
        let b = 2u32;
        ((a / b) >> 12) as u16
    }); // maximum dividend
    check!({
        let a = 0xFFFF_FFF0u32;
        let b = 0x8000_0001u32;
        (a / b) as u16
    }); // divisor ≥ 2^31 — the forced-commit (33rd bit) path
    check!({
        let a = 0xFFFF_FFF0u32;
        let b = 0x8000_0001u32;
        (a % b) as u16
    });
    check!({
        let a = 5u32;
        let b = 100000u32;
        ((a / b) as u16) + ((a % b) as u16)
    }); // dividend < divisor: q = 0, rem = dividend
}

#[test]
fn u32_percent_shape() {
    // The percent/ratio family's exact shape: widen, multiply by an unsuffixed literal
    // (rustc infers it as u32 — the lowering zero-extends to match), divide, narrow.
    check!({
        let part = 700u16;
        let whole = 1000u16;
        (part as u32 * 100 / whole as u32) as u16
    }); // = 70 — the u16 version wraps to 4
    check!({
        let part = 65535u16;
        let whole = 65535u16;
        (part as u32 * 1000 / whole as u32) as u16
    }); // permille at the u16 extreme
    check!({
        let value = 1000u16;
        let pct = 200u16;
        (value as u32 * pct as u32 / 100) as u16
    }); // scale_percent(1000, 200) = 2000
}

#[test]
fn u32_struct_fields() {
    // A `u32` field: two little-endian slots, read/written wide through the `self`
    // pointer — the state-cell shape (`total` accumulates past 65535).
    struct S {
        n: u16,
        total: u32,
    }
    fn host() -> u16 {
        let mut s = S { n: 300, total: 0 };
        s.total = s.n as u32 * s.n as u32; // 90000 — past the u16 ceiling
        s.total = s.total + 70000;
        ((s.total >> 16) as u16) ^ (s.total as u16)
    }
    let src = "
        struct S { n: u16, total: u32 }
        impl S {
            fn run(&mut self) -> u16 {
                self.total = self.n as u32 * self.n as u32;
                self.total = self.total + 70000u32;
                ((self.total >> 16u32) as u16) ^ (self.total as u16)
            }
        }
        fn run() -> u16 {
            let mut s = S { n: 300u16, total: 0u32 };
            s.run()
        }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn u32_field_by_value_local() {
    // A by-value struct local with a wide field: `Var32`/`Assign32` slot addressing,
    // including a wide literal initialiser and a 16-bit value widening into the field.
    struct S {
        lo: u16,
        big: u32,
        hi: u16,
    }
    fn host() -> u16 {
        let mut s = S {
            lo: 3,
            big: 0x0001_0005,
            hi: 7,
        };
        s.big = s.big * 3;
        let w = s.lo; // the neighbours must survive the wide store
        s.big = s.big + w as u32;
        (s.big as u16) + ((s.big >> 16) as u16) + s.lo + s.hi
    }
    let src = "
        struct S { lo: u16, big: u32, hi: u16 }
        fn run() -> u16 {
            let mut s = S { lo: 3u16, big: 0x0001_0005u32, hi: 7u16 };
            s.big = s.big * 3u32;
            let w = s.lo;
            s.big = s.big + w as u32;
            (s.big as u16) + ((s.big >> 16u32) as u16) + s.lo + s.hi
        }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn u32_comparisons_as_conditions() {
    // Direct u32 comparisons in `if`/`while` — the word-split idiom retires.
    // The q_max shape the fixed-point pack hand-splits today:
    check!({
        let a = 100_000u32;
        let b = 70_000u32;
        let m = if a < b { b } else { a };
        (m >> 16) as u16 * 1000u16 + (m & 0xFFFFu32) as u16 / 100u16
    });
    // Boundary pairs around the high/low word seams, every operator.
    check!({
        let mut n = 0u16;
        let pairs_a = 65_536u32;
        let pairs_b = 65_535u32;
        if pairs_a > pairs_b {
            n = n + 1u16;
        }
        if pairs_b >= pairs_a {
            n = n + 10u16;
        }
        if pairs_a != pairs_b {
            n = n + 100u16;
        }
        if pairs_a - 1u32 == pairs_b {
            n = n + 1000u16;
        }
        n
    });
    // A wide loop guard: while over a u32 accumulator.
    check!({
        let mut total = 0u32;
        let mut steps = 0u16;
        while total < 300_000u32 {
            total = total + 70_000u32;
            steps = steps + 1u16;
        }
        steps
    });
    // Negated wide condition.
    check!({
        let x = 5u32;
        let mut r = 0u16;
        if !(x >= 100_000u32) {
            r = 7u16;
        }
        r
    });
}

#[test]
fn u32_comparisons_as_values() {
    // Materialised 0/1 — mixed-width sides widen (the annotated-literal shape).
    check!({
        let w = 100_000u32;
        (w > 65_535u32) as u16 * 100u16 + (w == 100_000u32) as u16 * 10u16 + (w <= 99_999u32) as u16
    });
    check!({
        let small = 42u16;
        let wide = 42u32;
        (wide == small as u32) as u16 + (wide < 43u32) as u16 * 10u16
    });
}

#[test]
fn u32_saturating() {
    // saturating_add/_sub ride the new wide compare; both clamped and clean sides.
    check!({
        let a = 0xFFFF_0000u32;
        let b = 0x2_0000u32;
        (a.saturating_add(b) >> 24) as u16 + (a.saturating_sub(b) >> 24) as u16 * 10u16
    });
    check!({
        let a = 70_000u32;
        (a.saturating_sub(80_000u32) == 0u32) as u16 * 10u16
            + (a.saturating_add(1u32) == 70_001u32) as u16
    });
}

#[test]
fn u32_local_arrays() {
    // `[u32; N]` locals — repeat and literal init, runtime-index read/write,
    // a wide accumulate loop. Checked against rustc.
    check!({
        let mut a = [0u32; 4];
        a[0] = 100_000u32;
        a[3] = a[0] + 30_000u32;
        let i = 3u16;
        ((a[i as usize] - a[0]) >> 8) as u16
    });
    check!({
        let a = [70_000u32, 5u32, 0xFFFF_FFFFu32];
        (a[2] >> 24) as u16 + (a[0] > a[1]) as u16 * 10u16 + (a[0] & 0xFFu32) as u16
    });
    check!({
        // Sliding wide accumulator — the running-statistics pack shape.
        let mut win = [0u32; 3];
        let mut total = 0u32;
        for i in 0..9u16 {
            let slot = i % 3u16;
            total = total - win[slot as usize] + (i as u32) * 40_000u32;
            win[slot as usize] = (i as u32) * 40_000u32;
        }
        (total >> 16) as u16
    });
}

#[test]
fn u32_array_state_fields() {
    // `[u32; N]` struct fields — by value and through `&mut self`, with a field
    // after the array proving the 2N-slot offset arithmetic.
    let src = "
        struct W { hist: [u32; 3], n: u16 }
        impl W {
            fn bump(&mut self, i: u16, k: u16) {
                self.hist[i] = self.hist[i] + (k as u32) * 35_000u32;
                self.n = self.n + 1u16;
            }
            fn run(&mut self) -> u16 {
                self.bump(0u16, 2u16);
                self.bump(2u16, 2u16);
                self.bump(2u16, 0u16);
                (self.hist[2] >> 16) as u16 * 100u16 + self.n * 10u16
                    + (self.hist[2] > self.hist[0]) as u16
            }
        }
        fn run() -> u16 {
            let mut w = W { hist: [0u32; 3], n: 0u16 };
            w.run()
        }
    ";
    struct W {
        hist: [u32; 3],
        n: u16,
    }
    impl W {
        fn bump(&mut self, i: u16, k: u16) {
            self.hist[i as usize] += (k as u32) * 35_000;
            self.n += 1;
        }
        fn run(&mut self) -> u16 {
            self.bump(0, 2);
            self.bump(2, 2);
            self.bump(2, 0);
            (self.hist[2] >> 16) as u16 * 100 + self.n * 10 + (self.hist[2] > self.hist[0]) as u16
        }
    }
    fn host() -> u16 {
        let mut w = W { hist: [0; 3], n: 0 };
        w.run()
    }
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn u32_array_rejections() {
    // Bare names/fields aren't values; the steering names the fix.
    let err = rustz80::compile_fn("fn f() -> u16 { let a = [0u32; 2]; (a + 1u32) as u16 }")
        .err()
        .unwrap();
    assert!(err.contains("index it"), "unexpected: {err}");
    let err = rustz80::compile_program(
        "struct S { a: [u32; 2] }
         impl S { fn run(&mut self) -> u16 { (self.a + 1u32) as u16 } }",
    )
    .err()
    .unwrap();
    assert!(err.contains("index it"), "unexpected: {err}");
}
