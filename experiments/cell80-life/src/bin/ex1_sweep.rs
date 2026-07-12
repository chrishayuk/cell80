//! GPU-only exploratory/calibration sweep for EX-1 (`experiments/deterministic-ecology.md`).
//! Not a `#[test]` — this reports numbers for the findings doc, matching how
//! `cell80-life`/`cell-synth-evolve`/`evolved-cells` already report (binary output + a
//! companion `-findings.md`), since "does the population reach a steady/oscillatory
//! regime" is a result to report, not a boolean CI assertion.
//!
//! Two parts: (1) real per-tick GPU wall-clock at increasing population N, measured fresh
//! rather than assumed from docs/14's megakernel-launch number (a different code path);
//! (2) the calibration run — grazer vs rapid_reproducer through the (now contention-fixed)
//! 2D engine, computing real CV_tail/R_tail numbers rather than guessed thresholds.
//!
//! Both parts reuse `ex1::run` as-is (full per-tick `TickRecord2D` retention) rather than a
//! separate lightweight-summary path — deliberately: whether that retention is actually a
//! problem at the scales tested here is exactly what part 1 measures, not assumed up front.

#[cfg(not(target_os = "macos"))]
fn main() {
    println!(
        "ex1_sweep needs macOS (Metal) — the CPU-reference engine works everywhere, but \
         this binary specifically benchmarks/sweeps the GPU path."
    );
}

#[cfg(target_os = "macos")]
fn main() {
    macos::main();
}

#[cfg(target_os = "macos")]
mod macos {
    use std::path::{Path, PathBuf};
    use std::time::Instant;

    use cell80_life::ex1::{self, RunConfig2D};
    use cell80_life::genes::{EngineKind, GeneSet};
    use cell80_life::load_starting_genome;

    fn cells_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cell80/cells")
    }

    fn genome_path(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("genomes/{name}.json"))
    }

    /// Coefficient of variation and max/min ratio of a population time series — the
    /// numeric regime-classification signal (see the design/findings docs), not an
    /// eyeballed curve.
    fn cv_and_ratio(pops: &[u32]) -> (f64, f64) {
        let n = pops.len() as f64;
        let mean = pops.iter().map(|&p| p as f64).sum::<f64>() / n;
        let var = pops.iter().map(|&p| (p as f64 - mean).powi(2)).sum::<f64>() / n;
        let cv = if mean > 0.0 { var.sqrt() / mean } else { 0.0 };
        let max = *pops.iter().max().unwrap() as f64;
        let min = *pops.iter().min().unwrap() as f64;
        let ratio = if min > 0.0 { max / min } else { f64::INFINITY };
        (cv, ratio)
    }

    pub fn main() {
        let cells = cells_dir();
        let grazer = load_starting_genome(&genome_path("grazer"));
        let grazer_genes = GeneSet::load(&cells, &grazer.genes).expect("compiling grazer genes");

        println!("== part 1: per-tick GPU wall-clock vs population N (real, not assumed) ==");
        for &n in &[100usize, 1_000, 10_000, 100_000] {
            // ~6 tiles/organism on average — a rough, sparse-ish world for a wall-clock
            // scaling probe, not tuned for any particular regime.
            let side = ((n as f64 * 6.0).sqrt().ceil() as usize).max(8);
            let cfg = RunConfig2D {
                seed: 0x5eed_1234,
                ticks: 20,
                initial_organisms: n,
                world_width: side,
                world_height: side,
                food_density: 0.3,
                food_value: 40,
                regrow_ticks: 8,
            };
            let t0 = Instant::now();
            let out = ex1::run(EngineKind::Gpu, &cfg, &grazer, &grazer_genes);
            let dt = t0.elapsed();
            println!(
                "  N={n:<7} world={side}x{side:<4} 20 ticks in {:>9.1} ms  ({:>6.2} ms/tick)  final_pop={}",
                dt.as_secs_f64() * 1e3,
                dt.as_secs_f64() * 1e3 / 20.0,
                out.final_population
            );
        }

        println!("\n== part 2: calibration — grazer vs rapid_reproducer (CV_tail / R_tail) ==");
        println!("(swept across world size/density — the 1D baseline's tight 24-tile world");
        println!(" has far fewer escape routes than any 2D world at the same tile count,");
        println!(" so the regime split might only show up at a small/dense enough config)");
        let rapid = load_starting_genome(&genome_path("rapid_reproducer"));
        let rapid_genes =
            GeneSet::load(&cells, &rapid.genes).expect("compiling rapid_reproducer genes");

        let ticks = 1000u32;
        let tail = 500usize;
        let seeds = [1u64, 2, 3, 42, 999];
        // (side, food_density) — from close to the 1D baseline's tile count/density up to
        // the original, more generous first calibration point.
        let world_configs: [(usize, f64); 4] = [(8, 0.33), (8, 0.6), (12, 0.33), (24, 0.2)];

        for &(side, density) in &world_configs {
            println!("\n  -- world {side}x{side} @ density {density} --");
            for (label, genome, genes) in [
                ("grazer", &grazer, &grazer_genes),
                ("rapid_reproducer", &rapid, &rapid_genes),
            ] {
                for &seed in &seeds {
                    let cfg = RunConfig2D {
                        seed,
                        ticks,
                        initial_organisms: 8,
                        world_width: side,
                        world_height: side,
                        food_density: density,
                        food_value: 40,
                        regrow_ticks: 8,
                    };
                    let out = ex1::run(EngineKind::Gpu, &cfg, genome, genes);
                    if out.ticks.len() < tail {
                        println!(
                            "    {label:<18} seed={seed:<6} extinct at tick {} (before the {tail}-tick tail window)",
                            out.ticks.len()
                        );
                        continue;
                    }
                    let pops: Vec<u32> = out.ticks[out.ticks.len() - tail..]
                        .iter()
                        .map(|t| t.organisms.len() as u32)
                        .collect();
                    let (cv, ratio) = cv_and_ratio(&pops);
                    let osc = ex1::oscillator_rate(&out, 50);
                    println!(
                        "    {label:<18} seed={seed:<6} final_pop={:<5} CV_tail={cv:.3}  R_tail={ratio:.2}  oscillator_rate={osc:.3}",
                        out.final_population
                    );
                }
            }
        }

        // --- Part 3: isolate "axis-decomposition mechanism" from "genuine 2D-ness" ---
        // A height=1 world makes north/south both wrap to the same tile as "here", so
        // argmax3's own tie-break (ties -> lowest index -> "stay") makes the Y axis always
        // report "stay" — the engine is still running the *same* two-axis-decomposition
        // code path (still calls sense_move twice, still runs the priority-combination
        // logic), but with only one dimension of real movement, closely mimicking a 1D
        // ring. If boom-bust reappears here, the extra escape routes of true 2D (not the
        // axis-decomposition mechanism itself) explain the earlier collapse.
        println!("\n== part 3: height=1 ring (same engine, one real movement axis) ==");
        for &(width, density) in &[(24usize, 0.33), (8, 0.33)] {
            println!("\n  -- ring width={width} @ density {density} --");
            for (label, genome, genes) in [
                ("grazer", &grazer, &grazer_genes),
                ("rapid_reproducer", &rapid, &rapid_genes),
            ] {
                for &seed in &seeds {
                    let cfg = RunConfig2D {
                        seed,
                        ticks,
                        initial_organisms: 8,
                        world_width: width,
                        world_height: 1,
                        food_density: density,
                        food_value: 40,
                        regrow_ticks: 8,
                    };
                    let out = ex1::run(EngineKind::Gpu, &cfg, genome, genes);
                    if out.ticks.len() < tail {
                        println!(
                            "    {label:<18} seed={seed:<6} extinct at tick {} (before the {tail}-tick tail window)",
                            out.ticks.len()
                        );
                        continue;
                    }
                    let pops: Vec<u32> = out.ticks[out.ticks.len() - tail..]
                        .iter()
                        .map(|t| t.organisms.len() as u32)
                        .collect();
                    let (cv, ratio) = cv_and_ratio(&pops);
                    println!(
                        "    {label:<18} seed={seed:<6} final_pop={:<5} CV_tail={cv:.3}  R_tail={ratio:.2}",
                        out.final_population
                    );
                }
            }
        }

        // --- Part 4: does the regime split survive at 10^3-10^4 population? ---
        // The actual EX-1 gate: not just "does the mechanism exist at small n" (parts 2/3)
        // but "at 10^4-10^5 organisms." Uses the height=1 ring (where part 3 found the
        // split is real) scaled to host that many organisms — multiple organisms per tile
        // is allowed (no collision exclusion in this engine, matching ex0.rs), so a modest
        // ring width still works at large N.
        println!("\n== part 4: regime split at scale (height=1 ring, larger N) ==");
        let scale_ticks = 500u32;
        let scale_tail = 250usize;
        for &(width, initial_n) in &[(100usize, 1_000usize), (300, 10_000)] {
            println!("\n  -- ring width={width}, initial_organisms={initial_n} --");
            for (label, genome, genes) in [
                ("grazer", &grazer, &grazer_genes),
                ("rapid_reproducer", &rapid, &rapid_genes),
            ] {
                for &seed in &[1u64, 2] {
                    let cfg = RunConfig2D {
                        seed,
                        ticks: scale_ticks,
                        initial_organisms: initial_n,
                        world_width: width,
                        world_height: 1,
                        food_density: 0.33,
                        food_value: 40,
                        regrow_ticks: 8,
                    };
                    let t0 = Instant::now();
                    let out = ex1::run(EngineKind::Gpu, &cfg, genome, genes);
                    let dt = t0.elapsed();
                    if out.ticks.len() < scale_tail {
                        println!(
                            "    {label:<18} seed={seed:<6} extinct at tick {} (before the {scale_tail}-tick tail window), {:.0} ms total",
                            out.ticks.len(), dt.as_secs_f64() * 1e3
                        );
                        continue;
                    }
                    let pops: Vec<u32> = out.ticks[out.ticks.len() - scale_tail..]
                        .iter()
                        .map(|t| t.organisms.len() as u32)
                        .collect();
                    let (cv, ratio) = cv_and_ratio(&pops);
                    println!(
                        "    {label:<18} seed={seed:<6} final_pop={:<7} CV_tail={cv:.3}  R_tail={ratio:.2}  ({:.0} ms total, {:.2} ms/tick)",
                        out.final_population, dt.as_secs_f64() * 1e3, dt.as_secs_f64() * 1e3 / scale_ticks as f64
                    );
                }
            }
        }
    }
}
