//! `for`/`loop`/`break`/`continue`, early `return`, `enum` + `match`.

use crate::harness::*;

#[test]
#[allow(unused_assignments)]
fn enum_and_match() {
    // C-like enum + match on it (arms assign a result), checked against rustc.
    #[allow(dead_code)]
    enum Color {
        Red,
        Green,
        Blue,
    }
    fn host() -> u16 {
        let c = Color::Green;
        let mut v = 0u16;
        match c {
            Color::Red => v = 100,
            Color::Green => v = 200,
            Color::Blue => v = 300,
        }
        v
    }
    let src = "
        enum Color { Red, Green, Blue }
        fn run() -> u16 {
            let c = Color::Green;
            let mut v = 0u16;
            match c {
                Color::Red => v = 100u16,
                Color::Green => v = 200u16,
                Color::Blue => v = 300u16,
            }
            v
        }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
#[allow(unused_assignments)]
fn match_literals_with_wildcard_and_enum_param() {
    #[allow(dead_code)]
    enum Op {
        Add,
        Sub,
        Mul,
    }
    fn apply(op: Op, a: u16, b: u16) -> u16 {
        let mut r = 0u16;
        match op {
            Op::Add => r = a + b,
            Op::Sub => r = a - b,
            Op::Mul => r = a * b,
        }
        r
    }
    fn classify(n: u16) -> u16 {
        let mut r = 0u16;
        match n {
            0 => r = 10,
            1 => r = 20,
            _ => r = 99,
        }
        r
    }
    fn host() -> u16 {
        apply(Op::Add, 7, 6) + apply(Op::Mul, 4, 5) + classify(0) + classify(1) + classify(7)
    }
    let src = "
        enum Op { Add, Sub, Mul }
        fn apply(op: Op, a: u16, b: u16) -> u16 {
            let mut r = 0u16;
            match op {
                Op::Add => r = a + b,
                Op::Sub => r = a - b,
                Op::Mul => r = a * b,
            }
            r
        }
        fn classify(n: u16) -> u16 {
            let mut r = 0u16;
            match n {
                0u16 => r = 10u16,
                1u16 => r = 20u16,
                _ => r = 99u16,
            }
            r
        }
        fn run() -> u16 {
            apply(Op::Add, 7u16, 6u16) + apply(Op::Mul, 4u16, 5u16)
                + classify(0u16) + classify(1u16) + classify(7u16)
        }
    ";
    assert_eq!(run_program(src, "run"), host()); // 13 + 20 + 10 + 20 + 99 = 162
}

#[test]
fn for_loops() {
    // Exclusive range: sum 0..10 = 45.
    check!({
        let mut s = 0u16;
        for i in 0u16..10u16 {
            s = s + i;
        }
        s
    });
    // Inclusive range: sum 1..=5 = 15.
    check!({
        let mut s = 0u16;
        for i in 1u16..=5u16 {
            s = s + i;
        }
        s
    });
    // Variable upper bound, evaluated once.
    check!({
        let n = 6u16;
        let mut s = 0u16;
        for i in 0u16..n {
            s = s + i;
        }
        s
    }); // 0+1+2+3+4+5 = 15
        // `for` indexing an array (the render-loop shape).
    check!({
        let a = [3u16, 1u16, 4u16, 1u16, 5u16];
        let mut sum = 0u16;
        for i in 0u16..5u16 {
            sum = sum + a[i as usize];
        }
        sum
    }); // 14
        // Nested loops with distinct counters.
    check!({
        let mut s = 0u16;
        for i in 0u16..3u16 {
            for j in 0u16..3u16 {
                s = s + i * j;
            }
        }
        s
    }); // (0+1+2)*(0+1+2) = 9
        // `for _ in` — a counted loop whose variable is unused in the body.
    check!({
        let mut count = 0u16;
        for _ in 0u16..7u16 {
            count = count + 1u16;
        }
        count
    }); // 7
}

#[test]
fn loop_break_continue() {
    // `loop` exited by `break`.
    check!({
        let mut i = 0u16;
        loop {
            if i == 5u16 {
                break;
            }
            i = i + 1u16;
        }
        i
    }); // 5
        // do-while shape: body runs before the break test.
    check!({
        let mut i = 0u16;
        let mut s = 0u16;
        loop {
            s = s + i;
            i = i + 1u16;
            if i > 4u16 {
                break;
            }
        }
        s
    }); // 0+1+2+3+4 = 10
        // `continue` in a `while` re-checks the condition (no step skipped).
    check!({
        let mut i = 0u16;
        let mut s = 0u16;
        while i < 10u16 {
            i = i + 1u16;
            if (i % 2u16) == 1u16 {
                continue;
            }
            s = s + i;
        }
        s
    }); // evens 2+4+6+8+10 = 30
        // `continue` in a `for` MUST run the induction step (else it never ends).
    check!({
        let mut s = 0u16;
        for i in 0u16..10u16 {
            if (i % 2u16) == 0u16 {
                continue;
            }
            s = s + i;
        }
        s
    }); // odds 1+3+5+7+9 = 25
        // `break` out of a `for`.
    check!({
        let mut s = 0u16;
        for i in 0u16..100u16 {
            if i == 5u16 {
                break;
            }
            s = s + i;
        }
        s
    }); // 0+1+2+3+4 = 10
}

#[test]
fn early_return() {
    // Early `return` from an `if` (skips the tail), plus `loop` + `return` — the
    // rejection-sampling idiom (a loop with no `break`, exited only by `return`).
    #[allow(clippy::needless_return)]
    fn classify(n: u16) -> u16 {
        if n == 0 {
            return 100;
        }
        if n == 1 {
            return 200;
        }
        n + 1000
    }
    fn first_ge(start: u16, limit: u16) -> u16 {
        let mut i = start;
        loop {
            if i >= limit {
                return i;
            }
            i = i + 2;
        }
    }
    fn host() -> u16 {
        classify(0) + classify(1) + classify(5) + first_ge(1, 10)
    }
    let src = "
        fn classify(n: u16) -> u16 {
            if n == 0u16 { return 100u16; }
            if n == 1u16 { return 200u16; }
            n + 1000u16
        }
        fn first_ge(start: u16, limit: u16) -> u16 {
            let mut i = start;
            loop {
                if i >= limit { return i; }
                i = i + 2u16;
            }
        }
        fn run() -> u16 {
            classify(0u16) + classify(1u16) + classify(5u16) + first_ge(1u16, 10u16)
        }
    ";
    assert_eq!(run_program(src, "run"), host()); // 100 + 200 + 1005 + 11 = 1316
}

#[test]
fn control_flow_rejections() {
    // `break`/`continue` outside any loop are rejected (not dangling jumps).
    assert!(rustz80::compile_fn("fn f() -> u16 { break; 0u16 }").is_err());
    assert!(rustz80::compile_fn("fn f() -> u16 { continue; 0u16 }").is_err());
    // `break <value>` and labeled break/continue are outside the subset.
    assert!(rustz80::compile_fn("fn f() -> u16 { loop { break 1u16; } }").is_err());
    assert!(rustz80::compile_fn("fn f() -> u16 { 'a: loop { break 'a; } }").is_err());
    // `for` only accepts integer ranges, not arbitrary iterators.
    assert!(rustz80::compile_fn("fn f() -> u16 { let a = [1u16]; for x in a { } 0u16 }").is_err());
}
