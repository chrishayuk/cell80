//! Byte literals (`b'a'` → `u8`) and byte-string literals (`b"…"` → packed
//! `[u8; N]` const data, **no** length prefix — the type carries the length).
//! Phase S §2.2: both are real Rust, so everything value-shaped here is checked
//! against rustc.

use crate::harness::*;

#[test]
fn byte_literal_is_u8() {
    // The idiom the feature exists for: ASCII classification without magic numbers.
    check!({
        let c = b'7';
        (c >= b'0' && c <= b'9') as u16
    });
    check!({
        let c = b' ';
        (c == b' ') as u16 + (c == b'_') as u16
    });
}

#[test]
fn byte_literal_arithmetic() {
    // u8 arithmetic and casts behave exactly like `u8`-suffixed ints.
    check!({ (b'Z' - b'A') as u16 });
    check!({
        let d = b'9' - b'0';
        d as u16 * 100u16 + (b'a' as u16)
    });
    // Escapes carry their byte values.
    check!({ b'\n' as u16 * 256u16 + b'\\' as u16 });
}

#[test]
fn byte_literal_match_pattern() {
    // Byte literals as `match` arms, on a byte scrutinee.
    check!({
        let c = b'x';
        match c {
            b'x' => 1u16,
            b'y' => 2u16,
            _ => 0u16,
        }
    });
}

#[test]
fn byte_literal_in_consts() {
    // Scalar consts and const-array elements take byte literals.
    const ZERO: u8 = b'0';
    const HEX: [u8; 3] = [b'a', b'b', b'c'];
    fn host() -> u16 {
        ZERO as u16 + HEX[2] as u16
    }
    let src = "
        const ZERO: u8 = b'0';
        const HEX: [u8; 3] = [b'a', b'b', b'c'];
        fn run() -> u16 { ZERO as u16 + HEX[2] as u16 }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn byte_string_interns_raw() {
    // A `b"…"` literal is packed const data with **no** length prefix — byte 0 is
    // the first content byte (contrast the `"…"` string convention, where
    // `peek(s)` is the length). Duplicates intern once.
    let src = r#"
        fn first(p: u16) -> u16 { peek(p) as u16 }
        fn run() -> u16 {
            let p = b"AB";
            first(p) * 1000u16 + peek(p + 1u16) as u16 + first(b"AB")
        }
    "#;
    // 'A' = 65 → 65000, 'B' = 66, + 65 again = 65131.
    assert_eq!(run_program(src, "run"), 65131);

    let prog = rustz80::compile_program(src).expect("compiles");
    assert!(prog.symbols.contains_key("__bytes0"));
    assert!(!prog.symbols.contains_key("__bytes1"));
}

#[test]
fn byte_string_const_is_an_address() {
    // `const B: &[u8; N] = b"…";` — real Rust; the bare name is the data address,
    // and elements index like any array const. Checked against rustc.
    const HTTP: &[u8; 4] = b"HTTP";
    fn host() -> u16 {
        let i = 3;
        HTTP[0] as u16 * 256 + HTTP[i] as u16
    }
    let src = r#"
        const HTTP: &[u8; 4] = b"HTTP";
        fn run() -> u16 {
            let i = 3u16;
            HTTP[0] as u16 * 256u16 + HTTP[i] as u16
        }
    "#;
    assert_eq!(run_program(src, "run"), host());

    // And the bare name passes as a pointer argument.
    let src = r#"
        const CRLF: &[u8; 2] = b"\r\n";
        fn first(p: u16) -> u16 { peek(p) as u16 }
        fn run() -> u16 { first(CRLF) }
    "#;
    assert_eq!(run_program(src, "run"), 13);
}

#[test]
fn byte_string_rejections() {
    // Length mismatch against the declared `[u8; N]` is a compile error…
    let src = r#"
        const X: &[u8; 3] = b"HTTP";
        fn run() -> u16 { X[0] as u16 }
    "#;
    let err = rustz80::compile_program(src).err().unwrap();
    assert!(err.contains("4 bytes"), "unexpected: {err}");

    // …a non-byte-string initializer is rejected instructively…
    let err = rustz80::compile_program(
        r#"
        const X: &[u8; 2] = b"no";
        const Y: &[u8; 2] = X;
        fn run() -> u16 { Y[0] as u16 }
    "#,
    )
    .err()
    .unwrap();
    assert!(err.contains("byte-string literal"), "unexpected: {err}");

    // …a double borrow of a `&[u8; N]` const names the fix…
    let err = rustz80::compile_program(
        r#"
        const B: &[u8; 2] = b"ok";
        fn f(p: u16) -> u16 { peek(p) as u16 }
        fn run() -> u16 { f(&B) }
    "#,
    )
    .err()
    .unwrap();
    assert!(
        err.contains("reference to a reference"),
        "unexpected: {err}"
    );

    // …and char literals stay out, steering to `b'a'`.
    let err = rustz80::compile_fn("fn f() -> u16 { let c = 'a'; c as u16 }")
        .err()
        .unwrap();
    assert!(err.contains("b'a'"), "unexpected: {err}");
}
