//! GPU parity for the bytecode-interpreter backend: linearized cells run on
//! the GPU interp kernel must agree with [`rustmsl::interp::cpu_run`] — the
//! portable reference VM (itself pinned to `cell80_core::Interp` by the
//! crate's unit tests) — on values, trap status, and step counts.
//!
//! Runs on whichever backend this build has: `InterpBatch` (Metal) on
//! macOS, `CudaInterpBatch` under `--features cuda` elsewhere — the same
//! dispatch rule as the compiled-backend corner battery.
//!
//! Fuel-exhaustion inputs are deliberately absent: the GPU kernel coalesces
//! `Step` runs within a basic block, so a trap that lands mid-block reports
//! the post-block count while `cpu_run` (uncoalesced) stops at the exact
//! threshold. Completed runs are identical (coalescing never crosses a jump
//! target); the compiled-backend corners pin fuel-trap semantics.

#![cfg(any(target_os = "macos", feature = "cuda"))]

use rustmsl::interp::{cpu_run, linearize, CellProgram, VmOut};

#[cfg(target_os = "macos")]
fn gpu_run(progs: &[CellProgram], probes: &[[u16; 3]]) -> Vec<[u16; 6]> {
    let (batch, skipped) = rustmsl::interp::InterpBatch::new(progs).expect("interp batch");
    assert_eq!(skipped, 0, "corpus cell over the local-slot bound");
    batch.run(probes)
}

#[cfg(all(not(target_os = "macos"), feature = "cuda"))]
fn gpu_run(progs: &[CellProgram], probes: &[[u16; 3]]) -> Vec<[u16; 6]> {
    let (batch, skipped) = rustmsl::CudaInterpBatch::new(progs).expect("cuda interp batch");
    assert_eq!(skipped, 0, "corpus cell over the local-slot bound");
    batch.run(probes).expect("cuda interp run")
}

/// Lower + linearize a snippet's `run` (the interp backend's front door).
fn cell(src: &str) -> CellProgram {
    let file: syn::File =
        syn::parse_str(src).unwrap_or_else(|e| panic!("parse failed: {e}\nsrc: {src}"));
    let lowered = rustz80::lower_program_full(&file, &rustz80::PreludeConfig::default())
        .unwrap_or_else(|e| panic!("lower failed: {e}\nsrc: {src}"));
    linearize(&lowered.funcs, "run")
        .unwrap_or_else(|e| panic!("linearize bailed: {e:?}\nsrc: {src}"))
}

/// The seeded probe schedule (the `cell_fuzz` xorshift, no `rand`).
fn probes(seed: u64, n: usize) -> Vec<[u16; 3]> {
    let corners: &[u16] = &[0, 1, 2, 0x7F, 0x80, 0xFF, 0x7FFF, 0x8000, 0xFFFF];
    let mut v: Vec<[u16; 3]> = Vec::new();
    for &a in corners {
        for &b in corners {
            v.push([a, b, 3]);
        }
    }
    let mut x = seed;
    let mut next = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x as u16
    };
    for _ in 0..n {
        v.push([next(), next(), next()]);
    }
    v
}

/// Grade one (cell, probe) sextet against the reference VM. Div-by-zero
/// carries no step count in [`VmOut`], so its steps are not compared.
fn check_one(src: &str, prog: &CellProgram, probe: &[u16; 3], got: &[u16; 6]) {
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
        VmOut::Fuel(_) => panic!("fuel-exhausting corpus cell — not this battery's scope"),
        VmOut::DivZero => ([0, 0, 0, 1], None),
    };
    let got_quad = [got[0], got[1], got[2], got[3]];
    assert_eq!(
        got_quad, want_quad,
        "probe {probe:?}: gpu {got_quad:?} != vm {want_quad:?}\nsrc: {src}"
    );
    if let Some(s) = want_steps {
        let got_steps = got[4] as u64 | ((got[5] as u64) << 16);
        assert_eq!(
            got_steps, s,
            "probe {probe:?}: gpu steps != vm steps\nsrc: {src}"
        );
    }
}

/// The corpus — the interp-supported subset's shapes (no runtime-amount
/// shifts, no memory ops, no `for`, calls fully inlined), one battery run:
/// all cells × all probes in one GPU dispatch, cell-major readback.
#[test]
fn interp_kernel_matches_cpu_vm() {
    let sources = [
        "fn run(x: u16, y: u16) -> u16 { (x + y) * (x ^ y) - (x & y) }",
        "fn run(x: u16, y: u16) -> u16 { let a = (x as u8) as u16; let b = (y as u8) as u16; (a * b) & 0xFFFF }",
        "fn run(a: i16, b: i16) -> i16 { (a / b) + (a % b) }",
        "fn run(a: u16, b: u16) -> u16 { (a / b) * b + (a % b) }",
        "fn run(x: u16) -> u16 { (x << 3) ^ (x >> 15) ^ (x << 15) ^ ((x as i16) >> 2) as u16 }",
        "fn run(x: u16) -> u16 { x.count_ones() + (x.leading_zeros() << 5) + (x.trailing_zeros() << 10) }",
        "fn run(a: u16, b: u16) -> u16 { if a > 0 && b / a > 2 { 1 } else { 0 } }",
        "fn run(x: u16) -> u16 { if x > 40000 { halt(7); } x + 1 }",
        "fn run(a: u16, b: u16) -> u16 { let mut x = a; let mut y = b; while y != 0 { let t = x % y; x = y; y = t; } x }",
        "fn run(x: u16) -> u16 { let mut v = x | 1; let mut n = 0; loop { if v == 1 { break; } if v % 2 == 0 { v = v / 2; } else { v = (v & 8191) * 3 + 1; } n = n + 1; if n > 400 { break; } } n }",
        "fn run(a: u16, b: u16, c: u16) -> u32 { let x = ((a as u32) << 16) | (b as u32); let d = c as u32; (x / (d + 1)) ^ (x * d) ^ (x >> 5) }",
        "fn run(a: i16, b: i16) -> i32 { let x = (a as i32) << 8; let y = (b as i32) | 1; (x / y) + (x % y) }",
        "fn run(a: i16, b: u16) -> u16 { (((a as i32) < (b as i32)) as u16) | ((((a as i32) >= 0) as u16) << 1) }",
        "fn helper(v: u16) -> u16 { if v > 100 { v - 100 } else { 100 - v } }\n\
         fn run(x: u16, y: u16) -> u16 { helper(x) + helper(y) }",
    ];
    let progs: Vec<CellProgram> = sources.iter().map(|s| cell(s)).collect();
    let ps = probes(0x5eed_127b, 500);
    let got = gpu_run(&progs, &ps);
    assert_eq!(got.len(), progs.len() * ps.len());
    for (ci, (src, prog)) in sources.iter().zip(&progs).enumerate() {
        for (pi, probe) in ps.iter().enumerate() {
            check_one(src, prog, probe, &got[ci * ps.len() + pi]);
        }
    }
}
