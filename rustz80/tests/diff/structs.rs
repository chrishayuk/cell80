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
