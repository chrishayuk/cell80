//! The E1+E2 library battery (Phase 6 WS-E, docs 14): every integer **value
//! cell** in the library — straight-line (E1) and looping (E2) — compiled to
//! MSL and run on the system's Metal device against the reference
//! interpreter. The full `[r0, r1, r2, status]` quad **and the IR-step
//! count** (docs 14 Q2 — the canonical family cost, metered identically on
//! both substrates) must agree bit for bit on every input, or the cell is a
//! filed defect.
//!
//! The E3 layouts ride the same kernel shape: `library_megakernel` fuses every
//! eligible cell into one translation unit and runs the whole library × a
//! probe set in a single launch — retrieval by execution's substrate (WS-F).
//! The `throughput_*` benches (ignored) print measured evals/s; docs 14's
//! ≥10⁸/s target is benchmarked, never assumed.
//!
//! Coverage is honest, not silent (docs 14 "no silent caps"): cells outside
//! the fragment refuse at codegen with a typed reason and are *counted* —
//! f32 is E4, state cells await typed-state readback on the GPU host path
//! (owed with the host integration).
//!
//! **Oracle transcripts** (docs 12's fact-file idea applied to this gate):
//! the interpreter's verdict for a `(cell, input schedule)` is deterministic,
//! so it memoizes as a digest in `tests/golden/msl_oracle_transcripts.json`,
//! keyed by the combined source hash. A hit turns grading into GPU-run +
//! digest compare — no interpreter wall clock at all; a miss or disagreement
//! falls back to the live oracle (which stays the only authority — a stale
//! transcript costs a live grade, never a verdict). The pre-registered gate
//! (10⁶ random inputs per cell) runs — and re-blesses transcripts for new or
//! changed cells — via:
//!
//! ```sh
//! UPDATE_GOLDEN=1 cargo test --release -p cell80 --test msl_battery \
//!     -- --ignored --nocapture
//! ```
//!
//! A deliberate *interpreter semantics* change must regenerate every
//! transcript (delete the file and re-bless); the always-live corner battery
//! in `rustmsl/tests/corners.rs` guards that seam on every push.
//!
//! The battery loops, oracle, transcripts, and cell discovery live in
//! `battery_common/` — shared verbatim with the CUDA battery
//! (`cuda_battery.rs`), so the two backends cannot drift. This file supplies
//! the Metal [`Backend`] plus the Metal-only diagnostics.
//!
//! **A defect class this battery's own discipline owns, named for future integrators
//! (found 2026-07-12, latent since this file's own earliest form):** any *other* CPU/GPU
//! dual-execution harness built on these same primitives (`Interp`/`GpuBatch`) must fold
//! a trap (div-by-zero, fuel exhaustion, `halt(code)`) to the *same* defined `r0` convention
//! this battery already tests for (`STATUS_DIV0`/`STATUS_FUEL`/`STATUS_HALT` above) — it is
//! not automatically inherited. `experiments/cell80-life/`'s own CPU-reference/GPU dual-body
//! engine independently re-derived this and got it wrong the first time (a bare `unwrap`
//! that panicked on a legitimate trap), invisible for months because its six curated genes
//! never tripped one; only pointing an *evolutionary* search at uncurated cells with
//! mutation-selected, adversarial inputs (no hand-designed test battery would have picked
//! those inputs) surfaced it immediately. The lesson generalizes: a curated test battery
//! encodes its author's priors about where bugs are; an evolutionary/fuzzing process has no
//! such priors and finds the corners no one thought to write a test for. Any new consumer of
//! `Interp`/`GpuBatch` should copy this battery's trap-folding convention explicitly, not
//! assume it comes for free.

#![cfg(target_os = "macos")]

mod battery_common;

use battery_common::*;
use rustmsl::{steps_of, GpuBatch};
use std::path::Path;

/// The Metal backend: MSL dialect + `GpuBatch`, and the blessing authority
/// for the shared oracle-transcript book.
const MSL: Backend = Backend {
    label: "msl",
    bless: true,
    compile: rustmsl::compile,
    compile_library: rustmsl::compile_library,
    run: msl_run,
    run_with_state: msl_run_with_state,
};

fn msl_run(m: &rustmsl::GpuModule, inputs: &[[u16; 3]]) -> Result<Vec<[u16; 6]>, String> {
    GpuBatch::new(m)?.run(inputs)
}

fn msl_run_with_state(
    m: &rustmsl::GpuModule,
    inputs: &[[u16; 3]],
    state_in: &[u8],
) -> Result<(Vec<[u16; 6]>, Vec<u8>), String> {
    GpuBatch::new(m)?.run_with_state(inputs, state_in)
}

/// The CI-speed battery: every eligible cell, a corner sweep + 512 random
/// inputs each. The full pre-registered gate is [`gate_one_million`].
#[test]
fn e1_e2_battery() {
    let n = std::env::var("CELL80_MSL_FUZZ_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(512);
    value_battery(n, &MSL);
}

/// The E1+E2 gate (docs 14): 10⁶ random inputs per admitted integer value
/// cell, values + status + steps bit-exact. Ignored by default — run in
/// release; the interpreter side dominates the wall clock.
#[test]
#[ignore = "the 10^6-input gate — run explicitly in release"]
fn gate_one_million() {
    value_battery(1_000_000, &MSL);
}

/// E3: the whole library fused into one translation unit, one dispatch.
#[test]
fn library_megakernel_matches_interpreter() {
    megakernel_battery(&MSL);
}

/// The CI-speed state battery: corner sweep + 256 random (input, state) pairs
/// per cell. The full gate is [`state_gate_one_million`].
#[test]
fn state_cells_battery() {
    let n = std::env::var("CELL80_MSL_FUZZ_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(256);
    state_battery(n, &MSL);
}

/// The state-cell gate: 10⁶ random (input, state) pairs per cell — values,
/// status, steps, AND final state bytes bit-exact. Run with `UPDATE_GOLDEN=1`
/// to bless transcripts; cached re-runs take seconds.
#[test]
#[ignore = "the 10^6-input state gate — run explicitly in release"]
fn state_gate_one_million() {
    state_battery(1_000_000, &MSL);
}

// ── Metal-only diagnostics (throughput, divergence, cost maps, rewrites) ───

/// E3 throughput, layout 1 (one cell × N inputs): steady-state end-to-end
/// evals/s (buffer setup + dispatch + readback included — the honest number).
#[test]
#[ignore = "throughput bench — run in release with --nocapture"]
fn throughput_one_cell() {
    let src = "fn run(x: u16, lo: u16, hi: u16) -> u16 { if x > hi { hi } else if x < lo { lo } else { x } }";
    let (funcs, consts, _) = lower_cell(src).unwrap();
    let module = rustmsl::compile(&funcs, &consts, "run").unwrap();
    let gpu = GpuBatch::new(&module).unwrap();

    let mut rng = Rng(0x5eed_e301);
    for n in [1 << 16, 1 << 20, 1 << 22, 1 << 24] {
        let inputs: Vec<[u16; 3]> = (0..n).map(|_| [rng.u16(), rng.u16(), rng.u16()]).collect();
        // Warm once, then time the steady state.
        gpu.run(&inputs).unwrap();
        let reps = 5;
        let t0 = std::time::Instant::now();
        for _ in 0..reps {
            gpu.run(&inputs).unwrap();
        }
        let dt = t0.elapsed().as_secs_f64() / reps as f64;
        println!(
            "one-cell throughput: N={n:>9} — {:>7.1} ms/launch, {:.2e} evals/s",
            dt * 1e3,
            n as f64 / dt
        );
    }
}

/// E3 throughput, layout 2 (library × probe set): whole-library launch
/// latency at fingerprint-probe scale and at retrieval scale.
#[test]
#[ignore = "throughput bench — run in release with --nocapture"]
fn throughput_library() {
    let lib = eligible_cells();
    let compilable: Vec<&(String, Funcs, Consts, String)> = lib
        .cells
        .iter()
        .filter(|(_, funcs, consts, _)| rustmsl::compile(funcs, consts, "run").is_ok())
        .collect();
    let cells: Vec<rustmsl::LibraryCell> = compilable
        .iter()
        .map(|(_, funcs, consts, _)| rustmsl::LibraryCell {
            funcs,
            consts,
            entry: "run",
            state_len: 0,
        })
        .collect();
    let t0 = std::time::Instant::now();
    let module = rustmsl::compile_library(&cells).expect("library compile");
    let t_codegen = t0.elapsed();
    let t0 = std::time::Instant::now();
    let gpu = GpuBatch::new(&module).unwrap();
    let t_metal = t0.elapsed();
    println!(
        "library codegen: {} cells in {:.1} ms (MSL {} KiB), metal compile {:.1} ms",
        cells.len(),
        t_codegen.as_secs_f64() * 1e3,
        module.source.len() / 1024,
        t_metal.as_secs_f64() * 1e3
    );
    let mut rng = Rng(0x5eed_e302);
    for n_probes in [8usize, 64, 512] {
        let probes: Vec<[u16; 3]> = (0..n_probes)
            .map(|_| [rng.u16(), rng.u16(), rng.u16()])
            .collect();
        gpu.run(&probes).unwrap();
        let reps = 10;
        let t0 = std::time::Instant::now();
        for _ in 0..reps {
            gpu.run(&probes).unwrap();
        }
        let dt = t0.elapsed().as_secs_f64() / reps as f64;
        let evals = cells.len() * n_probes;
        println!(
            "library×probes: {} cells × {n_probes:>3} probes = {evals:>7} evals — \
             {:>6.2} ms/launch, {:.2e} evals/s",
            cells.len(),
            dt * 1e3,
            evals as f64 / dt
        );
    }
}

/// The E2 divergence probe: gcd (data-dependent loop count) with uniform vs
/// shuffled random inputs. The hypothesis under test (docs 14, E2):
/// WCET-friendly ≈ SIMT-friendly — divergence costs what the *worst lane*
/// costs, so the ratio should track max/mean steps, not explode.
#[test]
#[ignore = "divergence probe — run in release with --nocapture"]
fn divergence_probe_gcd() {
    let src = "fn run(a: u16, b: u16) -> u16 { let mut x = a; let mut y = b; while y != 0 { let t = x % y; x = y; y = t; } x }";
    let file: syn::File = syn::parse_str(src).unwrap();
    let lowered = rustz80::lower_program_full(&file, &rustz80::PreludeConfig::default()).unwrap();
    let consts = lowered.const_data();
    let module = rustmsl::compile(&lowered.funcs, &consts, "run").unwrap();
    let gpu = GpuBatch::new(&module).unwrap();

    let n = 1 << 22;
    let mut rng = Rng(0x5eed_e303);
    // Worst-case-heavy random pairs (gcd of consecutive Fibonaccis is the WCET
    // shape; random uniforms already spread loop counts 1..~40).
    let random: Vec<[u16; 3]> = (0..n).map(|_| [rng.u16(), rng.u16(), 0]).collect();
    // Uniform: every lane the same (fib pair — deep but identical count).
    let uniform: Vec<[u16; 3]> = (0..n).map(|_| [46368, 28657, 0]).collect();

    for (label, inputs) in [
        ("uniform (deep)", &uniform),
        ("random (divergent)", &random),
    ] {
        gpu.run(inputs).unwrap();
        let reps = 5;
        let t0 = std::time::Instant::now();
        let mut out = Vec::new();
        for _ in 0..reps {
            out = gpu.run(inputs).unwrap();
        }
        let dt = t0.elapsed().as_secs_f64() / reps as f64;
        let (mut sum, mut max) = (0u64, 0u32);
        for o in &out {
            let s = steps_of(o);
            sum += s as u64;
            max = max.max(s);
        }
        println!(
            "divergence gcd {label:>18}: {:>6.2} ms/launch, {:.2e} evals/s, steps mean {:.1} max {max}",
            dt * 1e3,
            n as f64 / dt,
            sum as f64 / n as f64
        );
    }
}

/// Diagnostic: project the blessing gate's oracle cost per cell from the
/// GPU's own step counts (512-input sample × 10⁶/sample scaling). Prints the
/// heaviest cells and the cumulative distribution in processing order — the
/// map of where a blessing run's wall clock goes.
#[test]
#[ignore = "cost-map diagnostic — run with --nocapture"]
fn gate_cost_estimate() {
    let lib = eligible_cells();
    let mut rows: Vec<(String, u64, u64)> = Vec::new(); // (name, mean, worst steps/input)
    for (name, funcs, consts, src_hash) in lib.cells.iter() {
        let Ok(module) = rustmsl::compile(funcs, consts, "run") else {
            continue;
        };
        let seed = cell_seed(src_hash, 0x5eed_e100);
        let inputs = gen_inputs(512, seed);
        let gpu = GpuBatch::new(&module).unwrap();
        let got = gpu.run(&inputs).unwrap();
        let total: u64 = got.iter().map(|o| steps_of(o) as u64).sum();
        let worst: u64 = got.iter().map(|o| steps_of(o) as u64).max().unwrap_or(0);
        rows.push((name.clone(), total / inputs.len() as u64, worst));
    }
    let grand: u64 = rows.iter().map(|(_, m, _)| m * 1_000_000).sum();
    println!(
        "projected gate oracle cost: {:.2e} ticks total",
        grand as f64
    );
    let mut cum = 0u64;
    for (name, mean, worst) in &rows {
        cum += mean * 1_000_000;
        if *mean > 10_000 {
            println!(
                "  {name:>28}: mean {mean:>7} worst {worst:>8} — cumulative {:5.1}%",
                100.0 * cum as f64 / grand as f64
            );
        }
    }
    let mut top: Vec<_> = rows.iter().collect();
    top.sort_by_key(|(_, _, w)| std::cmp::Reverse(*w));
    println!("top 12 by WORST case (the step-budget number):");
    for (name, mean, worst) in top.iter().take(12) {
        println!("  {name:>28}: worst {worst:>8}  mean {mean}");
    }
}

/// The cell-rewrite audit: for each listed cell, the committed implementation
/// (HEAD) and the working tree's run the same inputs on the GPU; values and
/// trap status must agree input-for-input (steps may differ — that's usually
/// the point of a rewrite). Run this BEFORE committing any behaviour-critical
/// cell rewrite — edit the list to the cells under audit. The GPU makes
/// auditing even a 2M-steps-per-input implementation affordable (~seconds),
/// and it caught a real guard bug in the first offender-fix attempt. After
/// the rewrite commits, HEAD equals the working tree and the audit is
/// trivially green — the list is per-rewrite, not a regression suite.
#[test]
#[ignore = "offender-rewrite differential — run with --nocapture"]
fn offender_rewrites_are_value_identical() {
    let offenders = [
        "cell80/cells/calendrical-checksum/day_of_year.rs",
        "cell80/cells/number-theory/pow_small.rs",
        "cell80/cells/number-theory/wilson_theorem_check.rs",
        "cell80/cells/number-theory/sum_digit_powers.rs",
        "cell80/cells/number-theory/wilson_factorial_mod.rs",
        "cell80/cells/number-theory/is_quadratic_residue.rs",
    ];
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    for rel in offenders {
        let old_src = String::from_utf8(
            std::process::Command::new("git")
                .args(["show", &format!("HEAD:{rel}")])
                .current_dir(&repo)
                .output()
                .expect("git show")
                .stdout,
        )
        .unwrap();
        let new_src = std::fs::read_to_string(repo.join(rel)).unwrap();
        let (of, oc, _) = lower_cell(&old_src).unwrap();
        let (nf, nc, _) = lower_cell(&new_src).unwrap();
        let old_gpu = GpuBatch::new(&rustmsl::compile(&of, &oc, "run").unwrap()).unwrap();
        let new_gpu = GpuBatch::new(&rustmsl::compile(&nf, &nc, "run").unwrap()).unwrap();
        let inputs = gen_inputs(300_000, cell_seed("0123456789abcdef", 0xd1ff));
        let a = old_gpu.run(&inputs).unwrap();
        let b = new_gpu.run(&inputs).unwrap();
        let mut bad = 0usize;
        for (i, (oa, ob)) in a.iter().zip(&b).enumerate() {
            if oa[..4] != ob[..4] {
                bad += 1;
                if bad <= 5 {
                    eprintln!(
                        "{rel}: input {:?} — old {:?} != new {:?}",
                        inputs[i],
                        &oa[..4],
                        &ob[..4]
                    );
                }
            }
        }
        let mean_old: u64 = a.iter().map(|o| steps_of(o) as u64).sum::<u64>() / a.len() as u64;
        let mean_new: u64 = b.iter().map(|o| steps_of(o) as u64).sum::<u64>() / b.len() as u64;
        println!(
            "{rel}: {bad} value disagreements over {} inputs — mean steps {} -> {} ({}x)",
            inputs.len(),
            mean_old,
            mean_new,
            if mean_new > 0 {
                mean_old / mean_new.max(1)
            } else {
                0
            }
        );
        assert_eq!(bad, 0, "{rel}: rewrite is not value-identical");
    }
}
