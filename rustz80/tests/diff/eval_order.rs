//! Evaluation order is left-to-right wherever it is observable (A2a,
//! `docs/13-multi-target-spec.md` §2.2.3). Each program here puts a
//! side-effecting method call in both operand positions of the ops whose classic
//! codegen evaluated the right operand first (`-`, `/`, `%`, comparisons — 16-bit
//! and u32) — under the pre-A2a order every one of these diverged from rustc.
//! The rustc oracle *is* the order specification; `run_program` also asserts the
//! IR interpreter (left-to-right by construction) agrees.

use crate::harness::run_program;

/// The shared counter shape: `a()` bumps and scales, `b()` bumps and doubles —
/// so each side's value depends on whether the other ran first.
struct C {
    n: u16,
}
impl C {
    fn a(&mut self) -> u16 {
        self.n += 1;
        self.n * 10
    }
    fn b(&mut self) -> u16 {
        self.n += 1;
        self.n * 2
    }
}

const C_SRC: &str = "
    struct C { n: u16 }
    impl C {
        fn a(&mut self) -> u16 { self.n = self.n + 1; self.n * 10 }
        fn b(&mut self) -> u16 { self.n = self.n + 1; self.n * 2 }
    }
";

fn host<R>(f: impl Fn(&mut C) -> R) -> R {
    f(&mut C { n: 0 })
}

#[test]
fn sub_evaluates_left_then_right() {
    let src = format!("{C_SRC} fn run() -> u16 {{ let mut c = C {{ n: 0u16 }}; c.a() - c.b() }}");
    assert_eq!(run_program(&src, "run"), host(|c| c.a() - c.b())); // 10 - 4
}

#[test]
fn div_and_rem_evaluate_left_then_right() {
    let src = format!("{C_SRC} fn run() -> u16 {{ let mut c = C {{ n: 0u16 }}; c.b() / c.a() }}");
    assert_eq!(run_program(&src, "run"), host(|c| c.b() / c.a())); // 2 / 20
    let src = format!("{C_SRC} fn run() -> u16 {{ let mut c = C {{ n: 0u16 }}; c.a() % c.b() }}");
    assert_eq!(run_program(&src, "run"), host(|c| c.a() % c.b())); // 10 % 4
}

#[test]
fn comparisons_evaluate_left_then_right() {
    // Value position, all four orderings + equality (each classic shape differed).
    for (op, want) in [
        ("<", host(|c| (c.b() < c.a()) as u16)), // 2 < 20
        (">", host(|c| (c.b() > c.a()) as u16)), // 2 > 20
        ("<=", host(|c| (c.b() <= c.a()) as u16)),
        (">=", host(|c| (c.b() >= c.a()) as u16)),
        ("==", host(|c| (c.b() == c.a()) as u16)),
        ("!=", host(|c| (c.b() != c.a()) as u16)),
    ] {
        let src = format!(
            "{C_SRC} fn run() -> u16 {{ let mut c = C {{ n: 0u16 }}; (c.b() {op} c.a()) as u16 }}"
        );
        assert_eq!(run_program(&src, "run"), want, "op {op}");
    }
}

#[test]
fn condition_position_evaluates_left_then_right() {
    let src = format!(
        "{C_SRC} fn run() -> u16 {{ let mut c = C {{ n: 0u16 }};
             if c.b() < c.a() {{ 111 }} else {{ 222 }} }}"
    );
    let want = host(|c| if c.b() < c.a() { 111 } else { 222 });
    assert_eq!(run_program(&src, "run"), want);
}

#[test]
fn signed_comparison_evaluates_left_then_right() {
    // i16 ordering goes through the S⊕V route — same source-order contract.
    let src = "
        struct S { n: i16 }
        impl S {
            fn a(&mut self) -> i16 { self.n = self.n - 1; self.n }
            fn b(&mut self) -> i16 { self.n = self.n - 1; self.n * 3 }
        }
        fn run() -> u16 {
            let mut s = S { n: 0i16 };
            (s.a() < s.b()) as u16
        }
    ";
    struct S {
        n: i16,
    }
    impl S {
        fn a(&mut self) -> i16 {
            self.n -= 1;
            self.n
        }
        fn b(&mut self) -> i16 {
            self.n -= 1;
            self.n * 3
        }
    }
    let mut s = S { n: 0 };
    let want = (s.a() < s.b()) as u16; // -1 < -6 → 0
    assert_eq!(run_program(src, "run"), want);
}

#[test]
fn u32_sub_and_cmp_evaluate_left_then_right() {
    // Widened side-effecting operands drive the Bin32/Cmp32 shapes.
    let src = format!(
        "{C_SRC} fn run() -> u16 {{ let mut c = C {{ n: 0u16 }};
             ((c.a() as u32) - (c.b() as u32)) as u16 }}"
    );
    let want = host(|c| ((c.a() as u32) - (c.b() as u32)) as u16); // 10 - 4
    assert_eq!(run_program(&src, "run"), want);

    for (op, want) in [
        ("<", host(|c| ((c.b() as u32) < (c.a() as u32)) as u16)),
        (">=", host(|c| ((c.b() as u32) >= (c.a() as u32)) as u16)),
        ("==", host(|c| ((c.b() as u32) == (c.a() as u32)) as u16)),
    ] {
        let src = format!(
            "{C_SRC} fn run() -> u16 {{ let mut c = C {{ n: 0u16 }};
                 ((c.b() as u32) {op} (c.a() as u32)) as u16 }}"
        );
        assert_eq!(run_program(&src, "run"), want, "u32 op {op}");
    }
}
