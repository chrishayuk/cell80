//! Phase 1.1 — `if`/`match` as **expressions**: the single most idiomatic shape an LLM
//! emits (`let x = if c { a } else { b };`), lowered to statement form through the
//! destination slot. Every position (let / assign / return / tail), nesting, `else if`
//! chains, wide (u32) arms — all against the rustc oracle on both targets.

// The shapes below are deliberately the LLM-idiomatic forms under test — a `let`
// returned from a block and a bool-producing `if` are the *subject*, not style slips.
#![allow(clippy::let_and_return, clippy::needless_bool)]

fn rejected(src: &str, needle: &str) {
    let e = rustz80::compile_fn(src).expect_err("must not compile");
    assert!(e.contains(needle), "wrong diagnostic: {e}");
}

#[test]
fn if_expression_positions() {
    // let-position
    check!({
        let a = 3u16;
        let b = 8u16;
        let m = if a > b { a } else { b };
        m * 2u16
    });
    // assignment-position (mutating an existing var)
    check!({
        let x = 7u16;
        let mut r = 0u16;
        r = if x == 7u16 { 100u16 } else { 200u16 };
        r + 1u16
    });
    // tail-position: the classic `fn f() -> u16 { if c { 1 } else { 2 } }`
    check!({
        let c = 1u16;
        if c != 0u16 {
            1u16
        } else {
            2u16
        }
    });
    // return-position
    check!({
        let n = 41u16;
        if n > 40u16 {
            return if n % 2u16 == 0u16 { n } else { n + 1u16 };
        }
        0u16
    });
}

#[test]
fn nested_if_expressions() {
    // nesting in branch position + an `else if` chain
    check!({
        let score = 77u16;
        let grade = if score >= 90u16 {
            4u16
        } else if score >= 70u16 {
            if score >= 80u16 {
                3u16
            } else {
                2u16
            }
        } else {
            1u16
        };
        grade * 10u16 + 5u16
    });
    // branches with leading statements before the value
    check!({
        let a = 12u16;
        let r = if a > 10u16 {
            let sq = a * a;
            sq + 1u16
        } else {
            a
        };
        r
    });
}

#[test]
fn match_expressions() {
    check!({
        let op = 2u16;
        let r = match op {
            0u16 => 10u16,
            1u16 => 20u16,
            2u16 => 30u16,
            _ => 0u16,
        };
        r + 1u16
    });
    // arm blocks with statements + a nested if-expression arm
    check!({
        let kind = 1u16;
        let v = 6u16;
        match kind {
            0u16 => v * 2u16,
            1u16 => {
                let d = v + 1u16;
                if d > 5u16 {
                    d * 10u16
                } else {
                    d
                }
            }
            _ => 0u16,
        }
    });
}

#[test]
fn wide_if_expression() {
    // u32 arms: the percent-cell shape written the idiomatic way.
    check!({
        let part = 700u16;
        let whole = 1000u16;
        let q = if whole != 0u16 {
            part as u32 * 100u32 / whole as u32
        } else {
            0u32
        };
        q as u16
    });
}

#[test]
fn bool_if_expression() {
    check!({
        let a = 5u16;
        let b = 9u16;
        let swapped = if a > b { true } else { false };
        swapped as u16 + 10u16
    });
}

#[test]
fn value_conditionals_reject_incomplete_paths() {
    // A value-`if` without `else`: some path produces nothing.
    rejected(
        "fn f(a: u16) -> u16 { let x = if a > 1u16 { 2u16 }; x }",
        "needs an `else` branch",
    );
    // A value-`match` without a `_` arm.
    rejected(
        "fn f(a: u16) -> u16 { let x = match a { 0u16 => 1u16 }; x }",
        "needs a `_` arm",
    );
    // A branch ending in a statement instead of the value.
    rejected(
        "fn f(a: u16) -> u16 { let x = if a > 1u16 { let y = 2u16; } else { 3u16 }; x }",
        "end",
    );
}

#[test]
fn tail_if_statement_stays_a_statement() {
    // A void-shaped tail `if` (branches end in `;`) must still lower as a statement —
    // the value desugar only fires when every path yields a value.
    check!({
        let a = 3u16;
        let mut r = 0u16;
        if a > 1u16 {
            r = 5u16;
        } else {
            r = 6u16;
        }
        r
    });
}

#[test]
fn match_range_patterns() {
    // Range patterns — the graduated `range_pattern` repair class, now a feature.
    // The two shapes straight from the old repair rows:
    check!({
        let x = 5u16;
        match x {
            0u16..=9u16 => 1u16,
            _ => 0u16,
        }
    });
    check!({
        let mut total = 0u16;
        for s in 0..100u16 {
            let band = match s {
                0u16..=49u16 => 0u16,
                50u16..=79u16 => 1u16,
                _ => 2u16,
            };
            total = total + band;
        }
        total
    });
    // Exclusive ranges and byte-literal bounds.
    check!({
        let c = b'7';
        match c {
            b'0'..=b'9' => 1u16,
            b'a'..=b'z' => 2u16,
            _ => 0u16,
        }
    });
    check!({
        let x = 10u16;
        match x {
            0u16..10u16 => 1u16,
            _ => 9u16,
        }
    });
}

#[test]
fn match_or_patterns() {
    // Or-patterns, including a range inside an or-list.
    check!({
        let mut acc = 0u16;
        for x in 0..12u16 {
            let k = match x {
                1u16 | 2u16 | 3u16 => 1u16,
                5u16 | 7u16 => 2u16,
                8u16..=9u16 | 11u16 => 3u16,
                _ => 0u16,
            };
            acc = acc * 2u16 + k;
        }
        acc
    });
}

#[test]
fn match_pattern_rejections() {
    // Open ranges and bindings keep instructive rejections.
    let err = rustz80::compile_fn("fn f(x: u16) -> u16 { match x { 5u16.. => 1u16, _ => 0u16 } }")
        .err()
        .unwrap();
    assert!(err.contains("both bounds"), "unexpected: {err}");
    let err = rustz80::compile_fn("fn f(x: u16) -> u16 { match x { y => y } }")
        .err()
        .unwrap();
    assert!(err.contains("no bindings"), "unexpected: {err}");
}
