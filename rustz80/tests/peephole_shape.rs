//! Size/shape assertions for the Stage-2 peephole: each rule's canonical shape,
//! proven to have **fired** (exact emitted bytes, or presence/absence of the
//! rewritten sequence). The behavioural halves (rustc as the oracle that the
//! rewrite changed nothing) live in `tests/diff/peephole.rs` — a no-op rule passes
//! diff trivially, so these are the tests that keep the rules honest.

use rustz80::{compile_fn, compile_program};

fn contains(code: &[u8], needle: &[u8]) -> bool {
    code.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn r1_leaf_pair_exact_bytes() {
    // `a - b`: PUSH HL; LD HL,(a); POP DE → EX DE,HL; LD HL,(a).
    let code = compile_fn("fn sub(a: u16, b: u16) -> u16 { a - b }").unwrap();
    assert_eq!(
        code,
        [
            0x22, 0x00, 0x90, // LD (0x9000),HL   a
            0xED, 0x53, 0x02, 0x90, // LD (0x9002),DE   b
            0x2A, 0x02, 0x90, // LD HL,(b)
            0xEB, // EX DE,HL         (was PUSH HL … POP DE)
            0x2A, 0x00, 0x90, // LD HL,(a)
            0xB7, // OR A
            0xED, 0x52, // SBC HL,DE
            0xC9, // RET
        ],
        "R1 did not produce the EX DE,HL leaf-pair shape"
    );
}

#[test]
fn r2_literal_add_exact_bytes() {
    // `a + 3`: the literal goes straight into DE — no EX, no push/pop. R3 also
    // composes with the prologue here: `LD (a),HL; LD HL,(a)` drops the reload
    // (HL still holds the argument on entry), so the body is just the add. (Uses
    // `+ 3` so R7's `+1`/`+2` INC reduction doesn't further rewrite it.)
    let code = compile_fn("fn addc(a: u16) -> u16 { a + 3u16 }").unwrap();
    assert_eq!(
        code,
        [
            0x22, 0x00, 0x90, // LD (0x9000),HL   a   (reload elided by R3)
            0x11, 0x03, 0x00, // LD DE,3              (was PUSH; LD HL,3; POP DE)
            0x19, // ADD HL,DE
            0xC9, // RET
        ],
        "R2 did not load the literal straight into DE"
    );
}

#[test]
fn r7_inc_strength_reduction_exact_bytes() {
    // `a + 1` → INC HL (1 byte), `a + 2` → INC HL; INC HL — the `LD DE,imm; ADD HL,DE`
    // R2 leaves is strength-reduced away. (R3 elides the prologue reload as above.)
    let inc1 = compile_fn("fn inc(a: u16) -> u16 { a + 1u16 }").unwrap();
    assert_eq!(
        inc1,
        [
            0x22, 0x00, 0x90, // LD (0x9000),HL   a
            0x23, // INC HL           (was LD DE,1; ADD HL,DE)
            0xC9, // RET
        ],
        "R7 did not reduce `+1` to INC HL"
    );
    let inc2 = compile_fn("fn inc2(a: u16) -> u16 { a + 2u16 }").unwrap();
    assert_eq!(
        inc2,
        [
            0x22, 0x00, 0x90, // LD (0x9000),HL   a
            0x23, 0x23, // INC HL; INC HL   (was LD DE,2; ADD HL,DE)
            0xC9, // RET
        ],
        "R7 did not reduce `+2` to INC HL; INC HL"
    );
    // `+ 4` is left as an ADD (only ±1/±2 reduce).
    let add4 = compile_fn("fn add4(a: u16) -> u16 { a + 4u16 }").unwrap();
    assert!(
        contains(&add4, &[0x11, 0x04, 0x00, 0x19]),
        "R7 must not touch `+4` (LD DE,4; ADD HL,DE)"
    );
}

#[test]
fn r3_store_reload_exact_bytes() {
    // `let x = a; x`: the reload of `x` right after its store is elided; the store
    // itself must remain (memory is the source of truth for later reads).
    let code = compile_fn("fn f(a: u16) -> u16 { let x = a; x }").unwrap();
    assert_eq!(
        code,
        [
            0x22, 0x00, 0x90, // LD (0x9000),HL   a   (its reload elided too)
            0x22, 0x02, 0x90, // LD (x),HL        (x's reload elided — HL already x)
            0xC9, // RET
        ],
        "R3 did not elide the store-then-reload"
    );
}

#[test]
fn r4_one_arg_call_shape() {
    // 1-arg call: the argument's dead PUSH HL; POP HL pair is gone — the loaded
    // argument flows straight into CALL.
    let prog = compile_program(
        "fn dbl(x: u16) -> u16 { x + x }\n\
         fn f(a: u16) -> u16 { dbl(a) + dbl(a) }",
    )
    .unwrap();
    // `dbl` is laid out first, at ORG (0x8000); `f`'s `a` is slot 1 (0x9002).
    assert!(
        contains(&prog.code, &[0x2A, 0x02, 0x90, 0xCD, 0x00, 0x80]),
        "R4: expected LD HL,(a) directly followed by CALL dbl"
    );
    assert!(
        !contains(&prog.code, &[0xE5, 0xE1]),
        "R4: a dead PUSH HL; POP HL survived"
    );
}

#[test]
fn r5_two_arg_call_shape() {
    // 2-arg call tail: PUSH HL; POP DE; POP HL → EX DE,HL; POP HL, feeding CALL.
    let prog = compile_program(
        "fn sub2(x: u16, y: u16) -> u16 { x - y }\n\
         fn f(a: u16) -> u16 { sub2(a, 7u16) - sub2(a, 9u16) }",
    )
    .unwrap();
    assert!(
        contains(&prog.code, &[0xEB, 0xE1, 0xCD]),
        "R5: expected EX DE,HL; POP HL; CALL"
    );
    assert!(
        !contains(&prog.code, &[0xD1, 0xE1]),
        "R5: a POP DE; POP HL call tail survived"
    );
}

#[test]
fn r6_ex_ex_cancellation_shape() {
    // `100 - arr[i]`: the element load ends EX DE,HL and R1 inserts another —
    // the pair cancels, so the literal load follows LD D,(HL) directly.
    let code =
        compile_fn("fn f(i: u16) -> u16 { let arr = [10u16, 20u16, 30u16]; 100u16 - arr[i] }")
            .unwrap();
    assert!(
        contains(&code, &[0x56, 0x21, 0x64, 0x00]),
        "R6: expected LD D,(HL) directly followed by LD HL,100 (EX;EX cancelled)"
    );
    assert!(
        !contains(&code, &[0xEB, 0xEB]),
        "R6: an EX DE,HL; EX DE,HL pair survived"
    );
}
