//! The R1 corner battery (docs 14): GPU integer semantics ≡ the reference
//! interpreter, bit for bit, on exactly the corners where shading languages
//! drift — shift-by-≥-width, signed arithmetic shift saturation, signed
//! div/rem wrapping (`MIN/-1`), byte-width wrap, short-circuit evaluation
//! hiding a trap, and the trap statuses themselves (divide-by-zero, `halt`,
//! fuel exhaustion). E2 widens it to loops: `while`/`for`/`loop`,
//! `break`/`continue` (including `continue` reaching a `for`'s induction
//! step), nested loops, and data-dependent iteration counts.
//!
//! Every case also asserts **IR-step parity**: the GPU's per-thread step
//! count must equal the interpreter's `tick` count exactly — the canonical
//! family cost (docs 14, Q2) metered on both substrates.
//!
//! Each case lowers a dialect snippet once, runs every input on a fresh
//! interpreter and once as one GPU batch, and asserts the full
//! `[r0, r1, r2, status]` quad plus the step count agree. Seeded xorshift
//! inputs — no `rand`, fully reproducible (the `cell_fuzz` discipline).

#![cfg(any(target_os = "macos", feature = "cuda"))]

use cell80_core::{Interp, Target};
use rustmsl::{steps_of, STATUS_DIV0, STATUS_FUEL, STATUS_HALT, STATUS_OK};

/// Compile-and-run on whichever GPU backend this build has: Metal on macOS,
/// CUDA elsewhere under `--features cuda`. The battery text is identical on
/// both — this seam guard runs wherever a backend exists.
#[cfg(target_os = "macos")]
mod exec {
    pub fn compile_library(cells: &[rustmsl::LibraryCell]) -> Result<rustmsl::GpuModule, String> {
        rustmsl::compile_library(cells)
    }
    pub fn run(module: &rustmsl::GpuModule, inputs: &[[u16; 3]]) -> Result<Vec<[u16; 6]>, String> {
        rustmsl::GpuBatch::new(module)?.run(inputs)
    }
    pub fn run_with_state(
        module: &rustmsl::GpuModule,
        inputs: &[[u16; 3]],
        state_in: &[u8],
    ) -> Result<(Vec<[u16; 6]>, Vec<u8>), String> {
        rustmsl::GpuBatch::new(module)?.run_with_state(inputs, state_in)
    }
}

#[cfg(all(not(target_os = "macos"), feature = "cuda"))]
mod exec {
    pub fn compile_library(cells: &[rustmsl::LibraryCell]) -> Result<rustmsl::GpuModule, String> {
        rustmsl::compile_library_cuda(cells)
    }
    pub fn run(module: &rustmsl::GpuModule, inputs: &[[u16; 3]]) -> Result<Vec<[u16; 6]>, String> {
        rustmsl::CudaBatch::new(module)?.run(inputs)
    }
    pub fn run_with_state(
        module: &rustmsl::GpuModule,
        inputs: &[[u16; 3]],
        state_in: &[u8],
    ) -> Result<(Vec<[u16; 6]>, Vec<u8>), String> {
        rustmsl::CudaBatch::new(module)?.run_with_state(inputs, state_in)
    }
}

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

/// What the interpreter said, folded to the GPU's output shape.
fn interp_quad(res: Result<Vec<u16>, String>) -> [u16; 4] {
    match res {
        Ok(v) => [
            v.first().copied().unwrap_or(0),
            v.get(1).copied().unwrap_or(0),
            v.get(2).copied().unwrap_or(0),
            STATUS_OK,
        ],
        Err(e) if e.contains("divide by zero") => [0, 0, 0, STATUS_DIV0],
        Err(e) if e.contains("fuel exhausted") => [0, 0, 0, STATUS_FUEL],
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
/// quads and step counts agree bit for bit.
fn check(src: &str, inputs: &[[u16; 3]]) {
    let file: syn::File =
        syn::parse_str(src).unwrap_or_else(|e| panic!("parse failed: {e}\nsrc: {src}"));
    let lowered = rustz80::lower_program_full(&file, &rustz80::PreludeConfig::default())
        .unwrap_or_else(|e| panic!("lower failed: {e}\nsrc: {src}"));
    let consts = lowered.const_data();
    let module = exec::compile_library(&[rustmsl::LibraryCell {
        funcs: &lowered.funcs,
        consts: &consts,
        entry: "run",
        state_len: 0,
    }])
    .unwrap_or_else(|e| panic!("gpu compile failed: {e}\nsrc: {src}"));
    let got = exec::run(&module, inputs)
        .unwrap_or_else(|e| panic!("gpu run failed: {e}\nsource:\n{}", module.source));
    let n_args = module.cells[0].params;
    for (i, (args, gpu_out)) in inputs.iter().zip(&got).enumerate() {
        let mut interp = Interp::new(
            &lowered.funcs,
            consts.iter().map(|(n, b)| (n.as_str(), b.as_slice())),
            Target::Cell.descriptor(),
        );
        let res = interp.run("run", &args[..n_args]);
        let want = interp_quad(res);
        let want_steps = u32::try_from(interp.steps()).expect("steps fit u32");
        let got_quad = [gpu_out[0], gpu_out[1], gpu_out[2], gpu_out[3]];
        assert_eq!(
            got_quad, want,
            "case {i}: args {args:?} — gpu {got_quad:?} != interpreter {want:?}\nsrc: {src}"
        );
        assert_eq!(
            steps_of(gpu_out),
            want_steps,
            "case {i}: args {args:?} — gpu steps != interpreter steps\nsrc: {src}"
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

// ── E1: straight-line corners ──────────────────────────────────────────────

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
    // `b / a` must not run (or trap, or tick) when `a == 0` short-circuits it.
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

// ── E2: loops and branches ─────────────────────────────────────────────────

#[test]
fn while_gcd_data_dependent_iterations() {
    // The canonical data-dependent loop count — the divergence shape E2 is
    // about. Steps parity proves the iteration count matched exactly.
    check(
        "fn run(a: u16, b: u16) -> u16 { let mut x = a; let mut y = b; while y != 0 { let t = x % y; x = y; y = t; } x }",
        &sweep(0x5eed_0010, 4000),
    );
}

#[test]
fn for_range_accumulates() {
    check(
        "fn run(n: u16, k: u16) -> u16 { let mut s = 0; for i in 0..(n & 255) { s = s + i + k; } s }",
        &sweep(0x5eed_0011, 3000),
    );
}

#[test]
fn continue_reaches_the_induction_step() {
    // The do-while(false) wrapper: `continue` must land on the step, not the
    // condition — an infinite loop (and a steps mismatch) if it doesn't.
    check(
        "fn run(n: u16, m: u16) -> u16 { let mut s = 0; for i in 0..(n & 127) { if i % 3 == 0 { continue; } if i == m { continue; } s = s + i; } s }",
        &sweep(0x5eed_0012, 3000),
    );
}

#[test]
fn break_exits_the_right_loop() {
    check(
        "fn run(n: u16, k: u16) -> u16 { let mut s = 0; for i in 0..(n & 255) { if i > k { break; } s = s + 1; } while s > 40000 { s = s - 7; break; } s }",
        &sweep(0x5eed_0013, 3000),
    );
}

#[test]
fn nested_loops_with_inner_break_and_continue() {
    check(
        "fn run(a: u16, b: u16) -> u16 { let mut s = 0; for i in 0..(a & 31) { for j in 0..(b & 31) { if j == i { continue; } if j > 20 { break; } s = s + 1; } if s > 400 { break; } } s }",
        &sweep(0x5eed_0014, 2000),
    );
}

#[test]
fn loop_with_break_value_shape() {
    check(
        "fn run(x: u16) -> u16 { let mut v = x | 1; let mut n = 0; loop { if v == 1 { break; } if v % 2 == 0 { v = v / 2; } else { v = (v & 8191) * 3 + 1; } n = n + 1; if n > 400 { break; } } n }",
        &sweep(0x5eed_0015, 2000),
    );
}

#[test]
fn byte_loop_variable_wraps() {
    // A u8 induction variable: the step masks to the variable's width.
    check(
        "fn run(n: u16) -> u16 { let mut s = 0; let mut i: u8 = 250; while i != 4 { i = i + 1; s = s + 1; if s > 300 { break; } } s + (n & 0) }",
        &sweep(0x5eed_0016, 1000),
    );
}

#[test]
fn prelude_kernels_loop_on_the_gpu() {
    // gcd/isqrt from the shared prelude — helper calls that themselves loop.
    check(
        "fn gcd(a: u16, b: u16) -> u16 { let mut x = a; let mut y = b; while y != 0u16 { let t = x % y; x = y; y = t; } x }\n\
         fn isqrt(n: u16) -> u16 { let mut r = 0u16; while r < 255u16 && (r + 1u16) * (r + 1u16) <= n { r = r + 1u16; } r }\n\
         fn run(a: u16, b: u16) -> u16 { gcd(a, b) + isqrt(a) }",
        &sweep(0x5eed_0017, 3000),
    );
}

#[test]
fn runaway_loop_is_a_fuel_trap_on_both_sides() {
    // The budget bound (E2): a loop that never exits burns exactly FUEL steps
    // on both substrates and surfaces as the same trap — never a hung dispatch.
    check(
        "fn run(x: u16) -> u16 { let mut s = 0; loop { s = s + 1; if x > 65534 { if s == 0 { break; } } } s }",
        &[[1, 0, 0], [65535, 0, 0]],
    );
}

// ── typed state: the `impl State { fn run(&mut self) }` window ────────────

/// Lower an impl-state snippet, run every (input triple, state block) on a
/// fresh interpreter (state planted at 0xB000) and once as one GPU batch with
/// per-thread state, and assert sextets AND final state bytes agree.
fn check_state(src: &str, entry: &str, state_len: usize, inputs: &[[u16; 3]], seed: u64) {
    let file: syn::File =
        syn::parse_str(src).unwrap_or_else(|e| panic!("parse failed: {e}\nsrc: {src}"));
    let lowered = rustz80::lower_program_full(&file, &rustz80::PreludeConfig::default())
        .unwrap_or_else(|e| panic!("lower failed: {e}\nsrc: {src}"));
    let consts = lowered.const_data();
    let module = exec::compile_library(&[rustmsl::LibraryCell {
        funcs: &lowered.funcs,
        consts: &consts,
        entry,
        state_len,
    }])
    .unwrap_or_else(|e| panic!("gpu compile failed: {e}\nsrc: {src}"));

    // Random initial state blocks — any bit pattern is a valid scalar field.
    let mut rng = Rng(seed);
    let mut state_in = vec![0u8; state_len * inputs.len()];
    for b in state_in.iter_mut() {
        *b = rng.next() as u8;
    }
    let (got, state_out) = exec::run_with_state(&module, inputs, &state_in)
        .unwrap_or_else(|e| panic!("gpu run failed: {e}\nsource:\n{}", module.source));

    let n_args = module.cells[0].params;
    for (i, (args, gpu_out)) in inputs.iter().zip(&got).enumerate() {
        let mut interp = Interp::new(
            &lowered.funcs,
            consts.iter().map(|(n, b)| (n.as_str(), b.as_slice())),
            Target::Cell.descriptor(),
        );
        interp.plant(
            rustmsl::STATE_BASE,
            &state_in[i * state_len..(i + 1) * state_len],
        );
        // args: [self = STATE_BASE, extras from the input triple].
        let mut call: Vec<u16> = vec![rustmsl::STATE_BASE];
        call.extend_from_slice(&args[..n_args.saturating_sub(1)]);
        let res = interp.run(entry, &call);
        let want = interp_quad(res);
        let want_steps = u32::try_from(interp.steps()).expect("steps fit u32");
        let got_quad = [gpu_out[0], gpu_out[1], gpu_out[2], gpu_out[3]];
        assert_eq!(
            got_quad, want,
            "case {i}: args {args:?} — gpu {got_quad:?} != interpreter {want:?}\nsrc: {src}"
        );
        assert_eq!(steps_of(gpu_out), want_steps, "case {i}: steps\nsrc: {src}");
        let sb = rustmsl::STATE_BASE as usize;
        let want_state = &interp.mem[sb..sb + state_len];
        assert_eq!(
            &state_out[i * state_len..(i + 1) * state_len],
            want_state,
            "case {i}: final state bytes\nsrc: {src}"
        );
    }
}

#[test]
fn state_field_roundtrip_and_mutation() {
    // Two u16 fields: one read, one written — the StateCell shape.
    check_state(
        "struct S { x: u16, score: u16 }\n\
         impl S { fn run(&mut self) -> u16 { self.score = self.x * 2u16 + self.score; self.score } }",
        "S::run",
        4,
        &sweep(0x5eed_0020, 2000),
        0x57a7_0001,
    );
}

#[test]
fn state_dependent_control_flow() {
    // A state machine step: branches on a state field, mutates two others.
    check_state(
        "struct Cb { st: u16, fails: u16, thresh: u16 }\n\
         impl Cb { fn run(&mut self) -> u16 {\n\
             if self.st == 0u16 { self.fails = self.fails + 1u16; if self.fails >= self.thresh { self.st = 1u16; } }\n\
             else { self.st = 0u16; self.fails = 0u16; }\n\
             self.st } }",
        "Cb::run",
        6,
        &sweep(0x5eed_0021, 2000),
        0x57a7_0002,
    );
}

#[test]
fn state_loop_over_array_field() {
    // An array state field walked by a loop — the sliding-window shape.
    check_state(
        "struct W { buf: [u16; 8], n: u16 }\n\
         impl W { fn run(&mut self) -> u16 {\n\
             let mut s = 0u16;\n\
             for i in 0..8u16 { s = s + self.buf[i]; }\n\
             self.n = self.n + 1u16;\n\
             s } }",
        "W::run",
        18,
        &sweep(0x5eed_0022, 1500),
        0x57a7_0003,
    );
}

#[test]
fn state_u32_field_and_extra_arg() {
    // A wide (u32) state field plus a scalar argument after &mut self.
    check_state(
        "struct Acc { total: u32 }\n\
         impl Acc { fn run(&mut self, x: u16) -> u16 {\n\
             self.total = self.total + (x as u32);\n\
             (self.total >> 16u32) as u16 } }",
        "Acc::run",
        4,
        &sweep(0x5eed_0023, 2000),
        0x57a7_0004,
    );
}

#[test]
fn state_div_trap_leaves_partial_state_identically() {
    // A trap mid-mutation: the state bytes at the trap point must match too.
    check_state(
        "struct D { a: u16, b: u16 }\n\
         impl D { fn run(&mut self) -> u16 { self.a = self.a + 1u16; let q = 1000u16 / self.b; self.b = q; q } }",
        "D::run",
        4,
        &sweep(0x5eed_0024, 2000),
        0x57a7_0005,
    );
}
