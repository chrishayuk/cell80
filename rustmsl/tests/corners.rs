//! The R1 corner battery (docs 14): GPU integer semantics ≡ the reference
//! interpreter, bit for bit, on exactly the corners where shading languages
//! drift — shift-by-≥-width, signed arithmetic shift saturation, signed
//! div/rem wrapping (`MIN/-1`), byte-width wrap, short-circuit evaluation
//! hiding a trap, and the trap statuses themselves (divide-by-zero, `halt`).
//!
//! Each case lowers a dialect snippet once, runs every input on a fresh
//! interpreter and once as one GPU batch, and asserts the full
//! `[r0, r1, r2, status]` quad agrees. Seeded xorshift inputs — no `rand`,
//! fully reproducible (the `cell_fuzz` discipline).

#![cfg(target_os = "macos")]

use cell80_core::{Interp, Target};
use rustmsl::{GpuBatch, STATUS_DIV0, STATUS_HALT, STATUS_OK};

/// The `cell_fuzz` xorshift — fixed seeds, no `rand`, fully reproducible.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn u16(&mut self) -> u16 {
        self.next() as u16
    }
}

/// What the interpreter said, folded to the GPU's output quad shape.
fn interp_quad(res: Result<Vec<u16>, String>) -> [u16; 4] {
    match res {
        Ok(v) => [
            v.first().copied().unwrap_or(0),
            v.get(1).copied().unwrap_or(0),
            v.get(2).copied().unwrap_or(0),
            STATUS_OK,
        ],
        Err(e) if e.contains("divide by zero") => [0, 0, 0, STATUS_DIV0],
        Err(e) => {
            let code = e
                .strip_prefix("interp: halt(")
                .and_then(|s| s.strip_suffix(')'))
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or_else(|| panic!("unexpected interpreter refusal: {e}"));
            [code, 0, 0, STATUS_HALT]
        }
    }
}

/// Lower `src`, run every input triple on interpreter and GPU, assert the
/// quads agree bit for bit.
fn check(src: &str, inputs: &[[u16; 3]]) {
    let file: syn::File =
        syn::parse_str(src).unwrap_or_else(|e| panic!("parse failed: {e}\nsrc: {src}"));
    let lowered = rustz80::lower_program_full(&file, &rustz80::PreludeConfig::default())
        .unwrap_or_else(|e| panic!("lower failed: {e}\nsrc: {src}"));
    let consts = lowered.const_data();
    let module = rustmsl::compile(&lowered.funcs, &consts, "run")
        .unwrap_or_else(|e| panic!("msl compile failed: {e}\nsrc: {src}"));
    let gpu = GpuBatch::new(&module)
        .unwrap_or_else(|e| panic!("gpu pipeline failed: {e}\nmsl:\n{}", module.source));
    let got = gpu.run(inputs).expect("gpu run");
    let n_args = module.params;
    for (i, (args, gpu_quad)) in inputs.iter().zip(&got).enumerate() {
        let mut interp = Interp::new(
            &lowered.funcs,
            consts.iter().map(|(n, b)| (n.as_str(), b.as_slice())),
            Target::Cell.descriptor(),
        );
        let want = interp_quad(interp.run("run", &args[..n_args]));
        assert_eq!(
            *gpu_quad, want,
            "case {i}: args {args:?} — gpu {gpu_quad:?} != interpreter {want:?}\nsrc: {src}"
        );
    }
}

/// Corner inputs every numeric case sweeps, then a seeded-random tail.
fn sweep(seed: u64, n: usize) -> Vec<[u16; 3]> {
    let corners: &[u16] = &[
        0, 1, 2, 3, 7, 8, 15, 16, 17, 31, 32, 33, 0x7F, 0x80, 0xFF, 0x100, 0x7FFF, 0x8000, 0x8001,
        0xFFFE, 0xFFFF,
    ];
    let mut v = Vec::new();
    for &a in corners {
        for &b in corners {
            v.push([a, b, 0]);
        }
    }
    let mut rng = Rng(seed);
    for _ in 0..n {
        v.push([rng.u16(), rng.u16(), rng.u16()]);
    }
    v
}

#[test]
fn shift_var_by_ge_width_matches_interp() {
    // Runtime shift counts, both directions, unsigned: counts ≥ 16 must shift
    // out to 0 — the exact corner where MSL's `<<` alone would be undefined.
    check(
        "fn run(x: u16, n: u16) -> u16 { (x << n) | (x >> n) }",
        &sweep(0x5eed_0001, 4000),
    );
}

#[test]
fn signed_arithmetic_shift_saturates() {
    // i16 `>>` is arithmetic and a big count saturates at the sign fill.
    check(
        "fn run(x: i16, n: u16) -> i16 { x >> n }",
        &sweep(0x5eed_0002, 4000),
    );
}

#[test]
fn literal_shifts_match_interp() {
    check(
        "fn run(x: u16) -> u16 { (x << 3) ^ (x >> 15) ^ (x << 15) }",
        &sweep(0x5eed_0003, 2000),
    );
}

#[test]
fn signed_div_rem_wraps_min_over_minus_one() {
    // rustc `wrapping_div`: MIN/-1 is MIN (rem 0); division by zero is the
    // interpreter's refusal and must surface as the same trap on the GPU.
    check(
        "fn run(a: i16, b: i16) -> i16 { (a / b) + (a % b) }",
        &sweep(0x5eed_0004, 4000),
    );
}

#[test]
fn unsigned_div_rem_and_zero_trap() {
    check(
        "fn run(a: u16, b: u16) -> u16 { (a / b) * b + (a % b) }",
        &sweep(0x5eed_0005, 4000),
    );
}

#[test]
fn byte_width_wraps_mod_256() {
    check(
        "fn run(x: u16, y: u16) -> u16 { let a = x as u8; let b = y as u8; ((a + b) as u16) | (((a * b) as u16) << 8) }",
        &sweep(0x5eed_0006, 4000),
    );
}

#[test]
fn signed_compare_orders_by_twos_complement() {
    check(
        "fn run(a: i16, b: i16) -> u16 { ((a < b) as u16) | (((a >= b) as u16) << 1) | (((a == b) as u16) << 2) }",
        &sweep(0x5eed_0007, 4000),
    );
}

#[test]
fn short_circuit_hides_the_divide() {
    // `b / a` must not run (or trap) when `a == 0` short-circuits it away.
    check(
        "fn run(a: u16, b: u16) -> u16 { if a > 0 && b / a > 2 { 1 } else { 0 } }",
        &sweep(0x5eed_0008, 4000),
    );
}

#[test]
fn u32_arithmetic_matches_interp() {
    check(
        "fn run(a: u16, b: u16, c: u16) -> u32 { let x = ((a as u32) << 16) | (b as u32); let d = c as u32; (x / (d + 1)) ^ (x * d) ^ (x >> 5) }",
        &sweep(0x5eed_0009, 4000),
    );
}

#[test]
fn i32_div_wraps_min_over_minus_one() {
    // i32 MIN / -1: C++ overflows (UB); the IR wraps. The select in the
    // emitted MSL is exactly what this pins.
    check(
        "fn run(a: i16, b: i16) -> i32 { let x = (a as i32) << 16; let y = b as i32; (x / y) + (x % y) }",
        &sweep(0x5eed_000a, 4000),
    );
}

#[test]
fn sign_extend_and_widen_bridges() {
    check(
        "fn run(a: i16, b: u16) -> u32 { (a as u32) + ((b as u32) << 3) }",
        &sweep(0x5eed_000b, 4000),
    );
}

#[test]
fn bit_method_kernels_match() {
    check(
        "fn run(x: u16) -> u16 { x.count_ones() + (x.leading_zeros() << 5) + (x.trailing_zeros() << 10) }",
        &sweep(0x5eed_000c, 4000),
    );
}

#[test]
fn halt_code_rides_r0() {
    check(
        "fn run(x: u16) -> u16 { if x > 40000 { halt(7); } x + 1 }",
        &sweep(0x5eed_000d, 2000),
    );
}

#[test]
fn tuple_return_pads_like_the_interpreter() {
    check(
        "fn run(a: u16, b: u16) -> (u16, u16) { let hi = if a > b { a } else { b }; let lo = if a > b { b } else { a }; (hi - lo, (a == b) as u16) }",
        &sweep(0x5eed_000e, 2000),
    );
}

#[test]
fn helper_call_through_the_slot_file() {
    check(
        "fn diff(a: i16, b: i16) -> i16 { if a > b { a - b } else { b - a } }\n\
         fn run(x: i16, y: i16, z: i16) -> i16 { diff(diff(x, y), z) }",
        &sweep(0x5eed_000f, 4000),
    );
}

#[test]
fn loops_refuse_with_a_typed_error() {
    let src = "fn run(x: u16) -> u16 { let mut s = 0; let mut i = 0; while i < x { s = s + i; i = i + 1; } s }";
    let file: syn::File = syn::parse_str(src).unwrap();
    let lowered = rustz80::lower_program_full(&file, &rustz80::PreludeConfig::default()).unwrap();
    let err = rustmsl::compile(&lowered.funcs, &lowered.const_data(), "run").unwrap_err();
    assert!(
        err.contains("E2") && err.contains("straight-line"),
        "want a typed E2 refusal, got: {err}"
    );
}
