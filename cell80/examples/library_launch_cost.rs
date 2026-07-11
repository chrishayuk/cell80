//! Price the library launch — kill-control #1 for retrieval-by-execution (WS-F).
//!
//! The megakernel (`compile_library`) runs the whole library against a probe
//! set in ONE dispatch. Everything downstream — retrieval-by-execution and
//! synthesis-by-execution both — rides on that launch being cheap. The
//! `gpu_cells` demo measures a single point (249 cells × 16 probes ≈ 48 ms on
//! an M-series part), which is ~83k evals/s: four orders below the streaming
//! headline. This example decomposes that number into the two costs that have
//! OPPOSITE strategic implications:
//!
//!   1. **Probe sweep** (full library, n_probes = 1..4096): fit dispatch time
//!      to `fixed + marginal·evals`. A large intercept that amortizes as you
//!      add probes means retrieval-by-execution works — you just need a fat
//!      enough probe batch. Marginal-dominated means it already works.
//!
//!   2. **Library-size sweep** (fixed probes, K = 1..N cells fused): per-eval
//!      cost as a function of how many cells share the kernel. FLAT per-eval =
//!      the megakernel scales to the "millions of tools" pitch. RISING per-eval
//!      = an occupancy/register wall that gets worse as the library grows —
//!      the scaling result that would gate the whole premise.
//!
//! Run: `cargo run --release -p cell80 --example library_launch_cost` (macOS)

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("library_launch_cost: rustmsl is macOS/Metal only");
}

#[cfg(target_os = "macos")]
fn main() {
    macos::run();
}

#[cfg(target_os = "macos")]
mod macos {
    use rustmsl::{compile_library, GpuBatch, LibraryCell};
    use std::time::Instant;

    type Funcs = Vec<(String, cell80_core::Func)>;
    type Consts = Vec<(String, Vec<u8>)>;

    /// Same lowering path as `gpu_cells`: prelude + f32 kernels, parse, lower,
    /// inline, DCE-root at `run`.
    fn lower(src: &str, entry: &str) -> Result<(Funcs, Consts), String> {
        let combined = format!("{src}\n{}{}", cell80::CELL_PRELUDE, rustz80::F32_KERNELS);
        let file: syn::File = syn::parse_str(&combined).map_err(|e| format!("parse: {e}"))?;
        let lowered = rustz80::lower_program_full(&file, &rustz80::PreludeConfig::default())?;
        if !lowered.funcs.iter().any(|(n, _)| n == entry) {
            return Err(format!("no `{entry}` entry"));
        }
        let consts = lowered.const_data();
        let funcs = cell80_core::inline::inline(lowered.funcs, &[entry]);
        let funcs = cell80_core::dce::prune(funcs, &[entry]);
        Ok((funcs, consts))
    }

    // Deterministic xorshift so probe sets are reproducible across runs.
    struct Rng(u32);
    impl Rng {
        fn next(&mut self) -> u16 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            self.0 = x;
            (x & 0xFFFF) as u16
        }
        fn probes(&mut self, n: usize) -> Vec<[u16; 3]> {
            (0..n).map(|_| [self.next(), self.next(), self.next()]).collect()
        }
    }

    /// Time one warm dispatch: warm a few times, then loop until `budget`
    /// seconds elapse and return mean seconds-per-dispatch.
    fn time_dispatch(batch: &GpuBatch, probes: &[[u16; 3]], budget: f64) -> f64 {
        for _ in 0..3 {
            batch.run(probes).unwrap();
        }
        let mut iters = 0u64;
        let t = Instant::now();
        let mut acc = 0u64; // keep the result live so nothing gets optimized away
        while t.elapsed().as_secs_f64() < budget {
            let out = batch.run(probes).unwrap();
            acc = acc.wrapping_add(out[out.len() / 2][0] as u64);
            iters += 1;
        }
        std::hint::black_box(acc);
        t.elapsed().as_secs_f64() / iters as f64
    }

    /// Ordinary-least-squares slope+intercept of y on x.
    fn fit(xs: &[f64], ys: &[f64]) -> (f64, f64) {
        let n = xs.len() as f64;
        let sx: f64 = xs.iter().sum();
        let sy: f64 = ys.iter().sum();
        let sxx: f64 = xs.iter().map(|x| x * x).sum();
        let sxy: f64 = xs.iter().zip(ys).map(|(x, y)| x * y).sum();
        let slope = (n * sxy - sx * sy) / (n * sxx - sx * sx);
        let intercept = (sy - slope * sx) / n;
        (slope, intercept)
    }

    pub fn run() {
        // Discover + lower + keep the MSL-compilable value cells (Act-3 filter).
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let dir = manifest.join("cells");
        let mut lowered: Vec<(String, Funcs, Consts)> = Vec::new();
        for path in cell80::discover_cell_files(dir.to_str().unwrap()).unwrap() {
            if path.extension().is_none_or(|x| x != "rs") {
                continue;
            }
            let name = path.file_stem().unwrap().to_string_lossy().into_owned();
            let src = std::fs::read_to_string(&path).unwrap();
            let Ok(sig) = rustz80::entry_signature(&src, "run") else {
                continue;
            };
            let scalar = sig.state.is_empty()
                && sig.params.iter().all(|(_, ty)| {
                    matches!(ty.as_str(), "u8" | "u16" | "i16" | "u32" | "i32" | "bool")
                });
            if !scalar {
                continue;
            }
            if let Ok((funcs, consts)) = lower(&src, "run") {
                lowered.push((name, funcs, consts));
            }
        }
        let cells: Vec<&(String, Funcs, Consts)> = lowered
            .iter()
            .filter(|(_, f, c)| rustmsl::compile(f, c, "run").is_ok())
            .collect();
        let n = cells.len();
        println!("library-launch cost — {n} MSL-compilable value cells\n");

        let mk_lib = |k: usize| -> Vec<LibraryCell> {
            cells[..k]
                .iter()
                .map(|(_, funcs, consts)| LibraryCell {
                    funcs,
                    consts,
                    entry: "run",
                    state_len: 0,
                })
                .collect()
        };

        // Build the full fused kernel once; report the one-time build cost.
        let t0 = Instant::now();
        let full = compile_library(&mk_lib(n)).unwrap();
        let batch = GpuBatch::new(&full).unwrap();
        let build = t0.elapsed().as_secs_f64();
        println!(
            "full kernel: {n} cells, {} KiB MSL, built in {:.2} s (one-time; amortized over every launch)\n",
            full.source.len() / 1024,
            build
        );

        // ── Sweep 1: probe count at full library ────────────────────────────
        println!("== probe sweep (all {n} cells fused, one dispatch) ==");
        println!(
            "  {:>7}  {:>7}  {:>11}  {:>13}  {:>13}",
            "probes", "evals", "dispatch", "evals/s", "ns/eval"
        );
        let mut rng = Rng(0x1234_5678);
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        for &p in &[1usize, 4, 16, 64, 256, 1024, 4096] {
            let probes = rng.probes(p);
            let secs = time_dispatch(&batch, &probes, 0.35);
            let evals = (n * p) as f64;
            xs.push(evals);
            ys.push(secs);
            println!(
                "  {:>7}  {:>7}  {:>9.3} ms  {:>13.3e}  {:>11.2}",
                p,
                n * p,
                secs * 1e3,
                evals / secs,
                secs / evals * 1e9
            );
        }
        let (slope, intercept) = fit(&xs, &ys);
        println!(
            "\n  least-squares fit:  dispatch ≈ {:.3} ms fixed  +  {:.2} ns/eval marginal",
            intercept * 1e3,
            slope * 1e9
        );
        println!(
            "  → asymptotic ceiling (marginal only): {:.3e} evals/s",
            1.0 / slope
        );
        let cross = if slope > 0.0 { intercept / slope } else { 0.0 };
        println!(
            "  → break-even (fixed = marginal) at ≈ {:.0} evals = {:.0} probes at this library size\n",
            cross,
            cross / n as f64
        );

        // ── Sweep 2: library size at fixed probe count ──────────────────────
        // Per-eval FLAT ⇒ megakernel scales; RISING ⇒ occupancy/register wall.
        const FIXED_PROBES: usize = 256;
        let probes = rng.probes(FIXED_PROBES);
        println!("== library-size sweep ({FIXED_PROBES} probes fixed; is per-eval flat?) ==");
        println!(
            "  {:>6}  {:>8}  {:>11}  {:>13}  {:>11}",
            "cells", "evals", "dispatch", "evals/s", "ns/eval"
        );
        let sizes: Vec<usize> = [1usize, 4, 16, 32, 64, 128, n]
            .into_iter()
            .filter(|&k| k <= n)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        for &k in &sizes {
            let module = compile_library(&mk_lib(k)).unwrap();
            let b = GpuBatch::new(&module).unwrap();
            let secs = time_dispatch(&b, &probes, 0.35);
            let evals = (k * FIXED_PROBES) as f64;
            println!(
                "  {:>6}  {:>8}  {:>9.3} ms  {:>13.3e}  {:>9.2}",
                k,
                k * FIXED_PROBES,
                secs * 1e3,
                evals / secs,
                secs / evals * 1e9
            );
        }
        println!(
            "\n  read: flat ns/eval ⇒ megakernel scales with library size;\n        rising ns/eval ⇒ occupancy wall (each thread pays for the whole switch)."
        );

        // ── Confound check: is the cliff about COUNT or about WHICH cells? ──
        // Re-run the size sweep over the REVERSED cell list. If the jump lands
        // at the same cell COUNT, it's a kernel-size cliff; if it moves, some
        // specific cells in the 64..128 prefix are just expensive.
        let rev: Vec<&(String, Funcs, Consts)> = cells.iter().rev().copied().collect();
        let mk_rev = |k: usize| -> Vec<LibraryCell> {
            rev[..k]
                .iter()
                .map(|(_, funcs, consts)| LibraryCell {
                    funcs,
                    consts,
                    entry: "run",
                    state_len: 0,
                })
                .collect()
        };
        println!("\n== confound: reversed-order size sweep ({FIXED_PROBES} probes) ==");
        println!("  {:>6}  {:>11}  {:>11}", "cells", "dispatch", "ns/eval");
        for &k in &sizes {
            let module = compile_library(&mk_rev(k)).unwrap();
            let b = GpuBatch::new(&module).unwrap();
            let secs = time_dispatch(&b, &probes, 0.35);
            let evals = (k * FIXED_PROBES) as f64;
            println!(
                "  {:>6}  {:>9.3} ms  {:>9.2}",
                k,
                secs * 1e3,
                secs / evals * 1e9
            );
        }
        println!("  cliff at the same COUNT as forward ⇒ kernel-size wall, not cell identity.");

        // ── The wall is a kernel-SWITCH cost, not a dispatch or sync cost ────
        // Tiling the library into sub-cliff (≤64-cell) kernels does NOT help —
        // it's worse — and the `same-kernel ×N` control below isolates why.
        //
        // A once-built path that encoded every tile into ONE command buffer
        // (one commit, one wait — a `TiledBatch` type, since reverted) was also
        // measured and made no difference: at 256 probes, 4×64-cell tiles ran
        //   monolithic          ~32 ms  (507 ns/eval)
        //   serial tiled/64     ~55 ms  (one cmd buffer per tile)
        //   1-cmdbuf tiled/64   ~52 ms  (all tiles, one command buffer)  ← no gain
        //   same-kernel ×4      ~1.3 ms (control: same tile 4×, no switch)
        // Removing the CPU↔GPU round trips changed nothing; dispatching the
        // SAME kernel 4× is ~40× cheaper than 4 DIFFERENT kernels. So the cost
        // is switching between distinct kernel binaries (PSO re-specialization /
        // residency churn — the exact mechanism unprofiled, but every candidate
        // is a function of per-kernel code size growing with the library). The
        // operative rule survives the mechanism uncertainty: kernel size must be
        // constant in library size (⇒ a fixed-size IR interpreter, not fusion).
        println!("\n== the wall is kernel-switch cost ({FIXED_PROBES} probes) ==");
        let mono = time_dispatch(&batch, &probes, 0.35);
        println!(
            "  {:>22}  {:>9.3} ms  {:>9.2} ns/eval",
            "monolithic (249)",
            mono * 1e3,
            mono / (n * FIXED_PROBES) as f64 * 1e9
        );
        for &tile in &[32usize, 64] {
            // Build per-tile fused modules (contiguous cell ranges).
            let mut modules = Vec::new();
            let mut start = 0;
            while start < n {
                let end = (start + tile).min(n);
                let lib: Vec<LibraryCell> = cells[start..end]
                    .iter()
                    .map(|(_, funcs, consts)| LibraryCell { funcs, consts, entry: "run", state_len: 0 })
                    .collect();
                modules.push(compile_library(&lib).unwrap());
                start = end;
            }
            let n_tiles = modules.len();
            let serial: Vec<GpuBatch> = modules.iter().map(|m| GpuBatch::new(m).unwrap()).collect();

            // Tiled: dispatch every distinct tile once per pass (kernel switches).
            for b in &serial {
                b.run(&probes).unwrap();
            }
            let tiled_secs = time_loop(0.35, |acc| {
                for b in &serial {
                    let out = b.run(&probes).unwrap();
                    *acc = acc.wrapping_add(out[out.len() / 2][0] as u64);
                }
            });

            // Control: dispatch the SAME tile n_tiles times (no kernel switch).
            let one = &serial[0];
            for _ in 0..3 {
                one.run(&probes).unwrap();
            }
            let same_secs = time_loop(0.35, |acc| {
                for _ in 0..n_tiles {
                    let out = one.run(&probes).unwrap();
                    *acc = acc.wrapping_add(out[out.len() / 2][0] as u64);
                }
            });

            println!(
                "  {:>22}  {:>9.3} ms  {:>9.2} ns/eval  ({} tiles, {:.1}× vs mono)",
                format!("tiled/{tile}"),
                tiled_secs * 1e3,
                tiled_secs / (n * FIXED_PROBES) as f64 * 1e9,
                n_tiles,
                mono / tiled_secs
            );
            println!(
                "  {:>22}  {:>9.3} ms  (control: same {}-cell tile ×{}, no switch — {:.0}× cheaper)",
                "same-kernel",
                same_secs * 1e3,
                tile.min(n),
                n_tiles,
                tiled_secs / same_secs
            );
        }
        println!(
            "\n  same-kernel ×N ≪ tiled/N ⇒ the wall is switching between distinct kernels,\n  not dispatch count or CPU↔GPU sync. Fix: one fixed-size kernel (IR interpreter)."
        );
    }

    /// Warm-free timing loop: run `body` (accumulating into a live u64 so the
    /// optimizer can't elide it) until `budget` seconds pass; return mean
    /// seconds per iteration.
    fn time_loop(budget: f64, mut body: impl FnMut(&mut u64)) -> f64 {
        let mut iters = 0u64;
        let mut acc = 0u64;
        let t = Instant::now();
        while t.elapsed().as_secs_f64() < budget {
            body(&mut acc);
            iters += 1;
        }
        std::hint::black_box(acc);
        t.elapsed().as_secs_f64() / iters as f64
    }
}
