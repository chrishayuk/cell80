//! Pre-silicon semantics battery for the CUDA dialect: the emitted CUDA
//! text, compiled by the **host C++ compiler** behind [`rustmsl::cpu_emu`]'s
//! shim, must agree bit-for-bit with the reference interpreter — values,
//! trap statuses, IR-step counts, and final state bytes — on exactly the
//! corners where the dialect's own emission choices carry the semantics
//! (guarded runtime shifts, signed div/rem `MIN/-1`, the `__popc`/`__clz`/
//! `__ffs` intrinsic mappings, trap folding, byte wrap, the do-while
//! `continue` wrapper, typed state incl. trap-point partial state).
//!
//! This is the corner battery's shape (`corners.rs`) pointed at the CUDA
//! text instead of a GPU. It runs everywhere a C++ compiler exists — no
//! NVIDIA hardware — and is what lets the docs/16 cloud session start from
//! "semantics already validated; only NVRTC acceptance and NVIDIA codegen
//! left to prove". It is NOT silicon verification and is never cited as
//! such.

use cell80_core::{Interp, Target};
use rustmsl::{cpu_emu, steps_of, STATUS_DIV0, STATUS_FUEL, STATUS_HALT, STATUS_OK};

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

/// Lower `src`, run every input on the interpreter and once through the
/// CUDA text on the CPU, assert quads + step counts agree bit for bit.
fn check(src: &str, inputs: &[[u16; 3]]) {
    let file: syn::File =
        syn::parse_str(src).unwrap_or_else(|e| panic!("parse failed: {e}\nsrc: {src}"));
    let lowered = rustz80::lower_program_full(&file, &rustz80::PreludeConfig::default())
        .unwrap_or_else(|e| panic!("lower failed: {e}\nsrc: {src}"));
    let consts = lowered.const_data();
    let module = rustmsl::compile_cuda(&lowered.funcs, &consts, "run")
        .unwrap_or_else(|e| panic!("cuda compile failed: {e}\nsrc: {src}"));
    let got = cpu_emu::run(&module, inputs)
        .unwrap_or_else(|e| panic!("emu failed: {e}\nsource:\n{}", module.source));
    let n_args = module.cells[0].params;
    for (i, (args, emu_out)) in inputs.iter().zip(&got).enumerate() {
        let mut interp = Interp::new(
            &lowered.funcs,
            consts.iter().map(|(n, b)| (n.as_str(), b.as_slice())),
            Target::Cell.descriptor(),
        );
        let res = interp.run("run", &args[..n_args]);
        let want = interp_quad(res);
        let want_steps = u32::try_from(interp.steps()).expect("steps fit u32");
        let got_quad = [emu_out[0], emu_out[1], emu_out[2], emu_out[3]];
        assert_eq!(
            got_quad, want,
            "case {i}: args {args:?} — cuda-text {got_quad:?} != interpreter {want:?}\nsrc: {src}"
        );
        assert_eq!(
            steps_of(emu_out),
            want_steps,
            "case {i}: args {args:?} — cuda-text steps != interpreter steps\nsrc: {src}"
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
fn shift_and_div_corners_match_interp() {
    // The R1 corners where the dialect's guards carry the semantics: runtime
    // shift counts ≥ width (min clamp / select-to-zero), signed arithmetic
    // shift saturation, and signed div/rem MIN/-1 wrapping via the helpers.
    check(
        "fn run(x: u16, n: u16) -> u16 { (x << n) | (x >> n) }",
        &sweep(0x5eed_0001, 2000),
    );
    check(
        "fn run(x: i16, n: u16) -> i16 { x >> n }",
        &sweep(0x5eed_0002, 2000),
    );
    check(
        "fn run(a: i16, b: i16) -> i16 { (a / b) + (a % b) }",
        &sweep(0x5eed_0004, 2000),
    );
    check(
        "fn run(a: u16, b: u16) -> u16 { (a / b) * b + (a % b) }",
        &sweep(0x5eed_0005, 2000),
    );
}

#[test]
fn i32_div_and_width_bridges_match_interp() {
    // i32 MIN/-1 (C++ UB, selected out in the helpers), u32 div/mul/shift,
    // sign-extension bridges, and byte-width wrap through the CUDA casts.
    check(
        "fn run(a: i16, b: i16) -> i32 { let x = (a as i32) << 16; let y = b as i32; (x / y) + (x % y) }",
        &sweep(0x5eed_000a, 2000),
    );
    check(
        "fn run(a: u16, b: u16, c: u16) -> u32 { let x = ((a as u32) << 16) | (b as u32); let d = c as u32; (x / (d + 1)) ^ (x * d) ^ (x >> 5) }",
        &sweep(0x5eed_0009, 2000),
    );
    check(
        "fn run(a: i16, b: u16) -> u32 { (a as u32) + ((b as u32) << 3) }",
        &sweep(0x5eed_000b, 2000),
    );
    check(
        "fn run(x: u16, y: u16) -> u16 { let a = x as u8; let b = y as u8; ((a + b) as u16) | (((a * b) as u16) << 8) }",
        &sweep(0x5eed_0006, 2000),
    );
}

#[test]
fn bits_intrinsics_match_interp() {
    // The __popc/__clz/__ffs mappings: zero-extended u16 operands, the
    // 32-bit __clz minus 16, the 1-based __ffs behind the zero guard.
    check(
        "fn run(x: u16) -> u16 { x.count_ones() + (x.leading_zeros() << 5) + (x.trailing_zeros() << 10) }",
        &sweep(0x5eed_000c, 2000),
    );
}

#[test]
fn traps_and_short_circuit_match_interp() {
    // Trap folding (halt code on r0, div0) and the short-circuit that must
    // hide a divide — and its ticks — exactly as the interpreter does.
    check(
        "fn run(a: u16, b: u16) -> u16 { if a > 0 && b / a > 2 { 1 } else { 0 } }",
        &sweep(0x5eed_0008, 2000),
    );
    check(
        "fn run(x: u16) -> u16 { if x > 40000 { halt(7); } x + 1 }",
        &sweep(0x5eed_000d, 1000),
    );
}

#[test]
fn loops_and_continue_wrapper_match_interp() {
    // Data-dependent loop counts (step parity proves iteration counts), and
    // `continue` reaching the for-loop induction step through the
    // do-while(false) wrapper.
    check(
        "fn run(a: u16, b: u16) -> u16 { let mut x = a; let mut y = b; while y != 0 { let t = x % y; x = y; y = t; } x }",
        &sweep(0x5eed_0010, 2000),
    );
    check(
        "fn run(n: u16, m: u16) -> u16 { let mut s = 0; for i in 0..(n & 127) { if i % 3 == 0 { continue; } if i == m { continue; } s = s + i; } s }",
        &sweep(0x5eed_0012, 1500),
    );
    check(
        "fn run(a: u16, b: u16) -> u16 { let mut s = 0; for i in 0..(a & 31) { for j in 0..(b & 31) { if j == i { continue; } if j > 20 { break; } s = s + 1; } if s > 400 { break; } } s }",
        &sweep(0x5eed_0014, 1000),
    );
}

#[test]
fn runaway_loop_is_a_fuel_trap() {
    // The budget bound: a loop that never exits burns exactly FUEL steps in
    // the CUDA text too — one full 10⁸-tick run on the CPU, so this case
    // rides a tiny input set.
    check(
        "fn run(x: u16) -> u16 { let mut s = 0; loop { s = s + 1; if x > 65534 { if s == 0 { break; } } } s }",
        &[[1, 0, 0], [65535, 0, 0]],
    );
}

// ── typed state through the CUDA text ─────────────────────────────────────

/// The state twin of [`check`]: per-thread state in/out, final state bytes
/// (including at a trap point) must match the interpreter's memory.
fn check_state(src: &str, entry: &str, state_len: usize, inputs: &[[u16; 3]], seed: u64) {
    let file: syn::File =
        syn::parse_str(src).unwrap_or_else(|e| panic!("parse failed: {e}\nsrc: {src}"));
    let lowered = rustz80::lower_program_full(&file, &rustz80::PreludeConfig::default())
        .unwrap_or_else(|e| panic!("lower failed: {e}\nsrc: {src}"));
    let consts = lowered.const_data();
    let module = rustmsl::compile_library_cuda(&[rustmsl::LibraryCell {
        funcs: &lowered.funcs,
        consts: &consts,
        entry,
        state_len,
    }])
    .unwrap_or_else(|e| panic!("cuda compile failed: {e}\nsrc: {src}"));

    let mut rng = Rng(seed);
    let mut state_in = vec![0u8; state_len * inputs.len()];
    for b in state_in.iter_mut() {
        *b = rng.next() as u8;
    }
    let (got, state_out) = cpu_emu::run_with_state(&module, inputs, &state_in)
        .unwrap_or_else(|e| panic!("emu failed: {e}\nsource:\n{}", module.source));

    let n_args = module.cells[0].params;
    for (i, (args, emu_out)) in inputs.iter().zip(&got).enumerate() {
        let mut interp = Interp::new(
            &lowered.funcs,
            consts.iter().map(|(n, b)| (n.as_str(), b.as_slice())),
            Target::Cell.descriptor(),
        );
        interp.plant(
            rustmsl::STATE_BASE,
            &state_in[i * state_len..(i + 1) * state_len],
        );
        let mut call: Vec<u16> = vec![rustmsl::STATE_BASE];
        call.extend_from_slice(&args[..n_args.saturating_sub(1)]);
        let res = interp.run(entry, &call);
        let want = interp_quad(res);
        let want_steps = u32::try_from(interp.steps()).expect("steps fit u32");
        let got_quad = [emu_out[0], emu_out[1], emu_out[2], emu_out[3]];
        assert_eq!(
            got_quad, want,
            "case {i}: args {args:?} — cuda-text {got_quad:?} != interpreter {want:?}\nsrc: {src}"
        );
        assert_eq!(steps_of(emu_out), want_steps, "case {i}: steps\nsrc: {src}");
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
fn state_roundtrip_and_trap_partial_state_match_interp() {
    check_state(
        "struct S { x: u16, score: u16 }\n\
         impl S { fn run(&mut self) -> u16 { self.score = self.x * 2u16 + self.score; self.score } }",
        "S::run",
        4,
        &sweep(0x5eed_0020, 1000),
        0x57a7_0001,
    );
    // A trap mid-mutation: the state bytes at the trap point must match too.
    check_state(
        "struct D { a: u16, b: u16 }\n\
         impl D { fn run(&mut self) -> u16 { self.a = self.a + 1u16; let q = 1000u16 / self.b; self.b = q; q } }",
        "D::run",
        4,
        &sweep(0x5eed_0024, 1000),
        0x57a7_0005,
    );
    // A wide (u32) state field plus a scalar argument after &mut self.
    check_state(
        "struct Acc { total: u32 }\n\
         impl Acc { fn run(&mut self, x: u16) -> u16 {\n\
             self.total = self.total + (x as u32);\n\
             (self.total >> 16u32) as u16 } }",
        "Acc::run",
        4,
        &sweep(0x5eed_0023, 1000),
        0x57a7_0004,
    );
}

// ── the interp kernel's CUDA text vs the portable reference VM ────────────

#[test]
fn interp_kernel_cuda_text_matches_cpu_vm() {
    use rustmsl::interp::{cpu_run, linearize, VmOut};
    let sources = [
        "fn run(x: u16, y: u16) -> u16 { (x + y) * (x ^ y) - (x & y) }",
        "fn run(a: i16, b: i16) -> i16 { (a / b) + (a % b) }",
        "fn run(x: u16) -> u16 { (x << 3) ^ (x >> 15) ^ ((x as i16) >> 2) as u16 }",
        "fn run(x: u16) -> u16 { x.count_ones() + (x.leading_zeros() << 5) + (x.trailing_zeros() << 10) }",
        "fn run(a: u16, b: u16) -> u16 { if a > 0 && b / a > 2 { 1 } else { 0 } }",
        "fn run(x: u16) -> u16 { if x > 40000 { halt(7); } x + 1 }",
        "fn run(a: u16, b: u16) -> u16 { let mut x = a; let mut y = b; while y != 0 { let t = x % y; x = y; y = t; } x }",
        "fn run(a: u16, b: u16, c: u16) -> u32 { let x = ((a as u32) << 16) | (b as u32); let d = c as u32; (x / (d + 1)) ^ (x * d) ^ (x >> 5) }",
        "fn run(a: i16, b: i16) -> i32 { let x = (a as i32) << 8; let y = (b as i32) | 1; (x / y) + (x % y) }",
    ];
    let progs: Vec<_> = sources
        .iter()
        .map(|src| {
            let file: syn::File = syn::parse_str(src).unwrap();
            let lowered =
                rustz80::lower_program_full(&file, &rustz80::PreludeConfig::default()).unwrap();
            linearize(&lowered.funcs, "run")
                .unwrap_or_else(|e| panic!("linearize bailed: {e:?}\nsrc: {src}"))
        })
        .collect();
    let probes = sweep(0x5eed_127c, 300);
    let (got, skipped) = cpu_emu::run_interp(&progs, &probes).expect("interp emu");
    assert_eq!(skipped, 0);
    assert_eq!(got.len(), progs.len() * probes.len());
    for (ci, (src, prog)) in sources.iter().zip(&progs).enumerate() {
        for (pi, probe) in probes.iter().enumerate() {
            let out = &got[ci * probes.len() + pi];
            let args = &probe[..prog.params.min(3)];
            let (want_quad, want_steps) = match cpu_run(prog, args) {
                VmOut::Value(v, s) => (
                    [
                        v.first().copied().unwrap_or(0),
                        v.get(1).copied().unwrap_or(0),
                        v.get(2).copied().unwrap_or(0),
                        0,
                    ],
                    Some(s),
                ),
                VmOut::Halt(code, s) => ([code, 0, 0, 2], Some(s)),
                VmOut::Fuel(_) => panic!("fuel-exhausting corpus cell"),
                VmOut::DivZero => ([0, 0, 0, 1], None),
            };
            let got_quad = [out[0], out[1], out[2], out[3]];
            assert_eq!(
                got_quad, want_quad,
                "probe {probe:?}: cuda-text {got_quad:?} != vm {want_quad:?}\nsrc: {src}"
            );
            if let Some(s) = want_steps {
                let got_steps = out[4] as u64 | ((out[5] as u64) << 16);
                assert_eq!(got_steps, s, "probe {probe:?}: steps\nsrc: {src}");
            }
        }
    }
}
