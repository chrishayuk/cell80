//! Scalar arithmetic, bitwise ops, shifts, comparisons-as-values, and `bool` logic.

use crate::harness::*;

#[test]
fn arithmetic() {
    check!({
        let a = 7u16;
        let b = 6u16;
        a + b
    });
    check!({
        let a = 1000u16;
        let b = 24u16;
        let c = 6u16;
        (a - b) + c
    });
    check!({
        let a = 5u16;
        a - 5u16 + 100u16
    });
}

#[test]
fn if_else() {
    check!({
        let a = 3u16;
        let b = 8u16;
        let mut m = a;
        if b > a {
            m = b;
        }
        m
    });
    check!({
        let x = 42u16;
        let mut r = 0u16;
        if x == 42u16 {
            r = 1u16;
        } else {
            r = 2u16;
        }
        r
    });
}

#[test]
fn while_loops() {
    // sum 0..10 = 45
    check!({
        let mut s = 0u16;
        let mut i = 0u16;
        while i < 10u16 {
            s = s + i;
            i = i + 1u16;
        }
        s
    });
    // countdown: multiply-by-repeated-addition (7 * 6 without a mul runtime yet)
    check!({
        let mut acc = 0u16;
        let mut n = 7u16;
        while n != 0u16 {
            acc = acc + 6u16;
            n = n - 1u16;
        }
        acc
    });
}

#[test]
fn mul_div_rem() {
    // `*`/`/`/`%` go through the appended micro-runtime — checked against rustc.
    check!({ 7u16 * 6u16 });
    check!({
        let a = 123u16;
        let b = 45u16;
        a * b
    });
    check!({ 1000u16 / 7u16 });
    check!({ 1000u16 % 7u16 });
    check!({
        let a = 9u16;
        let b = 4u16;
        a / b * b + a % b
    }); // == a
    check!({
        let mut s = 0u16;
        let mut i = 1u16;
        while i <= 5u16 {
            s = s + i * i;
            i = i + 1u16;
        }
        s
    }); // 1+4+9+16+25 = 55
}

#[test]
fn scalar_u8() {
    // Non-overflowing u8 arithmetic widened to u16.
    check!({
        let a = 100u8;
        let b = 50u8;
        (a + b) as u16
    }); // 150
        // u8 wrapping must match rustc's wrapping_* exactly.
    check!({
        let a = 200u8;
        let b = 100u8;
        a.wrapping_add(b) as u16
    }); // 300 wraps to 44
    check!({
        let a = 10u8;
        let b = 20u8;
        a.wrapping_sub(b) as u16
    }); // wraps to 246
    check!({
        let a = 20u8;
        let b = 20u8;
        a.wrapping_mul(b) as u16
    }); // 400 wraps to 144
        // u16 -> u8 cast truncates to the low byte.
    check!({
        let x = 300u16;
        (x as u8) as u16
    }); // 44
        // u8 loop counter with widening reads.
    check!({
        let mut sum = 0u16;
        let mut i = 0u8;
        while (i as u16) < 5u16 {
            sum = sum + i as u16;
            i = i.wrapping_add(1u8);
        }
        sum
    }); // 0+1+2+3+4 = 10
}

#[test]
fn bitwise() {
    check!({ 12u16 | 10u16 }); // 14
    check!({ 12u16 & 10u16 }); // 8
    check!({ 12u16 ^ 10u16 }); // 6
    check!({
        let a = 0xF0u8;
        let b = 0x0Fu8;
        (a | b) as u16
    }); // 255
    check!({
        let a = 200u8;
        let b = 0x0Fu8;
        (a & b) as u16
    }); // 200 & 15 = 8
}

#[test]
fn square_same_var() {
    // `x * x` (same variable) takes the load-once square path; must match `x * y` and rustc
    // across widths, including overflow wrap.
    fn host() -> u16 {
        let v = [0u16, 1u16, 7u16, 255u16, 256u16, 1000u16, 40000u16];
        let mut s = 0u16;
        for i in 0..7 {
            s = s.wrapping_add(v[i].wrapping_mul(v[i])); // square
        }
        s
    }
    let src = "
        fn run() -> u16 {
            let v = [0u16, 1u16, 7u16, 255u16, 256u16, 1000u16, 40000u16];
            let mut s = 0u16;
            let mut i = 0u16;
            while i < 7u16 { let x = v[i as usize]; s = s.wrapping_add(x * x); i = i + 1u16; }
            s
        }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn mul16_operand_widths() {
    // The multiplier-terminated `__mul16` must be correct across multiplier widths: 0
    // (immediate return), 1 bit, full 16 bits, and in between. `a[i] * b[i]` is var*var,
    // so it goes through the runtime (not const strength-reduction). Checked vs rustc.
    fn host() -> u16 {
        let a = [255u16, 1u16, 65535u16, 0u16, 123u16];
        let b = [255u16, 65535u16, 1u16, 12345u16, 456u16];
        let mut s = 0u16;
        for i in 0..5 {
            s = s.wrapping_add(a[i].wrapping_mul(b[i]));
        }
        s
    }
    let src = "
        fn run() -> u16 {
            let a = [255u16, 1u16, 65535u16, 0u16, 123u16];
            let b = [255u16, 65535u16, 1u16, 12345u16, 456u16];
            let mut s = 0u16;
            let mut i = 0u16;
            while i < 5u16 {
                s = s.wrapping_add(a[i as usize].wrapping_mul(b[i as usize]));
                i = i + 1u16;
            }
            s
        }
    ";
    let prog = rustz80::compile_program(src).expect("compile");
    assert!(prog.symbols.contains_key("__mul16")); // var*var uses the runtime
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn divmod16_small_dividend() {
    // The `dividend < divisor` fast path + the normal restoring path, across var/var
    // pairs: a<b (fast), a>b, a==b, 0/b, and a wide dividend. Checked vs rustc.
    fn host() -> u16 {
        let a = [5u16, 100u16, 7u16, 0u16, 65535u16];
        let b = [10u16, 7u16, 7u16, 5u16, 13u16];
        let mut s = 0u16;
        for i in 0..5 {
            s = s.wrapping_add(a[i] / b[i]).wrapping_add(a[i] % b[i]);
        }
        s
    }
    let src = "
        fn run() -> u16 {
            let a = [5u16, 100u16, 7u16, 0u16, 65535u16];
            let b = [10u16, 7u16, 7u16, 5u16, 13u16];
            let mut s = 0u16;
            let mut i = 0u16;
            while i < 5u16 {
                s = s.wrapping_add(a[i as usize] / b[i as usize]);
                s = s.wrapping_add(a[i as usize] % b[i as usize]);
                i = i + 1u16;
            }
            s
        }
    ";
    let prog = rustz80::compile_program(src).expect("compile");
    assert!(prog.symbols.contains_key("__divmod16")); // var/var uses the runtime
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn const_strength_reduction() {
    // Constant multiply (shift-and-add), divide/remainder by a power of two (shift/mask),
    // and const-folding — all must match rustc, and the program must compile *without* the
    // mul/div runtimes (the whole point).
    fn host() -> u16 {
        let mut acc = 0u16;
        let mut n = 1u16;
        while n < 8 {
            acc = acc + n * 3; // ×3 via shift-add
            acc = acc + (n * 7) % 8; // ×7 then mask
            acc = acc + n * 4 / 2; // ×4 (pow2) then >>1
            n = n + 1;
        }
        acc + (2 * 5 + 4) // const-folded to 14
    }
    let src = "
        fn run() -> u16 {
            let mut acc = 0u16;
            let mut n = 1u16;
            while n < 8u16 {
                acc = acc + n * 3u16;
                acc = acc + (n * 7u16) % 8u16;
                acc = acc + n * 4u16 / 2u16;
                n = n + 1u16;
            }
            acc + (2u16 * 5u16 + 4u16)
        }
    ";
    assert_eq!(run_program(src, "run"), host()); // 84 + 28 + 56 + 14 = 182
                                                 // Strength reduction fired: no `__mul16` / `__divmod16` runtime was appended.
    let prog = rustz80::compile_program(src).expect("compile");
    assert!(
        !prog.symbols.contains_key("__mul16"),
        "constant mul should not call __mul16"
    );
    assert!(
        !prog.symbols.contains_key("__divmod16"),
        "pow2 div/rem should not call __divmod16"
    );
}

#[test]
fn u16_shifts() {
    // `<<` / `>>` by a constant on a u16 (logical).
    fn host() -> u16 {
        let a = 3u16;
        let b = 0xF0F0u16;
        (a << 4) | (a >> 1) | (b >> 8) | ((b << 4) & 0xFF00)
    }
    let src = "
        fn run() -> u16 {
            let a = 3u16;
            let b = 0xF0F0u16;
            (a << 4u16) | (a >> 1u16) | (b >> 8u16) | ((b << 4u16) & 0xFF00u16)
        }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn comparisons_as_values() {
    // A comparison used as a value materialises to `1`/`0` (`bool as u16`) — every
    // operator, both outcomes, checked against rustc.
    check!({
        let a = 3u16;
        let b = 5u16;
        (a < b) as u16
    }); // 1
    check!({
        let a = 5u16;
        let b = 5u16;
        (a < b) as u16 + (a <= b) as u16 * 10u16
    }); // 0 + 1*10 = 10
    check!({
        let a = 7u16;
        let b = 2u16;
        (a > b) as u16 + (a >= b) as u16 * 10u16
    }); // 1 + 10 = 11
    check!({
        let a = 4u16;
        let b = 4u16;
        (a == b) as u16 + (a != b) as u16 * 10u16
    }); // 1 + 0 = 1
        // Composing bools by arithmetic, and a bool bound to a `let`.
    check!({
        let a = 1u16;
        let b = 2u16;
        let c = 2u16;
        (a < b) as u16 + (b < c) as u16 + (a == c) as u16
    }); // 1 + 0 + 0 = 1
    check!({
        let a = 10u16;
        let b = 3u16;
        let f = a > b;
        f as u16
    }); // 1
        // A predicate result feeding arithmetic in a loop: count evens in 0..10.
    check!({
        let mut n = 0u16;
        let mut i = 0u16;
        while i < 10u16 {
            n = n + (i % 2u16 == 0u16) as u16;
            i = i + 1u16;
        }
        n
    }); // 0,2,4,6,8 → 5
}

#[test]
fn logical_and_or() {
    // Short-circuit `&&` / `||` on bool operands, as values and in conditions — vs rustc.
    check!({
        let x = 5u16;
        ((x > 0u16) && (x < 10u16)) as u16
    }); // 1
    check!({
        let x = 15u16;
        ((x > 0u16) && (x < 10u16)) as u16
    }); // 0
    check!({
        let x = 15u16;
        ((x == 0u16) || (x > 10u16)) as u16
    }); // 1
    check!({
        let x = 5u16;
        ((x == 0u16) || (x > 10u16)) as u16
    }); // 0
        // Chained `&&` (three operands).
    check!({
        let a = 1u16;
        let b = 2u16;
        let c = 3u16;
        ((a < b) && (b < c) && (a < c)) as u16
    }); // 1
        // `&&` / `||` in condition position (the common case).
    check!({
        let x = 7u16;
        let mut r = 0u16;
        if x > 0u16 && x < 10u16 {
            r = 1u16;
        }
        r
    }); // 1
    check!({
        let x = 50u16;
        let mut r = 9u16;
        if x == 0u16 || x > 10u16 {
            r = 1u16;
        }
        r
    }); // 1
}

#[test]
fn variable_shifts() {
    // Shift by a *runtime* amount (a `let`/loop variable, not a literal) — left & right,
    // logical, checked against rustc. Amounts stay < 16 (a u16 shift ≥ 16 panics in rustc
    // debug; the cell's saturate-to-0 behaviour is covered in the cell80 suite).
    check!({
        let x = 1u16;
        let s = 0u16;
        x << s
    }); // 1
    check!({
        let x = 1u16;
        let s = 7u16;
        x << s
    }); // 128
    check!({
        let x = 1u16;
        let s = 15u16;
        x << s
    }); // 32768
    check!({
        let x = 0xF0F0u16;
        let s = 4u16;
        x >> s
    }); // 0x0F0F = 3855
    check!({
        let x = 12345u16;
        let s = 3u16;
        x >> s
    }); // 1543
        // Amount from a loop variable: sum of powers of two 2^0..2^7 = 255.
    check!({
        let mut acc = 0u16;
        let mut i = 0u16;
        while i < 8u16 {
            acc = acc + (1u16 << i);
            i = i + 1u16;
        }
        acc
    }); // 255
        // Build a mask via a runtime shift, then test a bit (the bitop idiom).
    check!({
        let x = 11u16;
        let bit = 3u16;
        (x >> bit) & 1u16
    }); // 11 = 1011b, bit 3 = 1
    check!({
        let x = 11u16;
        let bit = 2u16;
        (x >> bit) & 1u16
    }); // bit 2 = 0
}

// `bool` fields/locals/returns + `true`/`false` + unary `!` (logical not), in both value
// and condition position — the readability win for game flags (`if !self.started` instead
// of `self.started == 0u16`). The same source is real Rust, so it's checked vs rustc.
#[test]
fn bool_flags_and_logical_not() {
    fn host() -> u16 {
        struct S {
            on: bool,
            ready: bool,
        }
        impl S {
            fn arm(&mut self) {
                self.ready = true;
            }
            fn idle(&self) -> bool {
                !self.on
            }
        }
        let mut s = S {
            on: false,
            ready: false,
        };
        let mut acc = 0u16;
        if !s.on {
            acc = acc + 1u16; // on=false ⇒ +1
        }
        if s.idle() {
            acc = acc + 2u16; // idle = !on = true ⇒ +2
        }
        s.arm();
        if s.ready {
            acc = acc + 4u16; // ready=true ⇒ +4
        }
        let flag = !s.ready; // false
        acc = acc + (flag as u16) * 8u16; // +0
        while !s.on {
            s.on = true;
            acc = acc + 16u16; // runs once ⇒ +16
        }
        acc
    }
    let src = "
        struct S { on: bool, ready: bool }
        impl S {
            fn arm(&mut self) { self.ready = true; }
            fn idle(&self) -> bool { !self.on }
        }
        fn run() -> u16 {
            let mut s = S { on: false, ready: false };
            let mut acc = 0u16;
            if !s.on { acc = acc + 1u16; }
            if s.idle() { acc = acc + 2u16; }
            s.arm();
            if s.ready { acc = acc + 4u16; }
            let flag = !s.ready;
            acc = acc + (flag as u16) * 8u16;
            while !s.on { s.on = true; acc = acc + 16u16; }
            acc
        }
    ";
    assert_eq!(run_program(src, "run"), host()); // 1+2+4+0+16 = 23
}

#[test]
fn saturating_methods() {
    // `saturating_add`/`_sub`/`_mul` — real Rust, oracle-checked, u16 and u8,
    // exercising both the clamped and unclamped sides.
    check!({
        let a = 65530u16;
        let b = 10u16;
        a.saturating_add(b) - 60000u16 + 40000u16.saturating_add(1000u16) / 100u16
    });
    check!({
        let a = 5u16;
        a.saturating_sub(9u16) + 9u16.saturating_sub(5u16)
    });
    check!({
        let a = 300u16;
        a.saturating_mul(300u16) - 250u16.saturating_mul(250u16)
    });
    check!({
        let c = 250u8;
        c.saturating_add(10u8) as u16 + 3u8.saturating_add(4u8) as u16
    });
    check!({
        let c = 5u8;
        c.saturating_sub(9u8) as u16 + 200u8.saturating_sub(100u8) as u16
    });
    check!({
        let c = 20u8;
        c.saturating_mul(20u8) as u16 + 15u8.saturating_mul(15u8) as u16
    });
    // Saturating in a loop — the accumulate shape library cells hand-roll today.
    check!({
        let mut acc = 60000u16;
        for i in 0..10u16 {
            acc = acc.saturating_add(i * 1000u16);
        }
        acc
    });
}

#[test]
fn saturating_rejections() {
    // Effectful operands reject with a steering message (the clamp re-reads them);
    // every u32 saturating_* graduated to a feature (see u32_ops.rs).
    let err = rustz80::compile_program(
        "fn g(a: u16) -> u16 { a } fn f(a: u16) -> u16 { g(a).saturating_add(1u16) }",
    )
    .err()
    .unwrap();
    assert!(err.contains("bind them first"), "unexpected: {err}");
}

#[test]
fn bit_methods_counting() {
    // count_ones / leading_zeros / trailing_zeros — appended `__bits_*` kernels,
    // oracle-checked across widths and edge values (0, all-ones).
    check!({
        let x = 0b1011_0010_1000_0001u16;
        x.count_ones() as u16 * 100u16
            + 0u16.count_ones() as u16 * 10u16
            + 0xFFFFu16.count_ones() as u16
    });
    check!({
        let x = 0x0100u16;
        x.leading_zeros() as u16 * 100u16
            + 0u16.leading_zeros() as u16
            + 0x8000u16.leading_zeros() as u16 * 10u16
    });
    check!({
        let x = 0x0100u16;
        x.trailing_zeros() as u16 * 100u16
            + 0u16.trailing_zeros() as u16
            + 1u16.trailing_zeros() as u16 * 10u16
    });
    // u8 widths: lz is 8-based, tz caps at 8, count matches.
    check!({
        let c = 0b0010_0100u8;
        c.count_ones() as u16 * 100u16
            + c.leading_zeros() as u16 * 10u16
            + c.trailing_zeros() as u16
    });
    check!({ 0u8.leading_zeros() as u16 * 100u16 + 0u8.trailing_zeros() as u16 });
}

#[test]
fn bit_methods_rotate_swap() {
    // Constant amounts (unrolled shifts), zero/width-multiple amounts, and a
    // runtime amount — all against rustc.
    check!({
        let x = 0xABCDu16;
        x.rotate_left(4) ^ x.rotate_right(4) ^ x.rotate_left(0) ^ x.rotate_left(16)
    });
    check!({
        let x = 0xABCDu16;
        let mut acc = 0u16;
        for k in 0..20u16 {
            acc = acc ^ x.rotate_left(k as u32) ^ x.rotate_right(k as u32);
        }
        acc
    });
    check!({
        let c = 0b1001_0110u8;
        (c.rotate_left(3) as u16) * 256u16 + c.rotate_right(3) as u16
    });
    check!({
        let x = 0xABCDu16;
        x.swap_bytes() + 0x00FFu16.swap_bytes() / 256u16
    });
    check!({
        let c = 0x5Au8;
        c.swap_bytes() as u16
    });
}

#[test]
fn bit_method_rejections() {
    let err = rustz80::compile_fn("fn f(a: u16) -> u16 { ((a as u32).count_ones()) as u16 }")
        .err()
        .unwrap();
    assert!(err.contains("u32 `count_ones`"), "unexpected: {err}");
}

#[test]
fn bit_methods_over_composite_pure_operands() {
    // The purity walker sees through every node kind: swap/rotate over reads,
    // shifts, comparisons, and array/pointer loads all lower (and match rustc).
    let src = "
        const T: [u16; 2] = [0x1234u16, 0x00FFu16];
        struct S { arr: [u16; 2], x: u16 }
        impl S {
            fn run(&mut self) -> u16 {
                let k = 3u16;
                let a = [0xAAu16, 0x55u16];
                self.arr[0] = 0xF0F0u16;
                (self.arr[0]).swap_bytes()
                    ^ (a[1]).swap_bytes()
                    ^ T[0].swap_bytes()
                    ^ ((self.x < 5u16) as u16).swap_bytes()
                    ^ (a[0] << k).swap_bytes()
                    ^ (peek(0u16) as u16).rotate_left(2)
                    ^ ((self.x as u32 + 1u32) as u16).rotate_right(1)
            }
        }
        fn run() -> u16 {
            let mut s = S { arr: [0u16; 2], x: 2u16 };
            s.run()
        }
    ";
    struct S {
        arr: [u16; 2],
        x: u16,
    }
    const T: [u16; 2] = [0x1234, 0x00FF];
    impl S {
        fn run(&mut self) -> u16 {
            let k = 3u16;
            let a = [0xAAu16, 0x55u16];
            self.arr[0] = 0xF0F0;
            (self.arr[0]).swap_bytes()
                ^ (a[1]).swap_bytes()
                ^ T[0].swap_bytes()
                ^ ((self.x < 5) as u16).swap_bytes()
                ^ (a[0] << k).swap_bytes()
                ^ 0u16.rotate_left(2)
                ^ ((self.x as u32 + 1) as u16).rotate_right(1)
        }
    }
    fn host() -> u16 {
        let mut s = S { arr: [0; 2], x: 2 };
        s.run()
    }
    assert_eq!(run_program(src, "run"), host());
}
