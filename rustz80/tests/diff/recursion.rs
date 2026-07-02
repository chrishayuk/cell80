//! Phase-0 determinism gate: **recursion is rejected at compile time.** Stage 1 gives
//! every function static local slots, so a recursive call clobbers the caller's frame —
//! the slot-after-call factorial compiled and silently returned 1 instead of 120 before
//! this gate, and tail-shaped recursion only "worked" by riding the hardware stack.
//! No cyclic call graph may reach codegen, on either entry path.

fn rejected(src: &str) {
    let e = rustz80::compile_program(src)
        .map(|_| ())
        .expect_err("recursion must not compile");
    assert!(
        e.contains("recursion is not supported"),
        "wrong diagnostic: {e}"
    );
}

#[test]
fn slot_after_call_factorial_is_rejected() {
    // The silent-miscompile probe: `n * fact(n-1)` reads a slot after the recursive
    // call returned — pre-gate this compiled and returned 1 for fact(5).
    rejected(
        "fn fact(n: u16) -> u16 {
             let mut r = 1u16;
             if n > 1u16 { let t = fact(n - 1u16); r = n * t; }
             r
         }
         fn run(n: u16) -> u16 { fact(n) }",
    );
}

#[test]
fn tail_shaped_recursion_is_rejected_too() {
    // This shape happened to compute correctly (accumulator rides the call registers),
    // but "works by accident" is not a contract — the cycle is rejected uniformly.
    rejected(
        "fn fact(n: u16, acc: u16) -> u16 {
             let mut r = acc;
             if n > 1u16 { r = fact(n - 1u16, n * acc); }
             r
         }
         fn run(n: u16) -> u16 { fact(n, 1u16) }",
    );
}

#[test]
fn local_array_recursion_is_rejected() {
    // A local array in a recursive fn is the worst case: the whole array is one static
    // region shared by every live frame.
    rejected(
        "fn walk(n: u16) -> u16 {
             let buf = [n, n + 1u16, n + 2u16];
             let mut r = buf[0];
             if n > 0u16 { r = walk(n - 1u16) + buf[1]; }
             r
         }
         fn run(n: u16) -> u16 { walk(n) }",
    );
}

#[test]
fn mutual_recursion_is_rejected() {
    let e = rustz80::compile_program(
        "fn is_ev(n: u16) -> u16 { let mut r = 1u16; if n > 0u16 { r = is_od(n - 1u16); } r }
         fn is_od(n: u16) -> u16 { let mut r = 0u16; if n > 0u16 { r = is_ev(n - 1u16); } r }
         fn run(n: u16) -> u16 { is_ev(n) }",
    )
    .map(|_| ())
    .expect_err("mutual recursion must not compile");
    assert!(
        e.contains("recursion is not supported"),
        "wrong diagnostic: {e}"
    );
    // The diagnostic names the cycle, so the author sees *which* functions to unroll.
    assert!(
        e.contains("is_ev") && e.contains("is_od"),
        "cycle not named: {e}"
    );
}

#[test]
fn self_recursion_is_rejected_on_the_single_fn_path() {
    // `compile_fn` lowers one function directly (it never sees `lower_program`) — the
    // same gate must hold there.
    let e = rustz80::compile_fn(
        "fn f(n: u16) -> u16 { let mut r = 1u16; if n > 0u16 { r = f(n - 1u16); } r }",
    )
    .expect_err("self-recursion must not compile");
    assert!(
        e.contains("recursion is not supported"),
        "wrong diagnostic: {e}"
    );
}

#[test]
fn non_recursive_call_chains_still_compile() {
    // The gate is a cycle check, not a call check: a diamond (run → a,b → shared) is fine.
    rustz80::compile_program(
        "fn shared(x: u16) -> u16 { x + 1u16 }
         fn a(x: u16) -> u16 { shared(x) * 2u16 }
         fn b(x: u16) -> u16 { shared(x) * 3u16 }
         fn run(x: u16) -> u16 { a(x) + b(x) }",
    )
    .expect("a DAG of calls must still compile");
}
