//! Nested struct fields and field-of-field access (`self.sprite.x`, `a.b.c.d`): a
//! struct-typed field lays out as its sub-struct's whole slot range, access sums
//! offsets down the chain to a scalar/`u32` leaf, and nested struct literals initialise
//! the sub-fields. Each case is checked against rustc on both targets.

use crate::harness::*;

#[test]
fn nested_read_write_by_value() {
    // A 2-level nested struct in a by-value local: read + write through the inner field.
    struct Point {
        x: u16,
        y: u16,
    }
    struct Actor {
        pos: Point,
        hp: u16,
    }
    fn host() -> u16 {
        let mut a = Actor {
            pos: Point { x: 3, y: 4 },
            hp: 100,
        };
        a.pos.x = a.pos.x + 10;
        a.pos.y = a.pos.y * 2;
        a.pos.x * 1000 + a.pos.y * 10 + a.hp / 10 // 13*1000 + 8*10 + 10 = 13090
    }
    let src = "
        struct Point { x: u16, y: u16 }
        struct Actor { pos: Point, hp: u16 }
        fn run() -> u16 {
            let mut a = Actor { pos: Point { x: 3u16, y: 4u16 }, hp: 100u16 };
            a.pos.x = a.pos.x + 10u16;
            a.pos.y = a.pos.y * 2u16;
            a.pos.x * 1000u16 + a.pos.y * 10u16 + a.hp / 10u16
        }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn nested_via_self_methods() {
    // The SDK-kit shape: `&mut self` / `&self` methods drilling into a nested field.
    struct Point {
        x: u16,
        y: u16,
    }
    struct Actor {
        pos: Point,
        hp: u16,
    }
    impl Actor {
        fn move_by(&mut self, dx: u16, dy: u16) {
            self.pos.x = self.pos.x + dx;
            self.pos.y = self.pos.y + dy;
        }
        fn total(&self) -> u16 {
            self.pos.x + self.pos.y + self.hp
        }
    }
    fn host() -> u16 {
        let mut a = Actor {
            pos: Point { x: 1, y: 2 },
            hp: 5,
        };
        a.move_by(10, 20);
        a.total() // 11 + 22 + 5 = 38
    }
    let src = "
        struct Point { x: u16, y: u16 }
        struct Actor { pos: Point, hp: u16 }
        impl Actor {
            fn move_by(&mut self, dx: u16, dy: u16) {
                self.pos.x = self.pos.x + dx;
                self.pos.y = self.pos.y + dy;
            }
            fn total(&self) -> u16 { self.pos.x + self.pos.y + self.hp }
        }
        fn run() -> u16 {
            let mut a = Actor { pos: Point { x: 1u16, y: 2u16 }, hp: 5u16 };
            a.move_by(10u16, 20u16);
            a.total()
        }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn three_level_nesting() {
    // `c.b.a.v` — offsets sum three levels deep.
    struct A {
        v: u16,
    }
    struct B {
        a: A,
        w: u16,
    }
    struct C {
        b: B,
        z: u16,
    }
    fn host() -> u16 {
        let mut c = C {
            b: B {
                a: A { v: 7 },
                w: 8,
            },
            z: 9,
        };
        c.b.a.v = c.b.a.v + 100;
        c.b.a.v * 100 + c.b.w * 10 + c.z // 107*100 + 8*10 + 9 = 10789
    }
    let src = "
        struct A { v: u16 }
        struct B { a: A, w: u16 }
        struct C { b: B, z: u16 }
        fn run() -> u16 {
            let mut c = C { b: B { a: A { v: 7u16 }, w: 8u16 }, z: 9u16 };
            c.b.a.v = c.b.a.v + 100u16;
            c.b.a.v * 100u16 + c.b.w * 10u16 + c.z
        }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn nested_field_offset_not_first() {
    // The nested struct sits *between* scalar siblings — every field's offset must be
    // the running sum, so `p` starts after `tag` and `extra` after `p`.
    struct P {
        x: u16,
        y: u16,
    }
    struct Q {
        tag: u16,
        p: P,
        extra: u16,
    }
    fn host() -> u16 {
        let mut q = Q {
            tag: 1,
            p: P { x: 5, y: 6 },
            extra: 9,
        };
        q.p.x = q.p.x + q.tag; // 6
        q.p.y = q.p.y + q.extra; // 15
        q.tag * 10000 + q.p.x * 1000 + q.p.y * 10 + q.extra // 1*10000 + 6*1000 + 15*10 + 9 = 16159
    }
    let src = "
        struct P { x: u16, y: u16 }
        struct Q { tag: u16, p: P, extra: u16 }
        fn run() -> u16 {
            let mut q = Q { tag: 1u16, p: P { x: 5u16, y: 6u16 }, extra: 9u16 };
            q.p.x = q.p.x + q.tag;
            q.p.y = q.p.y + q.extra;
            q.tag * 10000u16 + q.p.x * 1000u16 + q.p.y * 10u16 + q.extra
        }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn nested_u32_field() {
    // A `u32` leaf inside a nested struct: wide read + wide write through the chain.
    struct Stats {
        score: u32,
        lives: u16,
    }
    struct Game {
        stats: Stats,
        level: u16,
    }
    fn host() -> u16 {
        let mut g = Game {
            stats: Stats {
                score: 100000,
                lives: 3,
            },
            level: 2,
        };
        g.stats.score = g.stats.score + 50000; // 150000
        let s = g.stats.score;
        (s / 10000) as u16 * 100 + g.stats.lives * 10 + g.level // 15*100 + 30 + 2 = 1532
    }
    let src = "
        struct Stats { score: u32, lives: u16 }
        struct Game { stats: Stats, level: u16 }
        fn run() -> u16 {
            let mut g = Game { stats: Stats { score: 100000u32, lives: 3u16 }, level: 2u16 };
            g.stats.score = g.stats.score + 50000u32;
            let s = g.stats.score;
            (s / 10000u32) as u16 * 100u16 + g.stats.lives * 10u16 + g.level
        }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn nested_field_as_call_arg() {
    // A nested field read flowing into a function call.
    struct Point {
        x: u16,
        y: u16,
    }
    struct Actor {
        pos: Point,
    }
    fn area(w: u16, h: u16) -> u16 {
        w * h
    }
    fn host() -> u16 {
        let a = Actor {
            pos: Point { x: 6, y: 7 },
        };
        area(a.pos.x, a.pos.y) // 42
    }
    let src = "
        struct Point { x: u16, y: u16 }
        struct Actor { pos: Point }
        fn area(w: u16, h: u16) -> u16 { w * h }
        fn run() -> u16 {
            let a = Actor { pos: Point { x: 6u16, y: 7u16 } };
            area(a.pos.x, a.pos.y)
        }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn array_field_inside_nested_struct() {
    // A `[u16; N]` array field reached *through* a nested struct — `g.hud.cells[i]`
    // composes for free: the index path routes through the recursive `field_target`,
    // which carries the leaf array's layout up the chain.
    struct Hud {
        cells: [u16; 3],
        tag: u16,
    }
    struct Game {
        hud: Hud,
        level: u16,
    }
    fn host() -> u16 {
        let mut g = Game {
            hud: Hud {
                cells: [10, 20, 30],
                tag: 7,
            },
            level: 2,
        };
        g.hud.cells[1] = g.hud.cells[1] + g.hud.tag; // 27
        g.hud.cells[0] + g.hud.cells[1] + g.hud.cells[2] + g.level // 10+27+30+2 = 69
    }
    let src = "
        struct Hud { cells: [u16; 3], tag: u16 }
        struct Game { hud: Hud, level: u16 }
        fn run() -> u16 {
            let mut g = Game { hud: Hud { cells: [10u16, 20u16, 30u16], tag: 7u16 }, level: 2u16 };
            g.hud.cells[1] = g.hud.cells[1] + g.hud.tag;
            g.hud.cells[0] + g.hud.cells[1] + g.hud.cells[2] + g.level
        }
    ";
    assert_eq!(run_program(src, "run"), host());
}

#[test]
fn nested_struct_rejections() {
    // Reading a whole nested-struct field is not a scalar — must reach a leaf.
    assert!(rustz80::compile_program(
        "struct P { x: u16, y: u16 } struct Q { p: P }
         fn run() -> u16 { let q = Q { p: P { x: 1u16, y: 2u16 } }; let r = q.p; r.x }"
    )
    .is_err());
    // Assigning a whole nested-struct field is likewise rejected.
    assert!(rustz80::compile_program(
        "struct P { x: u16 } struct Q { p: P, n: u16 }
         fn run() -> u16 { let mut q = Q { p: P { x: 1u16 }, n: 0u16 }; q.p = q.p; q.n }"
    )
    .is_err());
    // `.field` off a scalar field — the value before it has no sub-fields.
    assert!(rustz80::compile_program(
        "struct Q { n: u16 } fn run() -> u16 { let q = Q { n: 5u16 }; q.n.x }"
    )
    .is_err());
    // A nested-struct field must be initialised with a struct literal, not a scalar.
    assert!(rustz80::compile_program(
        "struct P { x: u16 } struct Q { p: P }
         fn run() -> u16 { let q = Q { p: 5u16 }; q.p.x }"
    )
    .is_err());
}
