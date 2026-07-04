//! Structs, methods (`&mut self`), and struct-element arrays.

use crate::harness::*;

#[test]
fn structs() {
    // A struct literal, field reads/writes, and a struct passed across functions
    // by mutating fields locally — checked against rustc.
    struct Point {
        x: u16,
        y: u16,
    }
    fn host() -> u16 {
        let mut p = Point { x: 3, y: 4 };
        p.x = p.x + 10;
        p.y = p.y * 2;
        p.x * 100 + p.y // 13*100 + 8 = 1308
    }
    let src = "
        struct Point { x: u16, y: u16 }
        fn run() -> u16 {
            let mut p = Point { x: 3u16, y: 4u16 };
            p.x = p.x + 10u16;
            p.y = p.y * 2u16;
            p.x * 100u16 + p.y
        }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn structs_compose_with_functions() {
    // Pass scalar fields into functions and combine the results.
    struct V {
        x: u16,
        y: u16,
    }
    fn area(w: u16, h: u16) -> u16 {
        w * h
    }
    fn host() -> u16 {
        let a = V { x: 6, y: 7 };
        let b = V { x: 3, y: 4 };
        area(a.x, a.y) + area(b.x, b.y) // 42 + 12 = 54
    }
    let src = "
        struct V { x: u16, y: u16 }
        fn area(w: u16, h: u16) -> u16 { w * h }
        fn run() -> u16 {
            let a = V { x: 6u16, y: 7u16 };
            let b = V { x: 3u16, y: 4u16 };
            area(a.x, a.y) + area(b.x, b.y)
        }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn methods_and_self() {
    // `&mut self` mutation through a pointer, plus a `&self` reader.
    struct Counter {
        n: u16,
    }
    impl Counter {
        fn bump(&mut self, by: u16) {
            self.n = self.n + by;
        }
        fn doubled(&self) -> u16 {
            self.n + self.n
        }
    }
    fn host() -> u16 {
        let mut c = Counter { n: 10 };
        c.bump(5);
        c.bump(7);
        c.doubled() // (10+5+7)*2 = 44
    }
    let src = "
        struct Counter { n: u16 }
        impl Counter {
            fn bump(&mut self, by: u16) { self.n = self.n + by; }
            fn doubled(&self) -> u16 { self.n + self.n }
        }
        fn run() -> u16 {
            let mut c = Counter { n: 10u16 };
            c.bump(5u16);
            c.bump(7u16);
            c.doubled()
        }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn methods_call_self_and_two_structs() {
    // A method calling another method on `self`, and two structs sharing a name.
    struct Vec2 {
        x: u16,
        y: u16,
    }
    impl Vec2 {
        fn sum(&self) -> u16 {
            self.x + self.y
        }
        fn scaled_sum(&self, k: u16) -> u16 {
            self.sum() * k
        }
    }
    struct Sq {
        w: u16,
    }
    impl Sq {
        fn area(&self) -> u16 {
            self.w * self.w
        }
    }
    fn host() -> u16 {
        let v = Vec2 { x: 3, y: 4 };
        let b = Sq { w: 5 };
        v.scaled_sum(10) + b.area() // 7*10 + 25 = 95
    }
    let src = "
        struct Vec2 { x: u16, y: u16 }
        impl Vec2 {
            fn sum(&self) -> u16 { self.x + self.y }
            fn scaled_sum(&self, k: u16) -> u16 { self.sum() * k }
        }
        struct Sq { w: u16 }
        impl Sq { fn area(&self) -> u16 { self.w * self.w } }
        fn run() -> u16 {
            let v = Vec2 { x: 3u16, y: 4u16 };
            let b = Sq { w: 5u16 };
            v.scaled_sum(10u16) + b.area()
        }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn struct_element_arrays() {
    // A local array of structs: `[Cell; N]`. Elements are multi-slot, addressed
    // `&a + i*stride (+ field_off)`. Same source type-checks under rustc.
    #[derive(Clone, Copy)]
    struct Cell {
        x: u16,
        y: u16,
    }
    fn host() -> u16 {
        let mut a = [Cell { x: 0, y: 0 }; 4];
        let mut i = 0u16;
        while i < 4 {
            a[i as usize] = Cell {
                x: i + 1,
                y: (i + 1) * 10,
            };
            i = i + 1;
        }
        a[0].x = 99; // overwrite one field
        let mut total = 0u16;
        let mut j = 0u16;
        while j < 4 {
            total = total + a[j as usize].x + a[j as usize].y;
            j = j + 1;
        }
        total
    }
    let src = "
        struct Cell { x: u16, y: u16 }
        fn run() -> u16 {
            let mut a = [Cell { x: 0u16, y: 0u16 }; 4];
            let mut i = 0u16;
            while i < 4u16 {
                a[i as usize] = Cell { x: i + 1u16, y: (i + 1u16) * 10u16 };
                i = i + 1u16;
            }
            a[0].x = 99u16;
            let mut total = 0u16;
            let mut j = 0u16;
            while j < 4u16 {
                total = total + a[j as usize].x + a[j as usize].y;
                j = j + 1u16;
            }
            total
        }
    ";
    // a = {99,10},{2,20},{3,30},{4,40}  → 109 + 22 + 33 + 44 = 208
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn struct_field_struct_arrays() {
    // A struct whose field is an array of structs (`[Cell; N]`) — the `Entities<Cell,
    // N>` shape. Methods access `self.cells[i].x` (through the receiver pointer) and
    // store whole elements `self.cells[i] = Cell { … }`. Same source under rustc.
    #[derive(Clone, Copy)]
    struct Cell {
        x: u16,
        y: u16,
    }
    struct Body {
        cells: [Cell; 4],
        len: u16,
    }
    impl Body {
        fn add(&mut self, x: u16, y: u16) {
            self.cells[self.len as usize] = Cell { x, y };
            self.len = self.len + 1;
        }
        fn checksum(&self) -> u16 {
            let mut s = 0u16;
            let mut i = 0u16;
            while i < self.len {
                s = s + self.cells[i as usize].x * 100 + self.cells[i as usize].y;
                i = i + 1;
            }
            s
        }
    }
    fn host() -> u16 {
        let mut b = Body {
            cells: [Cell { x: 0, y: 0 }; 4],
            len: 0,
        };
        b.add(1, 2);
        b.add(3, 4);
        b.add(5, 6);
        b.checksum() + b.cells[0].x // 912 + 1
    }
    let src = "
        struct Cell { x: u16, y: u16 }
        struct Body { cells: [Cell; 4], len: u16 }
        impl Body {
            fn add(&mut self, x: u16, y: u16) {
                self.cells[self.len as usize] = Cell { x: x, y: y };
                self.len = self.len + 1u16;
            }
            fn checksum(&self) -> u16 {
                let mut s = 0u16;
                let mut i = 0u16;
                while i < self.len {
                    s = s + self.cells[i as usize].x * 100u16 + self.cells[i as usize].y;
                    i = i + 1u16;
                }
                s
            }
        }
        fn run() -> u16 {
            let mut b = Body { cells: [Cell { x: 0u16, y: 0u16 }; 4], len: 0u16 };
            b.add(1u16, 2u16);
            b.add(3u16, 4u16);
            b.add(5u16, 6u16);
            b.checksum() + b.cells[0].x
        }
    ";
    assert_eq!(run_program(src, "run"), host()); // 912 + 1 = 913
}

#[test]
fn packed_u8_field_bytes() {
    // A `[u8; N]` field is **byte-packed** (Phase S §2.3): N bytes in ceil(N/2)
    // slots. Reads/writes are real u8 semantics — checked against rustc, with a
    // field *after* the array proving the packed offset arithmetic.
    struct Slug {
        buf: [u8; 5], // odd length: 3 slots, one padding byte
        tail: u16,
    }
    fn host() -> u16 {
        let mut s = Slug {
            buf: [1, 2, 3, 4, 5],
            tail: 700,
        };
        s.buf[0] = s.buf[4] + 250; // u8 wrap: 255
        s.buf[3] = b'x';
        s.buf[0] as u16 + s.buf[3] as u16 + s.tail
    }
    let src = "
        struct Slug { buf: [u8; 5], tail: u16 }
        fn run() -> u16 {
            let mut s = Slug { buf: [1u8, 2u8, 3u8, 4u8, 5u8], tail: 700u16 };
            s.buf[0] = s.buf[4] + 250u8;
            s.buf[3] = b'x';
            s.buf[0] as u16 + s.buf[3] as u16 + s.tail
        }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn packed_u8_field_via_method() {
    // The output-buffer shape from the Phase S spec: a method (`&mut self`
    // pointer receiver) appends bytes and bumps a length — the `str_out` idiom.
    struct Out {
        len: u16,
        buf: [u8; 8],
    }
    impl Out {
        fn push(&mut self, b: u8) {
            self.buf[self.len as usize] = b;
            self.len += 1;
        }
        fn run(&mut self) -> u16 {
            self.push(b'h');
            self.push(b'i');
            self.buf[0] as u16 * 256 + self.buf[1] as u16 + self.len * 10
        }
    }
    fn host() -> u16 {
        let mut o = Out {
            len: 0,
            buf: [0; 8],
        };
        o.run()
    }
    let src = "
        struct Out { len: u16, buf: [u8; 8] }
        impl Out {
            fn push(&mut self, b: u8) {
                self.buf[self.len] = b;
                self.len = self.len + 1u16;
            }
            fn run(&mut self) -> u16 {
                self.push(b'h');
                self.push(b'i');
                self.buf[0] as u16 * 256u16 + self.buf[1] as u16 + self.len * 10u16
            }
        }
        fn run() -> u16 {
            let mut o = Out { len: 0u16, buf: [0u8; 8] };
            o.run()
        }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn packed_u8_field_repeat_init() {
    // `[v; N]` fill on a packed field: every byte set, tail field untouched.
    struct B {
        buf: [u8; 4],
        tail: u16,
    }
    fn host() -> u16 {
        let mut b = B {
            buf: [0xAB; 4],
            tail: 9,
        };
        b.buf[2] = 1;
        b.buf[0] as u16 + b.buf[1] as u16 + b.buf[2] as u16 + b.buf[3] as u16 + b.tail
    }
    let src = "
        struct B { buf: [u8; 4], tail: u16 }
        fn run() -> u16 {
            let mut b = B { buf: [0xABu8; 4], tail: 9u16 };
            b.buf[2] = 1u8;
            b.buf[0] as u16 + b.buf[1] as u16 + b.buf[2] as u16 + b.buf[3] as u16 + b.tail
        }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn packed_u8_field_layout() {
    // The layout contract: ceil(N/2) slots, `bytes` reported, following fields
    // shifted accordingly — and a [u8; 2] field is never mistaken for a u16 scalar.
    let src = "
        struct S { a: [u8; 5], b: u16, c: [u8; 2], d: u32 }
        impl S { fn run(&mut self) -> u16 { self.b } }
    ";
    let l = rustz80::struct_layout(src, "S").expect("layout");
    let by_name = |n: &str| l.iter().find(|f| f.name == n).unwrap();
    assert_eq!(
        (by_name("a").slots, by_name("a").bytes, by_name("a").offset),
        (3, Some(5), 0)
    );
    assert_eq!((by_name("b").slots, by_name("b").offset), (1, 3));
    assert_eq!((by_name("c").slots, by_name("c").bytes), (1, Some(2)));
    assert_eq!((by_name("d").offset, by_name("d").dword), (5, true));
}
