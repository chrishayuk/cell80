//! The single-call-site inliner: behaviour must be unchanged (rustc oracle).

use crate::harness::*;

// The inliner folds single-call-site helpers at *statement* position. These exercise that
// behaviourally vs rustc — a `&mut self` method call (`c.bump(..)`) and a scalar
// `let x = helper(..)`, the two forms the chuk-speccy authoring kit relies on.
#[test]
fn inline_single_call_mut_self_method() {
    fn host() -> u16 {
        struct C {
            v: u16,
        }
        impl C {
            fn bump(&mut self, k: u16) {
                self.v = self.v + k;
            }
        }
        let mut c = C { v: 10 };
        c.bump(5);
        c.v
    }
    // `bump` has a single call site → the inliner folds it; the result must still match rustc.
    let src = "
        struct C { v: u16 }
        impl C { fn bump(&mut self, k: u16) { self.v = self.v + k; } }
        fn run() -> u16 { let mut c = C { v: 10u16 }; c.bump(5u16); c.v }
    ";
    let prog = rustz80::compile_program(src).expect("compile");
    assert_eq!(run_program(&prog, "run"), host()); // 15
}

#[test]
fn inline_single_call_scalar_assign() {
    fn host() -> u16 {
        fn add5(a: u16) -> u16 {
            a + 5
        }
        let x = add5(10);
        x + 1
    }
    let src = "
        fn add5(a: u16) -> u16 { a + 5u16 }
        fn run() -> u16 { let x = add5(10u16); x + 1u16 }
    ";
    let prog = rustz80::compile_program(src).expect("compile");
    assert_eq!(run_program(&prog, "run"), host()); // 16
}

// chuk-speccy's `chase` pattern exactly: a single-call `&mut self` method that writes an
// **array field through the (substituted) self pointer**, called inside a loop. Guards the
// inliner's argument substitution for pointer receivers + array-field stores.
#[test]
fn inline_mut_self_array_field_in_loop() {
    fn host() -> u16 {
        struct S {
            a: [u16; 4],
        }
        impl S {
            fn step(&mut self, k: u16) {
                self.a[k as usize] = self.a[k as usize] + 1;
            }
        }
        let mut s = S { a: [0; 4] };
        let mut k = 0u16;
        while k < 3 {
            s.step(k);
            k += 1;
        }
        s.a[0] + s.a[1] + s.a[2]
    }
    let src = "
        struct S { a: [u16; 4] }
        impl S { fn step(&mut self, k: u16) { self.a[k as usize] = self.a[k as usize] + 1u16; } }
        fn run() -> u16 {
            let mut s = S { a: [0u16, 0u16, 0u16, 0u16] };
            let mut k = 0u16;
            while k < 3u16 { s.step(k); k = k + 1u16; }
            s.a[0u16 as usize] + s.a[1u16 as usize] + s.a[2u16 as usize]
        }
    ";
    let prog = rustz80::compile_program(src).expect("compile");
    assert_eq!(run_program(&prog, "run"), host()); // 3
}

// chase has *two* single-call methods inlined into one caller (a `&mut self` in a loop, then
// a value-returning `&self`). They share the reused scratch region — guard that the slot
// reuse + substitution is correct across two sibling inlines.
#[test]
fn inline_two_sibling_methods_share_slots() {
    fn host() -> u16 {
        struct S {
            a: [u16; 4],
        }
        impl S {
            fn step(&mut self, k: u16) {
                self.a[k as usize] = self.a[k as usize] + 1;
            }
            fn sum3(&self) -> u16 {
                let mut t = 0u16;
                let mut j = 0u16;
                while j < 3 {
                    t = t + self.a[j as usize];
                    j += 1;
                }
                t
            }
        }
        let mut s = S { a: [0; 4] };
        let mut k = 0u16;
        while k < 3 {
            s.step(k);
            k += 1;
        }
        s.sum3()
    }
    let src = "
        struct S { a: [u16; 4] }
        impl S {
            fn step(&mut self, k: u16) { self.a[k as usize] = self.a[k as usize] + 1u16; }
            fn sum3(&self) -> u16 {
                let mut t = 0u16; let mut j = 0u16;
                while j < 3u16 { t = t + self.a[j as usize]; j = j + 1u16; }
                t
            }
        }
        fn run() -> u16 {
            let mut s = S { a: [0u16, 0u16, 0u16, 0u16] };
            let mut k = 0u16;
            while k < 3u16 { s.step(k); k = k + 1u16; }
            let r = s.sum3();
            r
        }
    ";
    let prog = rustz80::compile_program(src).expect("compile");
    assert_eq!(run_program(&prog, "run"), host()); // 3
}

// chase's `step_enemy` calls a free fn (`solid`) several times with `self.field[i]` args.
// Guard: a non-inlined call *inside* an inlined body, taking the substituted self pointer +
// relocated locals through its args.
#[test]
fn inline_body_with_nested_kept_call() {
    fn host() -> u16 {
        fn keep(v: u16) -> u16 {
            v + 1
        } // called twice ⇒ stays a real call
        struct S {
            a: [u16; 4],
        }
        impl S {
            fn step(&mut self, k: u16) {
                let i = k as usize;
                if keep(self.a[i]) > 0 {
                    self.a[i] = self.a[i] + keep(self.a[i]);
                }
            }
        }
        let mut s = S { a: [0; 4] };
        let mut k = 0u16;
        while k < 3 {
            s.step(k);
            k += 1;
        }
        s.a[0] + s.a[1] + s.a[2]
    }
    let src = "
        fn keep(v: u16) -> u16 { v + 1u16 }
        struct S { a: [u16; 4] }
        impl S {
            fn step(&mut self, k: u16) {
                let i = k as usize;
                if keep(self.a[i]) > 0u16 { self.a[i] = self.a[i] + keep(self.a[i]); }
            }
        }
        fn run() -> u16 {
            let mut s = S { a: [0u16, 0u16, 0u16, 0u16] };
            let mut k = 0u16;
            while k < 3u16 { s.step(k); k = k + 1u16; }
            s.a[0u16 as usize] + s.a[1u16 as usize] + s.a[2u16 as usize]
        }
    ";
    let prog = rustz80::compile_program(src).expect("compile");
    assert_eq!(run_program(&prog, "run"), host()); // 3
}

// chase's *full* shape: a struct with several array fields; a single-call `&mut self` helper
// (`setup`, like `draw_room`) that writes *some* fields in a loop; then "movement" code that
// writes *other* fields; return depends on the movement.
#[test]
fn inline_helper_then_movement_multi_array_fields() {
    fn host() -> u16 {
        struct S {
            cgx: [u16; 4],
            got: [u16; 4],
            ex: [u16; 3],
        }
        impl S {
            fn setup(&mut self) {
                let mut k = 0u16;
                while k < 4 {
                    self.cgx[k as usize] = k + 1;
                    self.got[k as usize] = 0;
                    k += 1;
                }
            }
        }
        let mut s = S {
            cgx: [0; 4],
            got: [0; 4],
            ex: [0; 3],
        };
        s.setup();
        let mut e = 0u16;
        while e < 3 {
            s.ex[e as usize] = s.ex[e as usize] + 5;
            e += 1;
        }
        s.ex[0] + s.ex[1] + s.ex[2]
    }
    let src = "
        struct S { cgx: [u16; 4], got: [u16; 4], ex: [u16; 3] }
        impl S {
            fn setup(&mut self) {
                let mut k = 0u16;
                while k < 4u16 { self.cgx[k as usize] = k + 1u16; self.got[k as usize] = 0u16; k = k + 1u16; }
            }
        }
        fn run() -> u16 {
            let mut s = S { cgx: [0u16,0u16,0u16,0u16], got: [0u16,0u16,0u16,0u16], ex: [0u16,0u16,0u16] };
            s.setup();
            let mut e = 0u16;
            while e < 3u16 { s.ex[e as usize] = s.ex[e as usize] + 5u16; e = e + 1u16; }
            s.ex[0u16 as usize] + s.ex[1u16 as usize] + s.ex[2u16 as usize]
        }
    ";
    let prog = rustz80::compile_program(src).expect("compile");
    assert_eq!(run_program(&prog, "run"), host()); // 15
}
