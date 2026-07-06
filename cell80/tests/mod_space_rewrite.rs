//! The mod-space rewrite (canon.rs Full mode): a `<chain> % m` tail with a leaf modulus
//! and a straight-line (+/-/*) chain rewrites to a step-wise mod-reduced chain threaded
//! through `m`, instead of computing the whole chain wide and reducing once at the end.
//!
//! The wide lane's plain `+`/`-`/`*` are **unchecked**, release-rustc wrapping ops (the
//! `Node::Sum` doc in `canon.rs` says as much) — the canonicalizer never auto-inserts the
//! checked kernels (`mul_checked_u32` et al.) into ordinary arithmetic, only hand-authored
//! cells call those explicitly. So the naive wide-then-mod path doesn't escalate on a
//! real overflow — it silently wraps mod 2^32 and reduces *that* mod m, which is not the
//! same number as the true value mod m unless m happens to divide 2^32 evenly (a power
//! of two — 1000 and 7 below aren't). The fallback path never surfaces the mismatch; it
//! just returns `Halt::Returned` with the wrong residue. Both cases below are proven
//! end-to-end (real compile, real Z80 run) against an independently-computed expected
//! value:
//!
//! 1. **Correct where the naive path is silently wrong on a genuine u32 overflow** — the
//!    AIME "reduce mod 1000" finishing move on a product chain whose true value vastly
//!    exceeds u32(2^32), but whose *answer* (a residue mod m) obviously fits.
//! 2. **Correct where the naive path is silently wrong on a mid-chain underflow** — a
//!    chain that goes negative before the final `% m` wraps mod 2^32 at that point too;
//!    reducing the wrapped value mod a non-power-of-two m doesn't recover the true
//!    residue either. The rewrite's `mod_sub` step is a real modular subtraction at
//!    every step, never a wrap, so it's exact in both cases.

use cell80::{Cartridge, CartridgeOpts, CellConfig, CellHost, DEFAULT_CYCLES};
use rustz80::CanonMode;

fn compile_wide(id: &str, src: &str) -> Cartridge {
    Cartridge::compile(
        src,
        CellConfig::sandboxed(),
        CartridgeOpts {
            id: Some(id.into()),
            canon: CanonMode::Full,
            canon_wide: true,
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| panic!("compiles: {e}\n{src}"))
}

fn run(id: &str, cart: Cartridge, args: &[u16]) -> cell80::Fast {
    let mut host = CellHost::new();
    host.add(cart);
    let h = host.handle_for(id).unwrap();
    host.run_fast(h, args, DEFAULT_CYCLES).unwrap()
}

#[test]
fn mod_rewrite_is_correct_where_the_naive_wide_path_silently_wraps() {
    // a*b*c genuinely overflows u32 (a*b fits, *c doesn't: true value 216,212,465,466,409),
    // but (a*b*c) % 1000 obviously fits — the point of reducing from the start instead
    // of at the end.
    let src = "fn run(a: u16, b: u16, c: u16) -> u16 { a * b * c % 1000 }";
    let cart = compile_wide("modchain", src);
    assert!(
        cart.canon_repairs
            .iter()
            .any(|r| r.to_string().contains("mod_space_rewrite")),
        "{:?}",
        cart.canon_repairs
    );
    let fast = run("modchain", cart, &[60013, 60017, 60029]);
    assert_eq!(fast.halt, cell80::Halt::Returned);
    let got = fast.regs[0] as u64 | ((fast.regs[1] as u64) << 16);
    let (a, b, c): (u32, u32, u32) = (60013, 60017, 60029);
    let true_val = a as u64 * b as u64 * c as u64;
    assert_eq!(got, true_val % 1000, "the exact residue, 409");

    // What the naive wide-then-mod path (no rewrite) would have silently returned: the
    // true product wraps mod 2^32 first (Z80 codegen's raw `*`, unchecked), then *that*
    // wrapped value's % 1000 is a different, wrong residue — no escalation, no signal,
    // just a plausible-looking wrong answer. This is what the rewrite is worth avoiding.
    let wrapped = a.wrapping_mul(b).wrapping_mul(c);
    assert_ne!(
        (wrapped % 1000) as u64,
        got,
        "the naive wide-then-mod path would have wrapped to a different (wrong) residue"
    );
}

#[test]
fn mod_rewrite_is_exact_where_the_wide_fallback_wraps() {
    // (a - b + c) % m with a < b goes negative mid-chain. The non-rewritten wide lane's
    // plain `-` wraps mod 2^32 (canon.rs's own documented semantics for Node::Sum
    // reordering); reducing *that* mod a non-power-of-two m does not generally recover
    // the true residue. The rewrite's mod_sub step never wraps: it's a real subtraction
    // mod m at every step, so it gets the mathematically correct answer either way.
    let src = "fn run(a: u16, b: u16, c: u16) -> u16 { (a - b + c) % 7 }";
    let cart = compile_wide("modsub", src);
    assert!(cart
        .canon_repairs
        .iter()
        .any(|r| r.to_string().contains("mod_space_rewrite")));
    let fast = run("modsub", cart, &[5, 10, 3]);
    assert_eq!(fast.halt, cell80::Halt::Returned);
    let got = fast.regs[0] as u64 | ((fast.regs[1] as u64) << 16);
    assert_eq!(
        got, 5,
        "(5 - 10 + 3) mod 7 == 5, the true nonnegative residue"
    );
}

#[test]
fn mod_rewrite_does_not_fire_through_division() {
    // A chain with real division in it (not just a folded constant) is out of scope for
    // the rewrite — division has no per-step modular analogue without a modular
    // inverse, which is a different, riskier operation. Falls back unchanged.
    let src = "fn run(a: u16, b: u16) -> u16 { (a * b / 2) % 1000 }";
    let cart = compile_wide("moddiv", src);
    assert!(!cart
        .canon_repairs
        .iter()
        .any(|r| r.to_string().contains("mod_space_rewrite")));
}

#[test]
fn mod_rewrite_does_not_fire_on_a_non_leaf_modulus() {
    // The modulus itself must be a leaf (param/const) so its value is available before
    // any chain op runs, with no ordering hazard against the rewritten chain.
    let src = "fn run(a: u16, b: u16, c: u16) -> u16 { (a * b) % (b + c) }";
    let cart = compile_wide("modnonleaf", src);
    assert!(!cart
        .canon_repairs
        .iter()
        .any(|r| r.to_string().contains("mod_space_rewrite")));
}
