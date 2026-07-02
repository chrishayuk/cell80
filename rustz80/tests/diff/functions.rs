//! Function calls and tuple bindings/returns/layout.

use crate::harness::*;

#[test]
fn function_calls() {
    // 1 + 2 args + the calling convention (HL/DE/BC), checked against rustc.
    fn add(a: u16, b: u16) -> u16 {
        a + b
    }
    fn sq(x: u16) -> u16 {
        x * x
    }
    fn f(a: u16, b: u16, c: u16) -> u16 {
        a + b * c
    }
    fn main_host() -> u16 {
        add(40, 2) + sq(5) - f(1, 2, 3)
    }

    let src = "
        fn add(a: u16, b: u16) -> u16 { a + b }
        fn sq(x: u16) -> u16 { x * x }
        fn f(a: u16, b: u16, c: u16) -> u16 { a + b * c }
        fn run() -> u16 { add(40u16, 2u16) + sq(5u16) - f(1u16, 2u16, 3u16) }
    ";
    let prog = rustz80::compile_program(src).expect("compile");
    assert_eq!(run_program(&prog, "run"), main_host()); // 42 + 25 - 7 = 60
}

#[test]
fn tuples() {
    // Multiple return values via tuples (in HL/DE/BC), destructured at the call site.
    fn divmod(a: u16, b: u16) -> (u16, u16) {
        (a / b, a % b)
    }
    fn minmax(a: u16, b: u16) -> (u16, u16) {
        let mut lo = a;
        let mut hi = b;
        if a > b {
            lo = b;
            hi = a;
        }
        (lo, hi)
    }
    fn three() -> (u16, u16, u16) {
        (1, 2, 3)
    }
    fn host() -> u16 {
        let (q, r) = divmod(1000, 7); // (142, 6)
        let (x, y) = (7u16, 3u16); // tuple-literal destructure
        let (lo, hi) = minmax(x, y); // (3, 7)
        let (a, b, c) = three(); // 3-tuple return
        q * 100 + r + hi * 10 + lo + a * 100 + b * 10 + c
    }
    let src = "
        fn divmod(a: u16, b: u16) -> (u16, u16) { (a / b, a % b) }
        fn minmax(a: u16, b: u16) -> (u16, u16) {
            let mut lo = a;
            let mut hi = b;
            if a > b { lo = b; hi = a; }
            (lo, hi)
        }
        fn three() -> (u16, u16, u16) { (1u16, 2u16, 3u16) }
        fn run() -> u16 {
            let (q, r) = divmod(1000u16, 7u16);
            let (x, y) = (7u16, 3u16);
            let (lo, hi) = minmax(x, y);
            let (a, b, c) = three();
            q * 100u16 + r + hi * 10u16 + lo + a * 100u16 + b * 10u16 + c
        }
    ";
    let prog = rustz80::compile_program(src).expect("compile");
    // 14206 + 73 + 123 = 14402
    assert_eq!(run_program(&prog, "run"), host());
}

#[test]
fn tuple_layout() {
    // A tuple return lands in HL/DE/BC in order — verify the register layout directly.
    let src = "
        fn pair() -> (u16, u16) { (42u16, 7u16) }
        fn triple() -> (u16, u16, u16) { (11u16, 22u16, 33u16) }
    ";
    let prog = rustz80::compile_program(src).expect("compile");
    let [hl, de, _] = run_program_regs(&prog, "pair");
    assert_eq!((hl, de), (42, 7), "2-tuple → (HL, DE)");
    assert_eq!(
        run_program_regs(&prog, "triple"),
        [11, 22, 33],
        "3-tuple → (HL, DE, BC)"
    );
}

#[test]
fn tuple_struct_fields() {
    // A struct with a tuple field, accessed by `.0`/`.1` — by value and through a
    // `&mut self` receiver.
    struct Sprite {
        pos: (u16, u16),
        id: u16,
    }
    impl Sprite {
        fn mv(&mut self, dx: u16, dy: u16) {
            self.pos.0 = self.pos.0 + dx;
            self.pos.1 = self.pos.1 + dy;
        }
        fn key(&self) -> u16 {
            self.pos.0 * 100 + self.pos.1 + self.id
        }
    }
    fn host() -> u16 {
        let mut s = Sprite { pos: (3, 4), id: 7 };
        s.mv(2, 5); // pos = (5, 9)
        s.key() // 5*100 + 9 + 7 = 516
    }
    let src = "
        struct Sprite { pos: (u16, u16), id: u16 }
        impl Sprite {
            fn mv(&mut self, dx: u16, dy: u16) {
                self.pos.0 = self.pos.0 + dx;
                self.pos.1 = self.pos.1 + dy;
            }
            fn key(&self) -> u16 { self.pos.0 * 100u16 + self.pos.1 + self.id }
        }
        fn run() -> u16 {
            let mut s = Sprite { pos: (3u16, 4u16), id: 7u16 };
            s.mv(2u16, 5u16);
            s.key()
        }
    ";
    let prog = rustz80::compile_program(src).expect("compile");
    assert_eq!(run_program(&prog, "run"), host()); // 516
}

#[test]
fn tuple_rejections() {
    // More than three return values has no register convention.
    assert!(rustz80::compile_program(
        "fn f() -> (u16, u16, u16, u16) { (1u16, 2u16, 3u16, 4u16) } fn run() -> u16 { 0u16 }"
    )
    .is_err());
    // A tuple binding needs a tuple literal or a function call as its RHS.
    assert!(rustz80::compile_fn("fn f() -> u16 { let (a, b) = 5u16; a + b }").is_err());
}
