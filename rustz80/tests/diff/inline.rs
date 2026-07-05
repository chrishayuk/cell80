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
    assert_eq!(run_program(src, "run"), host()); // 15
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
    assert_eq!(run_program(src, "run"), host()); // 16
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
    assert_eq!(run_program(src, "run"), host()); // 3
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
    assert_eq!(run_program(src, "run"), host()); // 3
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
    assert_eq!(run_program(src, "run"), host()); // 3
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
    assert_eq!(run_program(src, "run"), host()); // 15
}

#[test]
fn inline_remaps_every_ir_shape() {
    // A single-call-site helper whose body touches (nearly) every IR node — the
    // inliner's remap walkers must relocate all of it correctly. Checked against
    // rustc, and the helper must actually be folded away.
    let src = "
        const TBL: [u8; 4] = [3u8, 1u8, 4u8, 1u8];
        fn helper(seed: u16, k: u16) -> u16 {
            let mut acc = seed;
            let mut arr = [0u16; 3];
            let mask = [7u16; 2];
            arr[0] = TBL[1] as u16;
            arr[k % 3u16] = acc & mask[0];
            let wide = (acc as u32) * 3u32 + 1u32;
            let flag = (acc < 100u16) && (k != 0u16);
            let rot = acc.rotate_left(3) ^ acc.swap_bytes();
            let bits = acc.count_ones() as u16;
            let mut i = 0u16;
            loop {
                if i >= 2u16 {
                    break;
                }
                acc = acc.wrapping_add(arr[i] << 1u16);
                i = i + 1u16;
            }
            for j in 0..k % 4u16 {
                if j == 3u16 {
                    continue;
                }
                acc = acc ^ (j << (k & 3u16));
            }
            while acc > 60000u16 {
                acc = acc / 3u16;
            }
            acc = match acc & 3u16 {
                0u16 => acc + flag as u16,
                1u16 | 2u16 => acc + bits,
                _ => acc + rot % 5u16,
            };
            acc.wrapping_add((wide >> 8u32) as u16).wrapping_sub(peek(0u16) as u16)
        }
        fn feeder(x: u16) -> u16 { x + 1u16 }
        fn run() -> u16 {
            // A statement-shaped call site (the inliner's contract) with an impure
            // arg (a call) → the slot-bind path, not substitution.
            let r = helper(feeder(41u16), 5u16);
            r
        }
    ";
    #[allow(clippy::needless_range_loop)]
    fn host() -> u16 {
        const TBL: [u8; 4] = [3, 1, 4, 1];
        fn helper(seed: u16, k: u16) -> u16 {
            let mut acc = seed;
            let mut arr = [0u16; 3];
            let mask = [7u16; 2];
            arr[0] = TBL[1] as u16;
            arr[(k % 3) as usize] = acc & mask[0];
            let wide = (acc as u32) * 3 + 1;
            let flag = (acc < 100) && (k != 0);
            let rot = acc.rotate_left(3) ^ acc.swap_bytes();
            let bits = acc.count_ones() as u16;
            let mut i = 0u16;
            loop {
                if i >= 2 {
                    break;
                }
                acc = acc.wrapping_add(arr[i as usize] << 1);
                i += 1;
            }
            for j in 0..k % 4 {
                if j == 3 {
                    continue;
                }
                acc ^= j << (k & 3);
            }
            while acc > 60000 {
                acc /= 3;
            }
            acc = match acc & 3 {
                0 => acc + flag as u16,
                1 | 2 => acc + bits,
                _ => acc + rot % 5,
            };
            acc.wrapping_add((wide >> 8) as u16) // peek(0) is 0 in the harness RAM
        }
        fn feeder(x: u16) -> u16 {
            x + 1
        }
        helper(feeder(41), 5)
    }
    assert_eq!(run_program(src, "run"), host());
    // Through the pruned pipeline (the cell path — the one that inlines), both
    // helpers fold away: single-call-site each, so only `run` survives.
    let file: syn::File = syn::parse_str(src).unwrap();
    let prog = rustz80::compile_file_pruned(&file, rustz80::Target::Cell, &["run"]).unwrap();
    assert!(!prog.symbols.contains_key("helper"));
    // `feeder` rode as an argument (nested in an expression) — it stays a call.
    assert!(prog.symbols.contains_key("feeder"));
    assert!(prog.symbols.contains_key("run"));
}

#[test]
fn inline_leaves_expression_nested_and_multisite_calls_alone() {
    // The inliner only folds *statement-position* calls: a call nested in an expression
    // (`wide(..) & 0xFF`) stays real even though the callee is single-call-site, and a
    // 2+-site helper (`twice`) stays real regardless of position.
    let src = "
        fn wide(acc: u32) -> u32 { acc + 1u32 }
        fn twice(x: u16) -> u16 { x * 2u16 }
        fn run() -> u16 {
            (wide(70_000u32) & 0xFFu32) as u16 + twice(3u16) + twice(4u16)
        }
    ";
    let file: syn::File = syn::parse_str(src).unwrap();
    let prog = rustz80::compile_file_pruned(&file, rustz80::Target::Cell, &["run"]).unwrap();
    assert!(prog.symbols.contains_key("wide"));
    assert!(prog.symbols.contains_key("twice"));
    assert_eq!(run_program(src, "run"), (70_001u32 & 0xFF) as u16 + 6 + 8);
}

#[test]
fn inline_scalar_result_aliases_onto_let() {
    // A single-call helper that returns one of its own locals, bound to a `let`:
    // result-aliasing lands that local straight on the caller's slot — no trailing copy.
    // Correct vs the oracle, and folded away entirely.
    fn accumulate(n: u16) -> u16 {
        let mut s = 0u16;
        let mut i = 1u16;
        while i <= n {
            s += i;
            i += 1;
        }
        s
    }
    fn host() -> u16 {
        accumulate(10) * 2
    }
    let src = "
        fn accumulate(n: u16) -> u16 {
            let mut s = 0u16;
            let mut i = 1u16;
            while i <= n { s = s + i; i = i + 1u16; }
            s
        }
        fn run() -> u16 { let g = accumulate(10u16); g * 2u16 }
    ";
    assert_eq!(run_program(src, "run"), host()); // 110
    let file: syn::File = syn::parse_str(src).unwrap();
    let prog = rustz80::compile_file_pruned(&file, rustz80::Target::Cell, &["run"]).unwrap();
    assert!(!prog.symbols.contains_key("accumulate"));
}

#[test]
fn inline_wide_kernel_folds_on_struct_fields() {
    // The fraction-cell shape distilled: a `&mut self` method calls a shared wide kernel
    // on two `self` fields, binds the result, divides by it twice. The kernel folds fully
    // (field-arg substitution + result-aliasing), so it's byte-neutral; behaviour vs the
    // oracle, and no `gcd_u32` symbol survives.
    fn gcd_u32(a: u32, b: u32) -> u32 {
        let mut x = a;
        let mut y = b;
        while y != 0 {
            let t = y;
            y = x % y;
            x = t;
        }
        x
    }
    fn host() -> u16 {
        let g = gcd_u32(84_000, 36_000);
        (84_000 / g + 36_000 / g) as u16
    }
    let src = "
        fn gcd_u32(a: u32, b: u32) -> u32 {
            let mut x = a; let mut y = b;
            while y != 0u32 { let t = y; y = x % y; x = t; }
            x
        }
        struct F { n: u32, d: u32 }
        impl F {
            fn reduce(&mut self) -> u16 {
                let g = gcd_u32(self.n, self.d);
                (self.n / g + self.d / g) as u16
            }
        }
        fn run() -> u16 {
            let mut f = F { n: 84_000u32, d: 36_000u32 };
            f.reduce()
        }
    ";
    assert_eq!(run_program(src, "run"), host());
    let file: syn::File = syn::parse_str(src).unwrap();
    let prog = rustz80::compile_file_pruned(&file, rustz80::Target::Cell, &["run"]).unwrap();
    assert!(
        !prog.symbols.contains_key("gcd_u32"),
        "a once-called wide kernel on struct fields must fold into the method"
    );
}

#[test]
fn inline_wide_arg_relocated_and_multisite_wide_ret_kept() {
    // Exercises the wide *relocation* paths: a single-call wide kernel (`combine`) folds,
    // but its first arg is an effectful call result (`twice(500)`) that can't substitute,
    // so it binds via `Assign32` (the wide param-relocate + bind path). `combine` returns
    // an expression (not a local), so the wide result copies rather than aliases. And
    // `twice`, used at three sites, stays a real wide-return call. Oracle-checked.
    fn twice(w: u32) -> u32 {
        w + w
    }
    fn combine(a: u32, b: u32) -> u32 {
        a + b
    }
    fn host() -> u16 {
        let x = twice(1000);
        let y = twice(x);
        let g = combine(twice(500), y);
        (g / 100) as u16
    }
    let src = "
        fn twice(w: u32) -> u32 { w + w }
        fn combine(a: u32, b: u32) -> u32 { a + b }
        fn run() -> u16 {
            let x = twice(1000u32);
            let y = twice(x);
            let g = combine(twice(500u32), y);
            (g / 100u32) as u16
        }
    ";
    assert_eq!(run_program(src, "run"), host()); // 50
    let file: syn::File = syn::parse_str(src).unwrap();
    let prog = rustz80::compile_file_pruned(&file, rustz80::Target::Cell, &["run"]).unwrap();
    assert!(
        prog.symbols.contains_key("twice"),
        "3-site wide kernel stays a call"
    );
    assert!(
        !prog.symbols.contains_key("combine"),
        "single-call wide kernel folds"
    );
}

#[test]
fn inline_field_arg_bound_when_callee_writes_memory() {
    // The soundness guard for field-arg substitution: when the callee writes memory, an
    // effect-free `self.field` arg must be *captured* (bound to a slot) before the write,
    // not substituted — substitution would re-read the now-clobbered field. `helper` sets
    // `self.f = 99` then returns `x + self.f`; called as `helper(self.f)`, the correct
    // answer binds the *old* f (7): 7 + 99 = 106. Substituting would wrongly give 198.
    fn host() -> u16 {
        106
    }
    let src = "
        struct S { f: u16 }
        impl S {
            fn outer(&mut self) -> u16 { let r = self.helper(self.f); r }
            fn helper(&mut self, x: u16) -> u16 { self.f = 99u16; x + self.f }
        }
        fn run() -> u16 { let mut s = S { f: 7u16 }; s.outer() }
    ";
    assert_eq!(run_program(src, "run"), host());
}
