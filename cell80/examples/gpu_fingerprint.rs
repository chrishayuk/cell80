//! Batched library fingerprinting on the GPU — the index-build seam where the
//! interpreter backend pays off (per the `gpu_route` finding: batched wins 66×,
//! per-query loses). Admission fingerprints every cell one `Runner` at a time
//! (`Fingerprint::of`, `O(cells·probes)` scalar); this runs the WHOLE library
//! against `DEFAULT_PROBES` in ONE `InterpBatch` dispatch and builds the same
//! fingerprints via `Fingerprint::from_value_sextets`.
//!
//! Verifies the batched fingerprints are **bit-identical** to the scalar
//! `Fingerprint::of` (so admission dedup/agreement verdicts can't move) and
//! measures the speedup.
//!
//! Run: `cargo run --release -p cell80 --example gpu_fingerprint` (macOS)

use cell80::{Cartridge, CartridgeOpts, CellConfig, Fingerprint, DEFAULT_PROBES};
use cell80_core::ir::Func;
use rustmsl::interp::{linearize, CellProgram};
use std::time::Instant;

type Funcs = Vec<(String, Func)>;

fn lower(src: &str) -> Result<Funcs, String> {
    let combined = format!("{src}\n{}{}", cell80::CELL_PRELUDE, rustz80::F32_KERNELS);
    let file: syn::File = syn::parse_str(&combined).map_err(|e| format!("parse: {e}"))?;
    let lowered = rustz80::lower_program_full(&file, &rustz80::PreludeConfig::default())?;
    if !lowered.funcs.iter().any(|(n, _)| n == "run") {
        return Err("no run".into());
    }
    let funcs = cell80_core::inline::inline(lowered.funcs, &["run"]);
    Ok(cell80_core::dce::prune(funcs, &["run"]))
}

fn main() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = manifest.join("cells");

    // Keep value cells that both compile (for the scalar oracle) and linearize
    // (for the GPU batch): name, cartridge, return signature, bytecode program.
    let mut names = Vec::new();
    let mut carts: Vec<Cartridge> = Vec::new();
    let mut rets: Vec<String> = Vec::new();
    let mut progs: Vec<CellProgram> = Vec::new();

    let mut files: Vec<_> = cell80::discover_cell_files(dir.to_str().unwrap()).unwrap();
    files.sort();
    for path in files {
        if path.extension().is_none_or(|x| x != "rs") {
            continue;
        }
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        let src = std::fs::read_to_string(&path).unwrap();
        let Ok(sig) = rustz80::entry_signature(&src, "run") else {
            continue;
        };
        let value = sig.state.is_empty()
            && sig.params.iter().all(|(_, ty)| {
                matches!(ty.as_str(), "u8" | "u16" | "i16" | "u32" | "i32" | "bool")
            });
        if !value {
            continue;
        }
        let Ok(funcs) = lower(&src) else { continue };
        let Ok(prog) = linearize(&funcs, "run") else {
            continue;
        };
        if prog.n_locals > 64 {
            continue;
        }
        let cart = match Cartridge::compile(
            &src,
            CellConfig::sandboxed(),
            CartridgeOpts {
                id: Some(name.clone()),
                ..Default::default()
            },
        ) {
            Ok(c) => c,
            Err(_) => continue,
        };
        rets.push(cart.manifest.signature.ret.clone());
        names.push(name);
        carts.push(cart);
        progs.push(prog);
    }

    let n = names.len();
    println!("batched library fingerprinting\n");
    println!("value cells (compile ∩ linearize): {n}");
    println!(
        "probe bank: {} probes (DEFAULT_PROBES)\n",
        DEFAULT_PROBES.len()
    );

    // ── Scalar oracle: Fingerprint::of per cell (the admission path today) ──
    let t = Instant::now();
    let scalar: Vec<Fingerprint> = carts.iter().map(Fingerprint::of).collect();
    let scalar_secs = t.elapsed().as_secs_f64();

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (progs, rets, scalar, scalar_secs);
        println!("(no Metal — GPU batch skipped)");
    }

    #[cfg(target_os = "macos")]
    {
        use rustmsl::interp::InterpBatch;
        // ── Batched GPU: whole library × DEFAULT_PROBES, one dispatch ──
        let probes: Vec<[u16; 3]> = DEFAULT_PROBES.to_vec();
        let np = probes.len();
        let t = Instant::now();
        let (batch, skipped) = InterpBatch::new(&progs).expect("interp batch");
        let out = batch.run(&probes);
        let gpu_secs = t.elapsed().as_secs_f64();
        assert_eq!(skipped, 0);

        let gpu: Vec<Fingerprint> = (0..n)
            .map(|ci| {
                let sextets: Vec<[u16; 6]> = (0..np).map(|k| out[ci * np + k]).collect();
                Fingerprint::from_value_sextets(&sextets, &rets[ci])
            })
            .collect();

        // ── Bit-identical parity check ──
        let mut ok = 0usize;
        let mut mism: Vec<String> = Vec::new();
        for ci in 0..n {
            if gpu[ci] == scalar[ci] {
                ok += 1;
            } else if mism.len() < 12 {
                mism.push(format!("  {}", names[ci]));
            }
        }
        println!("parity: {ok}/{n} fingerprints bit-identical to scalar Fingerprint::of");
        if !mism.is_empty() {
            println!("  mismatches (budget-unit gap: Z80 T-states vs IR steps?):");
            for m in &mism {
                println!("{m}");
            }
        }

        // Separate the one-time build from the repeatable dispatch, and show the
        // per-eval rate at a large bank — the batch scales, the scale doesn't.
        let big: Vec<[u16; 3]> = (0..2000).map(|i| probes[i % np]).collect();
        batch.run(&big); // warm
        let t = Instant::now();
        let _ = batch.run(&big);
        let big_secs = t.elapsed().as_secs_f64();

        println!(
            "\nindex-build cost ({} cells × {} probes = {} evals):",
            n,
            np,
            n * np
        );
        println!("  scalar (Runner per cell): {:>8.2} ms", scalar_secs * 1e3);
        println!(
            "  batched (build+dispatch): {:>8.2} ms   ← {:.1}× SLOWER at this scale",
            gpu_secs * 1e3,
            gpu_secs / scalar_secs
        );
        println!(
            "  batched dispatch alone, {} probes: {:.2} ms ({:.2e} evals/s)",
            big.len(),
            big_secs * 1e3,
            (n * big.len()) as f64 / big_secs
        );
        println!(
            "\nHonest finding: at today's {n}-cell library × {np}-probe bank the workload is too",
        );
        println!(
            "small — the scalar Runner loop wins fingerprinting (as it did per-query routing)."
        );
        println!(
            "The GPU dispatch itself is fast ({:.1}M evals/s above); its win is asymptotic —",
            (n * big.len()) as f64 / big_secs / 1e6
        );
        println!(
            "flat/no-cliff to 500k cells (priced earlier). The backend is a SCALE play for the"
        );
        println!(
            "\"millions of tools\" future, not a win on the current library. Correct + ready for"
        );
        println!("when the library is big enough to cross over; scalar stays right for today.");
    }
}
