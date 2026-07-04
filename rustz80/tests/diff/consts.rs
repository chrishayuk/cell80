//! Program `const` items — scalar substitution and the **const-data section**
//! (`&CONST → addr`): tile/table bytes packed into the image, addressed
//! symbolically, read through `&[T; N]` params, `CONST[i]`, and `peek`.

use crate::harness::*;

#[test]
fn scalar_consts_substitute() {
    // Scalar consts are compile-time values (u16/u8/i16/bool), including one
    // const referencing an earlier one — checked against rustc.
    const SPEED: u16 = 3;
    const LIVES: u8 = 5;
    const BONUS: u16 = SPEED;
    fn host() -> u16 {
        SPEED * 100 + LIVES as u16 * 10 + BONUS
    }
    let src = "
        const SPEED: u16 = 3;
        const LIVES: u8 = 5;
        const BONUS: u16 = SPEED;
        fn run() -> u16 { SPEED * 100u16 + LIVES as u16 * 10u16 + BONUS }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn const_byte_array_indexes() {
    // `[u8; N]` const data: literal and runtime indices, summed in a loop —
    // the tile-row read pattern, checked against rustc.
    const TILE: [u8; 8] = [0x3C, 0x42, 0x81, 0x81, 0x81, 0x81, 0x42, 0x3C];
    fn host() -> u16 {
        let mut sum = 0u16;
        for i in 0..8u16 {
            sum = sum + TILE[i as usize] as u16;
        }
        sum + TILE[0] as u16
    }
    let src = "
        const TILE: [u8; 8] = [0x3Cu8, 0x42u8, 0x81u8, 0x81u8, 0x81u8, 0x81u8, 0x42u8, 0x3Cu8];
        fn run() -> u16 {
            let mut sum = 0u16;
            for i in 0..8u16 {
                sum = sum + TILE[i as usize] as u16;
            }
            sum + TILE[0] as u16
        }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn const_word_arrays_index() {
    // `[u16; N]` (LE word stride) and `[i16; N]` (signed element) const data.
    const FREQ: [u16; 4] = [262, 294, 330, 349];
    const DELTA: [i16; 3] = [-2, 0, 2];
    fn host() -> u16 {
        let d = DELTA[0]; // -2
        FREQ[2].wrapping_add(d as u16) // 330 - 2 = 328
    }
    let src = "
        const FREQ: [u16; 4] = [262u16, 294u16, 330u16, 349u16];
        const DELTA: [i16; 3] = [-2i16, 0i16, 2i16];
        fn run() -> u16 {
            let d = DELTA[0];
            FREQ[2].wrapping_add(d as u16)
        }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn ref_array_param_reads_const() {
    // The tile-blit shape: a helper takes `t: &[u8; 8]` (a read-only pointer to
    // packed bytes) and the caller passes `&TILE` — real Rust both ways.
    const TILE: [u8; 8] = [1, 2, 4, 8, 16, 32, 64, 128];
    fn pick(t: &[u8; 8], i: u16) -> u16 {
        t[i as usize] as u16
    }
    fn host() -> u16 {
        pick(&TILE, 3) * 100 + pick(&TILE, 7)
    }
    let src = "
        const TILE: [u8; 8] = [1u8, 2u8, 4u8, 8u8, 16u8, 32u8, 64u8, 128u8];
        fn pick(t: &[u8; 8], i: u16) -> u16 { t[i as usize] as u16 }
        fn run() -> u16 { pick(&TILE, 3u16) * 100u16 + pick(&TILE, 7u16) }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn ref_word_array_param() {
    // `&[u16; N]` params read word elements (stride 2) through the pointer.
    const NOTES: [u16; 3] = [300, 400, 500];
    fn second(t: &[u16; 3]) -> u16 {
        t[1]
    }
    fn host() -> u16 {
        second(&NOTES)
    }
    let src = "
        const NOTES: [u16; 3] = [300u16, 400u16, 500u16];
        fn second(t: &[u16; 3]) -> u16 { t[1] }
        fn run() -> u16 { second(&NOTES) }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn let_binds_const_ref() {
    // `let t = &CONST;` binds a read-only pointer; `t[i]` loads through it.
    const T: [u8; 4] = [10, 20, 30, 40];
    fn host() -> u16 {
        let t = &T;
        t[2] as u16 + t[0] as u16
    }
    let src = "
        const T: [u8; 4] = [10u8, 20u8, 30u8, 40u8];
        fn run() -> u16 {
            let t = &T;
            t[2] as u16 + t[0] as u16
        }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn struct_const_packs_bytes() {
    // A struct const (`Tile { rows: [u8; 8] }`) packs byte-for-byte into the
    // image at its symbol — the `speccy-assets` baked-tile format.
    let src = "
        struct Tile { rows: [u8; 8] }
        const HERO: Tile = Tile { rows: [9u8, 8u8, 7u8, 6u8, 5u8, 4u8, 3u8, 2u8] };
        fn row(t: u16, i: u16) -> u16 { peek(t + i) as u16 }
        fn run() -> u16 { row(&HERO, 0u16) * 100u16 + row(&HERO, 7u16) }
    ";
    // 9*100 + 2 — the packed rows read back by address. (Dialect-only: the
    // helper takes the address as u16, the routed-prelude shape.)
    assert_eq!(run_program(src, "run"), 902);

    // And the raw bytes sit at the symbol, exactly as declared.
    let prog = rustz80::compile_program(src).expect("compiles");
    let addr = prog.symbols["HERO"] - rustz80::ORG;
    assert_eq!(
        &prog.code[addr as usize..addr as usize + 8],
        &[9, 8, 7, 6, 5, 4, 3, 2]
    );
}

#[test]
fn struct_array_const_elements_address() {
    // `[Tile; N]` — `&SHEET[i]` addresses element `i` at the packed stride.
    let src = "
        struct Tile { rows: [u8; 8] }
        const SHEET: [Tile; 2] = [
            Tile { rows: [1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8, 1u8] },
            Tile { rows: [7u8, 6u8, 5u8, 4u8, 3u8, 2u8, 1u8, 0u8] },
        ];
        fn first_row(t: u16) -> u16 { peek(t) as u16 }
        fn run() -> u16 {
            let i = 1u16;
            first_row(&SHEET[i]) * 10u16 + first_row(&SHEET[0])
        }
    ";
    assert_eq!(run_program(src, "run"), 71); // tile1.rows[0]=7, tile0.rows[0]=1
}

#[test]
fn string_literals_intern_length_prefixed() {
    // A string literal argument becomes a pointer to length-prefixed bytes — a
    // little-endian u16 length at `s` (the Phase S wire format), byte `i` at
    // `s + 2 + i`. Duplicates intern once.
    let src = r#"
        fn len_of(s: u16) -> u16 { peek(s) as u16 + ((peek(s + 1u16) as u16) << 8) }
        fn char_at(s: u16, i: u16) -> u16 { peek(s + 2u16 + i) as u16 }
        fn run() -> u16 {
            len_of("SCORE") * 1000u16 + char_at("SCORE", 0u16)
        }
    "#;
    // len 5 → 5000, 'S' = 83 → 5083.
    assert_eq!(run_program(src, "run"), 5083);

    // Both uses of "SCORE" share one interned pool entry.
    let prog = rustz80::compile_program(src).expect("compiles");
    assert!(prog.symbols.contains_key("__str0"));
    assert!(!prog.symbols.contains_key("__str1"));
}

#[test]
fn str_const_is_an_address() {
    // A `&str` const's bare name is its (length-prefixed) address.
    let src = r#"
        const MSG: &str = "HI";
        fn len_of(s: u16) -> u16 { peek(s) as u16 + ((peek(s + 1u16) as u16) << 8) }
        fn run() -> u16 { len_of(MSG) }
    "#;
    assert_eq!(run_program(src, "run"), 2);
}

#[test]
fn unreferenced_consts_are_pruned() {
    // A const nothing addresses is dropped from the image (data DCE); the
    // referenced one is kept.
    let src = "
        const USED: [u8; 4] = [1u8, 2u8, 3u8, 4u8];
        const UNUSED: [u8; 200] = [0xEEu8; 200];
        fn run() -> u16 { USED[3] as u16 }
    ";
    let prog = rustz80::compile_program(src).expect("compiles");
    assert!(prog.symbols.contains_key("USED"));
    assert!(!prog.symbols.contains_key("UNUSED"));
    assert!(
        prog.code.len() < 200,
        "the 200-byte unused const must not ship"
    );
    assert_eq!(run_program(src, "run"), 4);
}

#[test]
fn frame_loop_lays_data_section() {
    // The game path (`lower_program_full` + `codegen_loop_full`) lays const data
    // into the frame-loop image — the SDK's `Frame::tile(&HERO, …)` route.
    let src = "
        const TILE: [u8; 8] = [0xAAu8, 0x55u8, 0xAAu8, 0x55u8, 0xAAu8, 0x55u8, 0xAAu8, 0x55u8];
        struct G { i: u16 }
        impl G {
            fn update(&mut self) {
                poke(16384u16 + self.i, TILE[(self.i % 8u16) as usize]);
                self.i = self.i + 1u16;
            }
        }
    ";
    let file: syn::File = syn::parse_str(src).unwrap();
    let lowered =
        rustz80::lower_program_full(&file, &rustz80::PreludeConfig::default()).expect("lowers");
    let code = rustz80::codegen_loop_full(&lowered, rustz80::ORG, "G::update", 0xB000, 2)
        .expect("frame loop compiles");
    // The packed tile bytes are laid (contiguously) somewhere in the image.
    let pat = [0xAAu8, 0x55, 0xAA, 0x55, 0xAA, 0x55, 0xAA, 0x55];
    assert!(
        code.windows(8).any(|w| w == pat),
        "const tile bytes missing from the frame-loop image"
    );
}

#[test]
fn const_misuse_is_rejected() {
    let no = |src: &str, needle: &str| {
        let e = match rustz80::compile_program(src) {
            Err(e) => e,
            Ok(_) => panic!("must not compile: {src}"),
        };
        assert!(e.contains(needle), "error `{e}` should mention `{needle}`");
    };
    // Assigning to const data.
    no(
        "const T: [u8; 2] = [1u8, 2u8];
        fn run() -> u16 { T[0] = 5u8; 0u16 }",
        "cannot assign to const data",
    );
    // Borrowing a non-const.
    no(
        "fn run() -> u16 { let x = 1u16; f(&x) } fn f(p: u16) -> u16 { p }",
        "borrows only const data",
    );
    // A literal element address out of bounds.
    no(
        "struct Tile { rows: [u8; 8] }
        const S: [Tile; 2] = [Tile { rows: [0u8; 8] }, Tile { rows: [0u8; 8] }];
        fn run() -> u16 { f(&S[2]) } fn f(p: u16) -> u16 { p }",
        "out of bounds",
    );
    // Bare data-const name as a value.
    no(
        "const T: [u8; 2] = [1u8, 2u8];
        fn run() -> u16 { let x = T; 0u16 }",
        "index it",
    );
}

#[test]
fn const_refs_walk_every_statement_shape() {
    // Data-const DCE must find `ConstAddr` references wherever they hide — every
    // statement/expression position the walker covers. Behaviour vs rustc, and
    // both consts must survive DCE (a missed reference would drop the data).
    let src = "
        const T: [u16; 4] = [3u16, 1u16, 4u16, 1u16];
        const B: [u8; 2] = [10u8, 20u8];
        struct S { acc: u32, arr: [u16; 2], n: u16 }
        impl S {
            fn step(&mut self) -> u16 {
                self.acc = self.acc + T[3] as u32;
                self.arr[T[1]] = B[0] as u16;
                self.n = self.n + self.arr[1];
                self.n
            }
        }
        fn run() -> u16 {
            let mut s = S { acc: 0u32, arr: [T[0]; 2], n: 0u16 };
            let mut acc = 0u16;
            if T[0] > 2u16 {
                acc = acc + B[1] as u16;
            }
            while acc < T[2] {
                acc = acc + T[1];
            }
            for i in 0..T[1] {
                acc = acc ^ (B[0] as u16 + i);
            }
            loop {
                acc = acc + s.step();
                if acc > T[2] * 4u16 {
                    break;
                }
            }
            if acc == 9999u16 {
                return T[0];
            }
            let wide = (T[3] as u32) << 4u32;
            let out = match acc & 1u16 {
                0u16 => acc + T[0] + (wide & 0xFu32) as u16,
                _ => acc + B[1] as u16,
            };
            out
        }
    ";
    struct S {
        acc: u32,
        arr: [u16; 2],
        n: u16,
    }
    const T: [u16; 4] = [3, 1, 4, 1];
    const B: [u8; 2] = [10, 20];
    impl S {
        fn step(&mut self) -> u16 {
            self.acc += T[3] as u32;
            self.arr[T[1] as usize] = B[0] as u16;
            self.n += self.arr[1];
            self.n
        }
    }
    fn host() -> u16 {
        let mut s = S {
            acc: 0,
            arr: [T[0]; 2],
            n: 0,
        };
        let mut acc = 0u16;
        if T[0] > 2 {
            acc += B[1] as u16;
        }
        while acc < T[2] {
            acc += T[1];
        }
        for i in 0..T[1] {
            acc ^= B[0] as u16 + i;
        }
        loop {
            acc += s.step();
            if acc > T[2] * 4 {
                break;
            }
        }
        if acc == 9999 {
            return T[0];
        }
        let wide = (T[3] as u32) << 4;
        match acc & 1 {
            0 => acc + T[0] + (wide & 0xF) as u16,
            _ => acc + B[1] as u16,
        }
    }
    assert_eq!(run_program(src, "run"), host());
    let prog = rustz80::compile_program(src).expect("compiles");
    assert!(prog.symbols.contains_key("T") && prog.symbols.contains_key("B"));
}
