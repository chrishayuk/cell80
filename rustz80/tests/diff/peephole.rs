//! Per-rule behavioural cases for the Stage-2 peephole (`codegen/peephole.rs`).
//! Each shape below compiles through a specific rewrite rule; rustc is the oracle
//! that the rewrite changed nothing. The matching size/shape assertions (proving
//! each rule actually *fired*) live in `tests/peephole_shape.rs` — a rule that
//! never fires would pass these trivially.

use crate::harness::*;

#[test]
fn r1_leaf_operand_pairs() {
    // Var⊕Var and Var⊕Lit for every binop family — the PUSH/POP → EX DE,HL rule.
    check!({
        let a = 1000u16;
        let b = 42u16;
        let mut acc = a - b; // leaf/leaf sub (non-commutative: order must survive)
        acc = acc + (a & b); // bitwise pair
        acc = acc + (a | 511u16); // Var|Lit
        acc = acc + (a ^ b); // xor pair
        acc
    });
}

#[test]
fn r2_literal_add() {
    // `x + lit` — the EX;LD HL,lit;ADD → LD DE,lit;ADD upgrade, incl. the loop
    // induction shape (`i = i + 1`) it exists for.
    check!({
        let mut i = 0u16;
        let mut acc = 0u16;
        while i < 10u16 {
            acc = acc + 3u16;
            i = i + 1u16;
        }
        acc + 100u16
    });
}

#[test]
fn r3_store_then_reload() {
    // An assignment immediately read back — the reload is elided; the stored value
    // must still be correct *in memory* (later reads) and in HL (the chained use).
    check!({
        let a = 7u16;
        let x = a * 6u16;
        let y = x + 1u16; // reload of x elided (HL already holds it)
        y + x // later read of x comes from memory — the store must have happened
    });
}

#[test]
fn r4_one_arg_call() {
    // 1-arg calls carry a dead PUSH HL;POP HL pair around the argument.
    fn dbl(x: u16) -> u16 {
        x + x
    }
    fn host() -> u16 {
        dbl(21) + dbl(100)
    }
    let src = "
        fn dbl(x: u16) -> u16 { x + x }
        fn run() -> u16 { dbl(21u16) + dbl(100u16) }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn r5_two_arg_call() {
    // 2-arg calls end PUSH HL;POP DE;POP HL → EX DE,HL;POP HL. Argument order must
    // survive (sub is non-commutative).
    fn sub2(x: u16, y: u16) -> u16 {
        x - y
    }
    fn host() -> u16 {
        sub2(500, 42) + sub2(1000, 999)
    }
    let src = "
        fn sub2(x: u16, y: u16) -> u16 { x - y }
        fn run() -> u16 { sub2(500u16, 42u16) + sub2(1000u16, 999u16) }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn r6_ex_ex_cancellation() {
    // `leaf - arr[i]`: the element load ends EX DE,HL and R1 inserts another —
    // the pair cancels. The subtraction result and the element must both survive.
    check!({
        let arr = [10u16, 20u16, 30u16, 40u16];
        let mut i = 1u16;
        let mut acc = 1000u16 - arr[i as usize]; // 980
        i = i + 2u16;
        acc = acc + (5000u16 - arr[i as usize]); // + 4960
        acc
    });
}

#[test]
fn peephole_kitchen_sink() {
    // All rules interacting in one body, against rustc.
    fn helper(x: u16, y: u16) -> u16 {
        x * 2 + y
    }
    fn only(x: u16) -> u16 {
        x ^ 0x5555
    }
    fn host() -> u16 {
        let mut acc = 0u16;
        let mut i = 0u16;
        while i < 8 {
            let t = (only(i) - helper(i, 3)) & 0x0FFF;
            acc = acc + t + (i + 1);
            i = i + 1;
        }
        acc
    }
    let src = "
        fn helper(x: u16, y: u16) -> u16 { x * 2u16 + y }
        fn only(x: u16) -> u16 { x ^ 0x5555u16 }
        fn run() -> u16 {
            let mut acc = 0u16;
            let mut i = 0u16;
            while i < 8u16 {
                let t = (only(i) - helper(i, 3u16)) & 0x0FFFu16;
                acc = acc + t + (i + 1u16);
                i = i + 1u16;
            }
            acc
        }
    ";
    assert_eq!(run_program(src, "run"), host());
}
