//! The E1 library battery (Phase 6 WS-E, docs 14): every straight-line integer
//! **value cell** in the library, compiled to MSL and run on the system's Metal
//! device against the reference interpreter — the full `[r0, r1, r2, status]`
//! quad must agree bit for bit on every input, or the cell is a filed defect.
//!
//! Coverage is honest, not silent (docs 14 "no silent caps"): cells outside the
//! E1 fragment refuse at codegen with a typed reason and are *counted* — loops
//! are E2, f32 is E4, state cells await typed-state readback on the GPU host
//! path (owed with E3). The default run keeps CI fast; the pre-registered E1
//! gate (10⁶ random inputs per cell) runs via:
//!
//! ```sh
//! CELL80_MSL_FUZZ_N=1000000 cargo test --release -p cell80 --test msl_battery \
//!     -- --ignored --nocapture
//! ```

#![cfg(target_os = "macos")]

use cell80_core::{Interp, Target};
use rustmsl::{GpuBatch, STATUS_DIV0, STATUS_HALT, STATUS_OK};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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

type Funcs = Vec<(String, cell80_core::Func)>;
type Consts = Vec<(String, Vec<u8>)>;

/// The cartridge pipeline up to the IR seam (`compile_rv32`'s steps, stopping
/// where the per-target body compiler takes over): prelude append, lower,
/// inline, DCE-root at `run`.
fn lower_cell(src: &str) -> Result<(Funcs, Consts), String> {
    let combined = format!("{src}\n{}{}", cell80::CELL_PRELUDE, rustz80::F32_KERNELS);
    let file: syn::File = syn::parse_str(&combined).map_err(|e| format!("parse: {e}"))?;
    let lowered = rustz80::lower_program_full(&file, &rustz80::PreludeConfig::default())?;
    if !lowered.funcs.iter().any(|(n, _)| n == "run") {
        return Err("no free `run` entry (state cell)".into());
    }
    let consts = lowered.const_data();
    let funcs = cell80_core::inline::inline(lowered.funcs, &["run"]);
    let funcs = cell80_core::dce::prune(funcs, &["run"]);
    Ok((funcs, consts))
}

/// A value cell's entry takes only scalar params — a pointer param driven with
/// a random u16 would write through wild addresses, which is the state-cell
/// harness's job (owed with E3), not this battery's.
fn scalar_signature(src: &str) -> bool {
    match rustz80::entry_signature(src, "run") {
        Ok(sig) => {
            sig.state.is_empty()
                && sig.params.iter().all(|(_, ty)| {
                    matches!(ty.as_str(), "u8" | "u16" | "i16" | "u32" | "i32" | "bool")
                })
        }
        Err(_) => false,
    }
}

/// What the interpreter said, folded to the GPU's output quad shape.
fn interp_quad(res: Result<Vec<u16>, String>) -> Result<[u16; 4], String> {
    match res {
        Ok(v) => Ok([
            v.first().copied().unwrap_or(0),
            v.get(1).copied().unwrap_or(0),
            v.get(2).copied().unwrap_or(0),
            STATUS_OK,
        ]),
        Err(e) if e.contains("divide by zero") => Ok([0, 0, 0, STATUS_DIV0]),
        Err(e) => e
            .strip_prefix("interp: halt(")
            .and_then(|s| s.strip_suffix(')'))
            .and_then(|s| s.parse::<u16>().ok())
            .map(|code| [code, 0, 0, STATUS_HALT])
            .ok_or(e),
    }
}

fn cell_paths() -> Vec<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut paths: Vec<PathBuf> =
        cell80::discover_cell_files(manifest.join("cells").to_str().unwrap())
            .unwrap_or_else(|e| panic!("{e}"))
            .into_iter()
            .filter(|p| p.extension().is_some_and(|x| x == "rs"))
            .collect();
    paths.sort();
    paths
}

/// One cell's battery: `n` seeded-random triples (plus a corner sweep), one GPU
/// dispatch, interpreter re-created per block so fuel never accumulates.
/// Returns the number of disagreeing inputs (0 is the only passing answer).
fn run_cell(
    name: &str,
    funcs: &Funcs,
    consts: &[(String, Vec<u8>)],
    module: &rustmsl::MslModule,
    n: usize,
    seed: u64,
) -> usize {
    let corners: &[u16] = &[0, 1, 2, 0x7F, 0x80, 0xFF, 0x100, 0x7FFF, 0x8000, 0xFFFF];
    let mut inputs = Vec::with_capacity(n + corners.len() * corners.len());
    for &a in corners {
        for &b in corners {
            inputs.push([a, b, 1]);
        }
    }
    let mut rng = Rng(seed);
    for _ in 0..n {
        inputs.push([rng.u16(), rng.u16(), rng.u16()]);
    }
    let gpu = GpuBatch::new(module)
        .unwrap_or_else(|e| panic!("{name}: gpu pipeline failed: {e}\n{}", module.source));
    let got = gpu.run(&inputs).unwrap_or_else(|e| panic!("{name}: {e}"));

    let n_args = module.params;
    let mut mismatches = 0;
    // Fresh interpreter per block: cheap enough, and 100M fuel per block can
    // never run dry mid-battery. Within a block, reset memory to pristine.
    const BLOCK: usize = 512;
    for (block, quads) in inputs.chunks(BLOCK).zip(got.chunks(BLOCK)) {
        let mut interp = Interp::new(
            funcs,
            consts.iter().map(|(n, b)| (n.as_str(), b.as_slice())),
            Target::Cell.descriptor(),
        );
        let pristine = interp.mem.clone();
        for (args, gpu_quad) in block.iter().zip(quads) {
            interp.mem.copy_from_slice(&pristine);
            let want = interp_quad(interp.run("run", &args[..n_args]))
                .unwrap_or_else(|e| panic!("{name}: unexpected interpreter refusal: {e}"));
            if *gpu_quad != want {
                mismatches += 1;
                if mismatches <= 5 {
                    eprintln!("{name}: args {args:?} — gpu {gpu_quad:?} != interpreter {want:?}");
                }
            }
        }
    }
    mismatches
}

/// Sweep the library: compile every eligible cell to MSL, run the battery, and
/// report coverage + refusals. `n` random inputs per cell.
fn battery(n: usize) {
    let mut compiled = 0usize;
    let mut clean = 0usize;
    let mut defects: Vec<String> = Vec::new();
    let mut refusals: BTreeMap<String, usize> = BTreeMap::new();
    let mut skipped_state = 0usize;
    let mut skipped_sig = 0usize;
    let paths = cell_paths();
    for path in &paths {
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        let src = std::fs::read_to_string(path).unwrap();
        if !scalar_signature(&src) {
            // State cells (impl `run`, or named state fields) vs value cells
            // with pointer params — both await the typed-state GPU harness.
            if src.contains("impl ") {
                skipped_state += 1;
            } else {
                skipped_sig += 1;
            }
            continue;
        }
        let (funcs, consts) = match lower_cell(&src) {
            Ok(v) => v,
            Err(e) if e.contains("state cell") => {
                skipped_state += 1;
                continue;
            }
            Err(e) => panic!("{name}: lower failed: {e}"),
        };
        match rustmsl::compile(&funcs, &consts, "run") {
            Ok(module) => {
                compiled += 1;
                // Per-cell seed: stable across runs, distinct across cells.
                let seed = 0x5eed_e100_0000_0000 ^ compiled as u64;
                let bad = run_cell(&name, &funcs, &consts, &module, n, seed);
                if bad == 0 {
                    clean += 1;
                } else {
                    defects.push(format!("{name}: {bad} disagreeing inputs"));
                }
            }
            Err(e) => {
                // Typed refusals, bucketed by reason — coverage stays honest.
                let key = if e.contains("E2") {
                    "loops (E2)".to_string()
                } else if e.contains("f32") || e.contains("E4") {
                    "f32 (E4)".to_string()
                } else {
                    e
                };
                *refusals.entry(key).or_default() += 1;
            }
        }
    }
    println!(
        "msl E1 battery: {} cells — {compiled} compiled ({clean} clean), \
         {skipped_sig} non-scalar/no-entry, {skipped_state} state, refusals: {:?}",
        paths.len(),
        refusals
    );
    assert!(
        defects.is_empty(),
        "GPU ≠ interpreter on {} cells:\n{}",
        defects.len(),
        defects.join("\n")
    );
    // A floor so a silent regression (everything refusing) can't read as green.
    assert!(
        compiled >= 170,
        "only {compiled} cells reached the GPU — the E1 fragment shrank (173 at E1)"
    );
}

/// The CI-speed battery: every eligible cell, a corner sweep + 512 random
/// inputs each. The full pre-registered gate is [`e1_gate_one_million`].
#[test]
fn e1_battery() {
    let n = std::env::var("CELL80_MSL_FUZZ_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(512);
    battery(n);
}

/// The E1 gate (docs 14): 10⁶ random inputs per admitted straight-line integer
/// cell, bit-exact. Ignored by default — minutes of wall clock; run in release.
#[test]
#[ignore = "the 10^6-input E1 gate — run explicitly in release"]
fn e1_gate_one_million() {
    battery(1_000_000);
}
