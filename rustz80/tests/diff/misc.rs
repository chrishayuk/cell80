//! Screen-memory output, `entry_signature`, `size_report`, unsupported-item errors.

use crate::harness::*;

#[test]
fn pixels_to_screen() {
    // A `plot()` written in the dialect (div/mod screen math + a mask table),
    // writing pixels through the poke/peek raw-memory intrinsics. Verified against
    // the canonical ZX Spectrum address formula computed independently in Rust.
    let src = "
        fn plot(x: u16, y: u16) {
            let masks = [128u8, 64u8, 32u8, 16u8, 8u8, 4u8, 2u8, 1u8];
            let addr = 16384u16
                + (y / 64u16) * 2048u16
                + (y % 8u16) * 256u16
                + ((y / 8u16) % 8u16) * 32u16
                + x / 8u16;
            let m = masks[(x % 8u16) as usize];
            poke(addr, peek(addr) | m);
        }
        fn run() {
            plot(0u16, 0u16);
            plot(255u16, 191u16);
            plot(128u16, 96u16);
            plot(7u16, 1u16);
            plot(1u16, 100u16);
        }
    ";
    let mem = run_to_memory(src, "run");

    let pixels = [(0u16, 0u16), (255, 191), (128, 96), (7, 1), (1, 100)];
    let mut want = vec![0u8; 0x1_0000];
    for (x, y) in pixels {
        let addr = 0x4000 + ((y & 0xC0) << 5) + ((y & 0x07) << 8) + ((y & 0x38) << 2) + (x >> 3);
        want[addr as usize] |= 0x80 >> (x & 7);
    }
    assert_eq!(
        &mem[0x4000..0x5800],
        &want[0x4000..0x5800],
        "screen bytes differ"
    );
}

#[test]
fn entry_signature_extracts_types() {
    use rustz80::entry_signature;
    // free fn: value params + tuple return.
    let s = entry_signature("fn run(a: u16, b: u8) -> (u16, u16) { (a, a) }", "run").unwrap();
    assert_eq!(
        s.params,
        vec![("a".into(), "u16".into()), ("b".into(), "u8".into())]
    );
    assert_eq!(s.ret, "(u16, u16)");
    assert!(s.state.is_empty());
    assert_eq!(s.to_decl("run"), "run(a: u16, b: u8) -> (u16, u16)");

    // reference params + no return (a game-shaped fn) — exercises the &/&mut + unit arms.
    let g = entry_signature("fn update(frame: &mut Frame, input: &Input) { }", "update").unwrap();
    assert_eq!(
        g.params,
        vec![
            ("frame".into(), "&mut Frame".into()),
            ("input".into(), "&Input".into())
        ]
    );
    assert_eq!(g.ret, "()");

    // method: the receiver struct's fields, incl. array + tuple types.
    let src = "struct S { v: u16, arr: [u16; 4], pt: (u16, u16) }
               impl S { fn run(&mut self) -> u16 { self.v } }";
    let m = entry_signature(src, "S::run").unwrap();
    assert!(m.params.is_empty());
    assert_eq!(m.ret, "u16");
    assert_eq!(
        m.state,
        vec![
            ("v".into(), "u16".into()),
            ("arr".into(), "[u16; 4]".into()),
            ("pt".into(), "(u16, u16)".into()),
        ]
    );
    assert!(entry_signature("fn run() -> u16 { 0u16 }", "nope").is_err());

    // Exotic syntax robustness — `entry_signature` parses arbitrary syn, so cover the
    // fallback arms: a const before the struct, a const-length array field, a slice param
    // (→ `&?`), a tuple-pattern param (→ `_`), and a non-fn impl item.
    let exotic = "const CAP: u16 = 8;
                  struct W { buf: [u16; CAP], tup: (u16, u16) }
                  impl W { const K: u16 = 0; fn run(&mut self, p: &[u16], (m, n): (u16, u16)) -> u16 { 0u16 } }";
    let w = entry_signature(exotic, "W::run").unwrap();
    assert_eq!(
        w.params,
        vec![("p".into(), "&?".into()), ("_".into(), "(u16, u16)".into())]
    );
    assert_eq!(
        w.state,
        vec![
            ("buf".into(), "[u16; CAP]".into()),
            ("tup".into(), "(u16, u16)".into())
        ]
    );
}

#[test]
fn size_report_covers_image() {
    let src = "
        fn id<T>(x: T) -> T { x }
        fn run() -> u16 { id(1u16) + id(2u8) as u16 }
    ";
    let prog = rustz80::compile_program(src).expect("compile");
    let report = prog.size_report();

    // One entry per symbol; sizes tile the whole image with no gaps or overlaps.
    assert_eq!(report.len(), prog.symbols.len());
    let total: usize = report.iter().map(|f| f.size as usize).sum();
    assert_eq!(total, prog.code.len(), "sizes cover the whole image");
    assert!(report.iter().all(|f| f.size > 0), "every fn emits ≥ 1 byte");
    // Monomorphic instances are present and flagged.
    assert!(report.iter().any(|f| f.name == "id$u16" && f.instance));
    assert!(report.iter().any(|f| f.name == "id$u8" && f.instance));
    // Entries are in layout order (ascending address).
    assert!(report.windows(2).all(|w| w[0].addr <= w[1].addr));
}

#[test]
fn unsupported_is_an_error() {
    // f32 is outside the dialect → a clear compile error (the host-only signal).
    assert!(rustz80::compile_fn("fn f() -> u16 { let x = 1.5f32; 0u16 }").is_err());
}
