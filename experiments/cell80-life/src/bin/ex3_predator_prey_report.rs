//! EX-3 predator/prey calibration + mutation-off control (`experiments/deterministic-ecology.md`).
//! Not a `#[test]` — reports numbers for the findings doc, matching `ex1_sweep.rs`'s "measure,
//! don't assume" convention.
//!
//! Part 1: world-size/ratio/density calibration sweep. Per-species tail-window CV/ratio
//! (`ex1_sweep.rs`'s own regime-classification signal, applied separately to each species
//! since a one-sided collapse in either direction is a different failure mode than a shared
//! stable oscillation) plus the predation-kill count, across 5 seeds per config — looking for
//! a configuration where both species persist to the end, not just one where the numbers look
//! plausible on a single seed.
//!
//! Part 2: the pre-registered mutation-off control (design decision 8) at the calibrated
//! config — isolates "more room" (world size/population ratio) from "evolution" as the
//! explanation for any population stability observed. If world-size/ratio levers alone
//! (mutation off) already stabilize the population, that's the clean, reportable answer to
//! the control, and Checkpoint B can decide the satiation mechanic is unnecessary without
//! building it.

#[cfg(not(target_os = "macos"))]
fn main() {
    println!(
        "ex3_predator_prey_report needs macOS (Metal) for the GPU engine path; the \
         CPU-reference engine works everywhere, but this binary specifically reports on the \
         GPU-dispatched run."
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

    use cell80_life::ex2::GenePools;
    use cell80_life::ex3::{self, RunConfig3, RunOutput3, StartingGenome3};
    use cell80_life::genes::EngineKind;
    use cell80_life::history::Species;
    use cell80_life::load_starting_genome;
    use cell80_life::pools::{discover_pools, Pools};

    fn cells_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cell80/cells")
    }

    fn genome_path(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("genomes/{name}.json"))
    }

    fn load_starting3(name: &str, species: Species, role_pools: &Pools) -> StartingGenome3 {
        let starting = load_starting_genome(&genome_path(name));
        StartingGenome3 {
            species,
            initial_energy: starting.initial_energy,
            decay_amount: starting.decay_amount,
            repro_threshold: starting.repro_threshold,
            repro_give_pct: starting.repro_give_pct,
            hungry_promoter: role_pools.promoter_index(&starting.genes.hungry_promoter),
            repro_promoter: role_pools.promoter_index(&starting.genes.repro_promoter),
            sense_move: role_pools.movement_index(&starting.genes.sense_move),
        }
    }

    /// Coefficient of variation and max/min ratio of a population time series —
    /// `ex1_sweep.rs`'s own numeric regime-classification signal.
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

    fn species_pops(out: &RunOutput3, tail: usize, species: Species) -> Option<Vec<u32>> {
        if out.ticks.len() < tail {
            return None;
        }
        Some(
            out.ticks[out.ticks.len() - tail..]
                .iter()
                .map(|t| t.organisms.iter().filter(|o| o.species == species).count() as u32)
                .collect(),
        )
    }

    struct WorldConfig {
        width: usize,
        height: usize,
        grazers: usize,
        predators: usize,
        density: f64,
    }

    fn report_run(seed: u64, out: &RunOutput3, tail: usize) {
        let grazer_pops = species_pops(out, tail, Species::Grazer);
        let predator_pops = species_pops(out, tail, Species::Predator);
        match (grazer_pops, predator_pops) {
            (Some(g), Some(p)) => {
                let (gcv, gratio) = cv_and_ratio(&g);
                let (pcv, pratio) = cv_and_ratio(&p);
                println!(
                    "    seed={seed:<6} ticks={:<5} final: grazers={:<5} predators={:<5} kills={:<6} | \
                     grazer CV={gcv:.3} R={gratio:.2} | predator CV={pcv:.3} R={pratio:.2}",
                    out.ticks.len(), out.final_grazers, out.final_predators, out.total_predation_kills
                );
            }
            _ => {
                println!(
                    "    seed={seed:<6} ended at tick {} (before the {tail}-tick tail window) \
                     final: grazers={} predators={} kills={}",
                    out.ticks.len(),
                    out.final_grazers,
                    out.final_predators,
                    out.total_predation_kills
                );
            }
        }
    }

    pub fn main() {
        let role_pools = discover_pools(&cells_dir());
        let grazer = load_starting3("grazer", Species::Grazer, &role_pools);
        let predator = load_starting3("predator", Species::Predator, &role_pools);
        let grazer_disk = load_starting_genome(&genome_path("grazer"));
        let genes = GenePools::load(
            &cells_dir(),
            &grazer_disk.genes.decay,
            &grazer_disk.genes.eat,
            &grazer_disk.genes.split,
            &role_pools,
        )
        .expect("compiling gene pools");

        let ticks = 3000u32;
        let tail = 500usize;
        let seeds = [1u64, 2, 3, 42, 999];

        if std::env::var("SKIP_PART1").is_err() {
            println!("== part 1: world-size/ratio/density calibration sweep ==");
            let configs = [
                WorldConfig {
                    width: 32,
                    height: 32,
                    grazers: 30,
                    predators: 6,
                    density: 0.25,
                },
                WorldConfig {
                    width: 48,
                    height: 48,
                    grazers: 40,
                    predators: 8,
                    density: 0.25,
                },
                WorldConfig {
                    width: 48,
                    height: 48,
                    grazers: 60,
                    predators: 10,
                    density: 0.3,
                },
                WorldConfig {
                    width: 64,
                    height: 64,
                    grazers: 60,
                    predators: 10,
                    density: 0.25,
                },
            ];

            let t0 = Instant::now();
            for c in &configs {
                println!(
                    "\n  -- world {}x{} grazers={} predators={} density={} --",
                    c.width, c.height, c.grazers, c.predators, c.density
                );
                for &seed in &seeds {
                    let cfg = RunConfig3 {
                        seed,
                        ticks,
                        initial_grazers: c.grazers,
                        initial_predators: c.predators,
                        world_width: c.width,
                        world_height: c.height,
                        food_density: c.density,
                        food_value: 40,
                        regrow_ticks: 8,
                        mutation_enabled: true,
                        predator_satiation_ticks: 0,
                    };
                    let out = ex3::run(EngineKind::Gpu, &cfg, &grazer, &predator, &genes);
                    report_run(seed, &out, tail);
                }
            }
            println!("\n(part 1 wall-clock: {:.1}s)", t0.elapsed().as_secs_f64());
        } else {
            println!("== part 1 skipped (SKIP_PART1 set) — see prior run's output ==");
        }

        let calibrated_configs = [
            WorldConfig {
                width: 48,
                height: 48,
                grazers: 60,
                predators: 10,
                density: 0.3,
            },
            WorldConfig {
                width: 64,
                height: 64,
                grazers: 60,
                predators: 10,
                density: 0.25,
            },
        ];

        if std::env::var("SKIP_PART2").is_err() {
            // ── Part 2: the pre-registered mutation-off control ─────────────────────────
            // Swept across the two configs part 1 found fully robust (5/5 seeds sustained
            // coexistence with mutation on) — not just the weaker 40:8 config — so a
            // mutation-off collapse can't be dismissed as "that config just wasn't generous
            // enough." No satiation cooldown here (`predator_satiation_ticks: 0`) — this is
            // the pre-satiation baseline; see part 3 for the mechanic itself.
            println!("\n== part 2: mutation-off control (design decision 8) ==");
            let t0 = Instant::now();
            for c in &calibrated_configs {
                println!(
                    "\n  -- world {}x{} grazers={} predators={} density={} --",
                    c.width, c.height, c.grazers, c.predators, c.density
                );
                for &mutation_enabled in &[true, false] {
                    println!("\n    -- mutation_enabled={mutation_enabled} --");
                    for &seed in &seeds {
                        let cfg = RunConfig3 {
                            seed,
                            ticks,
                            initial_grazers: c.grazers,
                            initial_predators: c.predators,
                            world_width: c.width,
                            world_height: c.height,
                            food_density: c.density,
                            food_value: 40,
                            regrow_ticks: 8,
                            mutation_enabled,
                            predator_satiation_ticks: 0,
                        };
                        let out = ex3::run(EngineKind::Gpu, &cfg, &grazer, &predator, &genes);
                        report_run(seed, &out, tail);
                    }
                }
            }
            println!("\n(part 2 wall-clock: {:.1}s)", t0.elapsed().as_secs_f64());
        } else {
            println!("== part 2 skipped (SKIP_PART2 set) — see prior run's output ==");
        }

        // ── Part 3: the satiation mechanic (Checkpoint B, built per explicit decision) ──
        // Does a predator kill-cooldown rescue the mutation-off case, or change the
        // mutation-on case? Tested at the same two robust configs, both mutation states.
        println!("\n== part 3: satiation mechanic (predator_satiation_ticks=20) ==");
        let t0 = Instant::now();
        for c in &calibrated_configs {
            println!(
                "\n  -- world {}x{} grazers={} predators={} density={} --",
                c.width, c.height, c.grazers, c.predators, c.density
            );
            for &mutation_enabled in &[true, false] {
                println!("\n    -- mutation_enabled={mutation_enabled} --");
                for &seed in &seeds {
                    let cfg = RunConfig3 {
                        seed,
                        ticks,
                        initial_grazers: c.grazers,
                        initial_predators: c.predators,
                        world_width: c.width,
                        world_height: c.height,
                        food_density: c.density,
                        food_value: 40,
                        regrow_ticks: 8,
                        mutation_enabled,
                        predator_satiation_ticks: 20,
                    };
                    let out = ex3::run(EngineKind::Gpu, &cfg, &grazer, &predator, &genes);
                    report_run(seed, &out, tail);
                }
            }
        }
        println!("\n(part 3 wall-clock: {:.1}s)", t0.elapsed().as_secs_f64());
    }
}
