//! `&str` parameters — Phase S §2.1. A `&str` param is one register holding the
//! address of a length-prefixed buffer (u16 LE length, bytes at `s + 2`); the
//! accepted methods (`len`/`is_empty`/`as_bytes()[i]`/`is_char_boundary`) are all
//! real Rust with identical semantics, so everything here runs against the rustc
//! oracle via `check_str!`.

use crate::harness::*;

#[test]
fn str_len_and_is_empty() {
    check_str!(s, { s.len() as u16 }, "", "a", "hello world", "héllo");
    check_str!(
        s,
        { s.is_empty() as u16 + 10u16 * (s.len() == 0) as u16 },
        "",
        "x",
        "many bytes here"
    );
}

#[test]
fn str_byte_reads() {
    // First/last byte via `as_bytes()[i]` — including a runtime index off `len()`.
    check_str!(
        s,
        { s.as_bytes()[0] as u16 * 256u16 + s.as_bytes()[s.len() - 1] as u16 },
        "AB",
        "hello",
        "é" // multibyte: first byte 0xC3, last 0xA9
    );
}

#[test]
fn str_char_boundary() {
    // `is_char_boundary` matches std exactly: 0 and len are boundaries, past-len
    // is false, continuation bytes are false.
    check_str!(s, { s.is_char_boundary(0) as u16 }, "", "a", "é");
    check_str!(s, { s.is_char_boundary(1) as u16 }, "a", "é", "ab"); // mid-é → false
    check_str!(s, { s.is_char_boundary(2) as u16 }, "a", "é", "ab"); // past "a" → false
    check_str!(
        s,
        {
            let i = 3;
            s.is_char_boundary(i) as u16
        },
        "héllo", // byte 3 = 'l', a boundary
        "ab"     // past the end → false
    );
}

#[test]
fn str_digit_parse() {
    // The S2 preview: parse_u16 as a plain byte loop over a `&str` — the cell shape
    // the adoption eval is expected to start demanding.
    check_str!(
        s,
        {
            let mut v = 0u16;
            for i in 0..s.len() {
                v = v * 10u16 + (s.as_bytes()[i] - b'0') as u16;
            }
            v
        },
        "0",
        "7",
        "42",
        "65535"
    );
}

#[test]
fn str_ascii_scan() {
    // Classify-and-count: uppercase letters, via byte-literal comparisons.
    check_str!(
        s,
        {
            let mut n = 0u16;
            for i in 0..s.len() {
                let c = s.as_bytes()[i];
                if c >= b'A' && c <= b'Z' {
                    n = n + 1u16;
                }
            }
            n
        },
        "",
        "Hello World",
        "ALLCAPS",
        "none here"
    );
}

#[test]
fn str_passes_between_fns() {
    // A `&str` rides one register across a call — and a string literal argument
    // is just its (length-prefixed) const address. Real Rust both ways.
    let src = r#"
        fn length(s: &str) -> u16 { s.len() as u16 }
        fn run() -> u16 { length("HEY") * 10u16 + length("") }
    "#;
    assert_eq!(run_program(src, "run"), 30);
}

#[test]
fn str_rejections() {
    // Direct indexing steers to `as_bytes()` (it isn't real Rust either)…
    let err = rustz80::compile_fn("fn f(s: &str) -> u16 { s[0] as u16 }")
        .err()
        .unwrap();
    assert!(err.contains("as_bytes"), "unexpected: {err}");

    // …a bare `as_bytes()` value has no home (no slice values)…
    let err = rustz80::compile_fn("fn f(s: &str) -> u16 { let b = s.as_bytes(); 0u16 }")
        .err()
        .unwrap();
    assert!(err.contains("indexed"), "unexpected: {err}");

    // …stores through a string are rejected as read-only…
    let err = rustz80::compile_fn("fn f(s: &str) -> u16 { s.as_bytes()[0] = 1u8; 0u16 }")
        .err()
        .unwrap();
    assert!(err.contains("read-only"), "unexpected: {err}");

    // …and an out-of-dialect method names the accepted surface.
    let err = rustz80::compile_fn("fn f(s: &str) -> u16 { let t = s.trim(); 0u16 }")
        .err()
        .unwrap();
    assert!(err.contains("as_bytes()[i]"), "unexpected: {err}");
}
