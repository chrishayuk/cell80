//! Differential testing — the compiler's oracle (spec 07 §8). Each `check!` takes
//! one Rust block and runs it two ways: under **rustc** (a host `fn`) and through
//! **rustz80** onto our Z80 (compile → run → read `HL`). They must agree. The
//! single-source property is what makes this airtight: there's no second copy to
//! drift.

// `check!` blocks are stringified into dialect source, so they must use the
// dialect's long-form (`x = x + 1`, an explicit swap) — not Rust's `+=`/`swap`.
// `needless_range_loop` is allowed deliberately: these tests exercise `for i in a..b`
// index loops as the *subject under test*, not as a style to refactor away. Likewise the
// `&&`/`||` bound checks (`x > lo && x < hi`) and `% 2 == 0` are the dialect feature being
// tested — not patterns to rewrite as `.contains()` / `.is_multiple_of()` (neither is in
// the subset anyway).
#![allow(
    clippy::assign_op_pattern,
    clippy::manual_swap,
    clippy::needless_range_loop,
    clippy::manual_range_contains,
    clippy::manual_is_multiple_of,
    clippy::manual_range_patterns,
    // `!(a >= b)` on u32 deliberately exercises negated wide conditions.
    clippy::nonminimal_bool
)]

#[macro_use]
mod harness;

mod arrays;
mod basics;
mod bytes;
mod conditionals;
mod consts;
mod control_flow;
mod functions;
mod generics;
mod inline;
mod misc;
mod nested_structs;
mod peephole;
mod recursion;
mod signed;
mod strings;
mod structs;
mod u32_ops;
