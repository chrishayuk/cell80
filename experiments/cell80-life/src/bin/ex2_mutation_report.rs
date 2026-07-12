//! EX-2 operator (a) mutation report (`experiments/deterministic-ecology.md`). Not a
//! `#[test]` — reports numbers for the findings doc, matching `ex1_sweep.rs`'s "measure,
//! don't assume" convention: genome-diversity growth over ticks, per-tick
//! dispatch-count-per-role (the number of distinct pool indices among the living
//! population that tick — exactly the count `genes::batch_run_grouped` issues one GPU call
//! per, read straight off the retained tick history rather than instrumented separately),
//! and wall-clock at this pass's validated population scale.

#[cfg(not(target_os = "macos"))]
fn main() {
    println!(
        "ex2_mutation_report needs macOS (Metal) for the GPU engine path; the CPU-reference \
         engine works everywhere, but this binary specifically reports on the GPU-dispatched \
         run."
    );
}

#[cfg(target_os = "macos")]
fn main() {
    macos::main();
}

#[cfg(target_os = "macos")]
mod macos {
    use std::collections::HashSet;
    use std::path::{Path, PathBuf};
    use std::time::Instant;

    use cell80_life::ex2::{self, GenePools, RunConfig2DGenome, StartingGenome2};
    use cell80_life::genes::EngineKind;
    use cell80_life::load_starting_genome;
    use cell80_life::pools::discover_pools;

    fn cells_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cell80/cells")
    }

    fn genome_path(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("genomes/{name}.json"))
    }

    pub fn main() {
        let role_pools = discover_pools(&cells_dir());
        println!(
            "discovered pools: {} promoters, {} movement candidates",
            role_pools.promoters.len(),
            role_pools.movement.len()
        );

        let starting = load_starting_genome(&genome_path("grazer"));
        let starting2 = StartingGenome2 {
            initial_energy: starting.initial_energy,
            decay_amount: starting.decay_amount,
            repro_threshold: starting.repro_threshold,
            repro_give_pct: starting.repro_give_pct,
            hungry_promoter: role_pools.promoter_index(&starting.genes.hungry_promoter),
            repro_promoter: role_pools.promoter_index(&starting.genes.repro_promoter),
            sense_move: role_pools.movement_index(&starting.genes.sense_move),
        };
        let genes = GenePools::load(
            &cells_dir(),
            &starting.genes.decay,
            &starting.genes.eat,
            &starting.genes.split,
            &role_pools,
        )
        .expect("compiling gene pools");

        let cfg = RunConfig2DGenome {
            seed: 0x5eed_1234_c311_80ff,
            ticks: 2000,
            initial_organisms: 8,
            world_width: 32,
            world_height: 32,
            food_density: 0.2,
            food_value: 40,
            regrow_ticks: 8,
        };

        let t0 = Instant::now();
        let out = ex2::run(EngineKind::Gpu, &cfg, &starting2, &genes);
        let dt = t0.elapsed();

        println!(
            "\n{} ticks in {:.1} ms ({:.3} ms/tick), final_pop={}, total_births={}, total_starved={}",
            out.ticks.len(),
            dt.as_secs_f64() * 1e3,
            dt.as_secs_f64() * 1e3 / out.ticks.len().max(1) as f64,
            out.final_population,
            out.total_births,
            out.total_starved,
        );

        if out.ticks.is_empty() {
            return;
        }

        println!("\n== genome diversity + dispatch-count-per-role over ticks ==");
        let step = (out.ticks.len() / 10).max(1);
        for t in (0..out.ticks.len()).step_by(step) {
            let rec = &out.ticks[t];
            let hungry: HashSet<u16> = rec.organisms.iter().map(|o| o.hungry_promoter).collect();
            let repro: HashSet<u16> = rec.organisms.iter().map(|o| o.repro_promoter).collect();
            let sense: HashSet<u16> = rec.organisms.iter().map(|o| o.sense_move).collect();
            let n = rec.organisms.len().max(1) as f64;
            let decay_avg = rec.organisms.iter().map(|o| o.decay_amount as f64).sum::<f64>() / n;
            let thresh_avg = rec.organisms.iter().map(|o| o.repro_threshold as f64).sum::<f64>() / n;
            let give_avg = rec.organisms.iter().map(|o| o.repro_give_pct as f64).sum::<f64>() / n;
            println!(
                "tick {:>5}  n={:<4}  dispatch-count: hungry={:<3} repro={:<3} sense={:<3}  avg: decay={:.1} thresh={:.0} give={:.0}%",
                rec.tick,
                rec.organisms.len(),
                hungry.len(),
                repro.len(),
                sense.len(),
                decay_avg,
                thresh_avg,
                give_avg
            );
        }

        println!("\n== birth log summary ==");
        println!("total births logged: {}", out.births.len());
        let swaps = out
            .births
            .iter()
            .filter(|b| {
                b.hungry_promoter != starting2.hungry_promoter
                    || b.repro_promoter != starting2.repro_promoter
                    || b.sense_move != starting2.sense_move
            })
            .count();
        println!(
            "births with at least one role differing from the run's starting genome: {swaps} ({:.1}%)",
            100.0 * swaps as f64 / out.births.len().max(1) as f64
        );
    }
}
