//! Generic fns/structs/methods and monomorphization.

use crate::harness::*;

#[test]
fn generics() {
    // Real generic Rust — monomorphized per call. `clamp` is generic *and* calls two
    // other generics, so one `clamp(_u16)` transitively instantiates `max`/`min` too.
    // The same source type-checks under rustc (bounds satisfied) and compiles here.
    fn max<T: Ord + Copy>(a: T, b: T) -> T {
        let mut r = a;
        if b > a {
            r = b;
        }
        r
    }
    fn min<T: Ord + Copy>(a: T, b: T) -> T {
        let mut r = a;
        if b < a {
            r = b;
        }
        r
    }
    fn clamp<T: Ord + Copy>(x: T, lo: T, hi: T) -> T {
        min(max(x, lo), hi)
    }
    fn host() -> u16 {
        let a = clamp(50u16, 10, 40); // 40
        let b = clamp(5u16, 10, 40); // 10
        let u = clamp(200u8, 50, 150); // 150 (a u8 instance)
        a + b + u as u16
    }
    let src = "
        fn max<T: Ord + Copy>(a: T, b: T) -> T { let mut r = a; if b > a { r = b; } r }
        fn min<T: Ord + Copy>(a: T, b: T) -> T { let mut r = a; if b < a { r = b; } r }
        fn clamp<T: Ord + Copy>(x: T, lo: T, hi: T) -> T { min(max(x, lo), hi) }
        fn run() -> u16 {
            let a = clamp(50u16, 10u16, 40u16);
            let b = clamp(5u16, 10u16, 40u16);
            let u = clamp(200u8, 50u8, 150u8);
            a + b + u as u16
        }
    ";
    let prog = rustz80::compile_program(src).expect("compile");
    assert_eq!(run_program(&prog, "run"), host()); // 40 + 10 + 150 = 200

    // The type argument is inferred (no turbofish), and `clamp` was instantiated at
    // both u16 and u8 — distinct monomorphic copies, each pulling in max/min.
    for sym in ["clamp$u16", "clamp$u8", "max$u16", "min$u8"] {
        assert!(prog.symbols.contains_key(sym), "missing instance {sym}");
    }
}

#[test]
fn const_generics() {
    // A const generic parameter sizes a local array and bounds the loops; each
    // `::<N>` instantiates a specialized copy. The same source type-checks under rustc.
    fn sum_to<const N: usize>() -> u16 {
        let mut a = [0u16; N];
        let mut i = 0usize;
        while i < N {
            a[i] = (i + 1) as u16;
            i = i + 1;
        }
        let mut s = 0u16;
        let mut j = 0usize;
        while j < N {
            s = s + a[j];
            j = j + 1;
        }
        s
    }
    fn host() -> u16 {
        sum_to::<4>() + sum_to::<6>()
    }
    let src = "
        fn sum_to<const N: usize>() -> u16 {
            let mut a = [0u16; N];
            let mut i = 0usize;
            while i < N { a[i] = (i + 1) as u16; i = i + 1; }
            let mut s = 0u16;
            let mut j = 0usize;
            while j < N { s = s + a[j]; j = j + 1; }
            s
        }
        fn run() -> u16 { sum_to::<4>() + sum_to::<6>() }
    ";
    let prog = rustz80::compile_program(src).expect("compile");
    assert_eq!(run_program(&prog, "run"), host()); // 10 + 21 = 31
                                                   // Distinct const instances, named by value.
    assert!(prog.symbols.contains_key("sum_to$4"));
    assert!(prog.symbols.contains_key("sum_to$6"));
}

#[test]
fn const_generic_entities() {
    // The full `Entities<Cell, const N>` shape: a const-generic struct whose field is an
    // array of structs (`data: [Cell; N]`), with methods that bound on `N`, store whole
    // elements, and read element fields. `N` is inferred from the literal's array length.
    #[derive(Clone, Copy)]
    struct Cell {
        x: u16,
        y: u16,
    }
    struct Entities<const N: usize> {
        data: [Cell; N],
        len: u16,
    }
    impl<const N: usize> Entities<N> {
        fn add(&mut self, x: u16, y: u16) {
            if self.len < N as u16 {
                self.data[self.len as usize] = Cell { x, y };
                self.len = self.len + 1;
            }
        }
        fn checksum(&self) -> u16 {
            let mut s = 0u16;
            let mut i = 0u16;
            while i < self.len {
                s = s + self.data[i as usize].x * 100 + self.data[i as usize].y;
                i = i + 1;
            }
            s
        }
    }
    fn host() -> u16 {
        let mut e = Entities {
            data: [Cell { x: 0, y: 0 }; 4],
            len: 0,
        };
        e.add(1, 2);
        e.add(3, 4);
        e.add(5, 6);
        e.add(7, 8); // 5th add — capacity is 4, ignored
        e.checksum()
    }
    let src = "
        struct Cell { x: u16, y: u16 }
        struct Entities<const N: usize> { data: [Cell; N], len: u16 }
        impl<const N: usize> Entities<N> {
            fn add(&mut self, x: u16, y: u16) {
                if self.len < N as u16 {
                    self.data[self.len as usize] = Cell { x: x, y: y };
                    self.len = self.len + 1u16;
                }
            }
            fn checksum(&self) -> u16 {
                let mut s = 0u16;
                let mut i = 0u16;
                while i < self.len {
                    s = s + self.data[i as usize].x * 100u16 + self.data[i as usize].y;
                    i = i + 1u16;
                }
                s
            }
        }
        fn run() -> u16 {
            let mut e = Entities { data: [Cell { x: 0u16, y: 0u16 }; 4], len: 0u16 };
            e.add(1u16, 2u16);
            e.add(3u16, 4u16);
            e.add(5u16, 6u16);
            e.add(7u16, 8u16);
            e.checksum()
        }
    ";
    let prog = rustz80::compile_program(src).expect("compile");
    assert!(prog.symbols.contains_key("Entities$4::add"));
    assert_eq!(run_program(&prog, "run"), host()); // capacity 4: 102+304+506 = 912
}

#[test]
fn const_generic_structs() {
    // A fixed-capacity stack: the const param sizes the `[u16; N]` field *and* bounds
    // `push`. Each `Stack<N>` is a distinct instance (layout + methods). Same source
    // type-checks under rustc.
    struct Stack<const N: usize> {
        data: [u16; N],
        len: u16,
    }
    impl<const N: usize> Stack<N> {
        fn push(&mut self, v: u16) {
            if self.len < N as u16 {
                self.data[self.len as usize] = v;
                self.len = self.len + 1;
            }
        }
        fn sum(&self) -> u16 {
            let mut s = 0u16;
            let mut i = 0u16;
            while i < self.len {
                s = s + self.data[i as usize];
                i = i + 1;
            }
            s
        }
    }
    fn host() -> u16 {
        let mut s: Stack<4> = Stack {
            data: [0; 4],
            len: 0,
        };
        s.push(10);
        s.push(20);
        s.push(30);
        s.push(40);
        s.push(50); // dropped — capacity 4
        s.sum()
    }
    let src = "
        struct Stack<const N: usize> { data: [u16; N], len: u16 }
        impl<const N: usize> Stack<N> {
            fn push(&mut self, v: u16) {
                if self.len < N as u16 {
                    self.data[self.len as usize] = v;
                    self.len = self.len + 1u16;
                }
            }
            fn sum(&self) -> u16 {
                let mut s = 0u16;
                let mut i = 0u16;
                while i < self.len { s = s + self.data[i as usize]; i = i + 1u16; }
                s
            }
        }
        fn run() -> u16 {
            let mut s = Stack { data: [0u16; 4], len: 0u16 };
            s.push(10u16); s.push(20u16); s.push(30u16); s.push(40u16); s.push(50u16);
            s.sum()
        }
    ";
    let prog = rustz80::compile_program(src).expect("compile");
    assert_eq!(run_program(&prog, "run"), host()); // 10+20+30+40 = 100
                                                   // The methods are instantiated per struct instance.
    for sym in ["Stack$4::push", "Stack$4::sum"] {
        assert!(prog.symbols.contains_key(sym), "missing instance {sym}");
    }
}

#[test]
fn generics_substitute_width() {
    // The instantiation's width is real: at u8 the body wraps (mod 256), at u16 it
    // does not — proving monomorphization substitutes the type, not just the name.
    // (`add` is only compiled here, not run on host, so the u8 overflow is fine.)
    let src = "
        fn add<T: core::ops::Add<Output = T> + Copy>(a: T, b: T) -> T { a + b }
        fn at_u16() -> u16 { add(200u16, 100u16) }
        fn at_u8() -> u16 { add(200u8, 100u8) as u16 }
    ";
    let prog = rustz80::compile_program(src).expect("compile");
    assert_eq!(run_program(&prog, "at_u16"), 300); // u16: no wrap
    assert_eq!(run_program(&prog, "at_u8"), 44); // u8: 300 wraps to 44
}

#[test]
fn generic_structs() {
    // A generic struct + generic methods. The same source type-checks under rustc and
    // compiles here (type arguments erased to 16-bit, like any struct's fields).
    struct Pair<T> {
        a: T,
        b: T,
    }
    impl<T: Copy + core::ops::Add<Output = T>> Pair<T> {
        fn sum(&self) -> T {
            self.a + self.b
        }
        fn bump(&mut self, d: T) {
            self.a = self.a + d;
        }
    }
    fn host() -> u16 {
        let mut p = Pair { a: 30u16, b: 12u16 };
        p.bump(3); // a = 33
        let q = Pair { a: 5u16, b: 7u16 };
        p.sum() + q.sum() // 45 + 12 = 57
    }
    let src = "
        struct Pair<T> { a: T, b: T }
        impl<T> Pair<T> {
            fn sum(&self) -> T { self.a + self.b }
            fn bump(&mut self, d: T) { self.a = self.a + d; }
        }
        fn run() -> u16 {
            let mut p = Pair { a: 30u16, b: 12u16 };
            p.bump(3u16);
            let q = Pair { a: 5u16, b: 7u16 };
            p.sum() + q.sum()
        }
    ";
    let prog = rustz80::compile_program(src).expect("compile");
    assert_eq!(run_program(&prog, "run"), host()); // 57
}

#[test]
fn monomorphization() {
    // Two type parameters → a combined instance key; the same instantiation called
    // twice is emitted once (dedup); distinct types make distinct instances.
    fn id<T>(x: T) -> T {
        x
    }
    fn first<A, B>(a: A, _b: B) -> A {
        a
    }
    fn host() -> u16 {
        id(10u16) + id(5u8) as u16 + first(7u16, 2u8) + first(7u16, 2u8)
    }
    let src = "
        fn id<T>(x: T) -> T { x }
        fn first<A, B>(a: A, b: B) -> A { a }
        fn run() -> u16 {
            id(10u16) + id(5u8) as u16 + first(7u16, 2u8) + first(7u16, 2u8)
        }
    ";
    let prog = rustz80::compile_program(src).expect("compile");
    assert_eq!(run_program(&prog, "run"), host()); // 10 + 5 + 7 + 7 = 29

    let instances: Vec<&String> = prog.symbols.keys().filter(|k| k.contains('$')).collect();
    // id at u16 and u8, first at (u16, u8) once — exactly three instances.
    assert_eq!(instances.len(), 3, "instances: {instances:?}");
    for sym in ["id$u16", "id$u8", "first$u16_u8"] {
        assert!(prog.symbols.contains_key(sym), "missing {sym}");
    }
    // `first` called twice with the same types ⇒ a single instance (dedup).
    assert_eq!(
        prog.symbols
            .keys()
            .filter(|k| k.starts_with("first"))
            .count(),
        1
    );
}

#[test]
fn generic_methods() {
    // Methods on a generic struct, including a method calling another on `self`.
    struct Acc<T> {
        v: T,
    }
    impl<T: Copy + core::ops::Add<Output = T> + core::ops::Mul<Output = T>> Acc<T> {
        fn add(&mut self, d: T) {
            self.v = self.v + d;
        }
        fn doubled(&self) -> T {
            self.v + self.v
        }
        fn add_then_double(&mut self, d: T) -> T {
            self.add(d);
            self.doubled()
        }
    }
    fn host() -> u16 {
        let mut a = Acc { v: 10u16 };
        a.add_then_double(5) // (10+5)*2 = 30
    }
    let src = "
        struct Acc<T> { v: T }
        impl<T> Acc<T> {
            fn add(&mut self, d: T) { self.v = self.v + d; }
            fn doubled(&self) -> T { self.v + self.v }
            fn add_then_double(&mut self, d: T) -> T { self.add(d); self.doubled() }
        }
        fn run() -> u16 {
            let mut a = Acc { v: 10u16 };
            a.add_then_double(5u16)
        }
    ";
    let prog = rustz80::compile_program(src).expect("compile");
    assert_eq!(run_program(&prog, "run"), host()); // 30
}
