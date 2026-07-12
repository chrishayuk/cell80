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

    use cell80_life::composition::{fingerprint_pool_member, grow_pool, ComposablePool};
    use cell80_life::ex2::{self, GenePools, RunConfig2DGenome, StartingGenome2};
    use cell80_life::genes::{CompiledGene, EngineKind};
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
            let decay_avg = rec
                .organisms
                .iter()
                .map(|o| o.decay_amount as f64)
                .sum::<f64>()
                / n;
            let thresh_avg = rec
                .organisms
                .iter()
                .map(|o| o.repro_threshold as f64)
                .sum::<f64>()
                / n;
            let give_avg = rec
                .organisms
                .iter()
                .map(|o| o.repro_give_pct as f64)
                .sum::<f64>()
                / n;
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

        // ── Part 2: operator (b) — composition-sweep receipts ──────────────────────────
        println!("\n== part 2: composition sweep (operator b) ==");
        let promoter_pool = ComposablePool::discover(&cells_dir(), &role_pools.promoters, 2);
        let movement_pool = ComposablePool::discover(&cells_dir(), &role_pools.movement, 3);
        println!(
            "composable (single-self-contained-function, no consts): {}/{} promoters, {}/{} movement",
            promoter_pool.funcs.len(),
            role_pools.promoters.len(),
            movement_pool.funcs.len(),
            role_pools.movement.len()
        );

        let promoter_fps: Vec<_> = role_pools
            .promoters
            .iter()
            .filter_map(|n| fingerprint_pool_member(&cells_dir(), n, 2))
            .collect();
        let movement_fps: Vec<_> = role_pools
            .movement
            .iter()
            .filter_map(|n| fingerprint_pool_member(&cells_dir(), n, 3))
            .collect();
        println!(
            "fingerprinted (for novelty comparison): {}/{} promoters, {}/{} movement — a pool \
             member with const data is excluded from both composability and this comparison, \
             the same stated limitation",
            promoter_fps.len(),
            role_pools.promoters.len(),
            movement_fps.len(),
            role_pools.movement.len()
        );

        let sweep_seed = 0x5eed_c0de_c0de_5eedu64;
        let sweep_attempts = 300u32;
        for (label, pool, fps) in [
            ("promoters (arity 2)", &promoter_pool, &promoter_fps),
            ("movement (arity 3)", &movement_pool, &movement_fps),
        ] {
            let report = grow_pool(pool, fps, sweep_seed, sweep_attempts);
            println!(
                "\n  {label}: {} attempts -> {} structurally invalid, {} not viable, {} duplicate, {} viable",
                report.attempts, report.structurally_invalid, report.not_viable, report.duplicate, report.viable.len()
            );
            if !report.viable.is_empty() {
                let closest: Vec<f32> = report
                    .viable
                    .iter()
                    .map(|c| {
                        fps.iter()
                            .map(|fp| c.fingerprint.agreement(fp))
                            .fold(0.0_f32, f32::max)
                    })
                    .collect();
                let avg_closest = closest.iter().sum::<f32>() / closest.len() as f32;
                let max_closest = closest.iter().cloned().fold(0.0_f32, f32::max);
                println!(
                    "    closest-existing-match agreement: avg={avg_closest:.3} max={max_closest:.3} (both < 1.0 by construction)"
                );
                for c in report.viable.iter().take(3) {
                    println!(
                        "    e.g. {}(..) wired into {}(.., slot {}, ..)",
                        c.f_name, c.g_name, c.slot
                    );
                }
            }
        }

        // ── Part 3: does the ecology ever exploit a composed candidate? ─────────────────
        println!("\n== part 3: ecology adoption — extended vs. control movement pool ==");
        let movement_growth = grow_pool(&movement_pool, &movement_fps, sweep_seed, sweep_attempts);
        if movement_growth.viable.is_empty() {
            println!("no viable composed movement candidates from this sweep — nothing to test adoption with.");
            return;
        }
        println!(
            "extending the movement pool with {} viable composed candidate(s), original size {}",
            movement_growth.viable.len(),
            role_pools.movement.len()
        );

        let base_movement_len = role_pools.movement.len();
        let build_genes = |extended: bool| -> GenePools {
            let mut g = GenePools::load(
                &cells_dir(),
                &starting.genes.decay,
                &starting.genes.eat,
                &starting.genes.split,
                &role_pools,
            )
            .expect("compiling gene pools");
            if extended {
                for c in &movement_growth.viable {
                    let name = format!("{}∘{}[slot{}]", c.f_name, c.g_name, c.slot);
                    let compiled = CompiledGene::from_funcs(&name, c.funcs.clone(), Vec::new())
                        .expect("compiling a composed candidate");
                    g.sense_pool.push(compiled);
                }
            }
            g
        };

        let control_genes = build_genes(false);
        let extended_genes = build_genes(true);
        let adoption_cfg = RunConfig2DGenome {
            seed: 0x5eed_1234_c311_80ff,
            ticks: 2000,
            initial_organisms: 8,
            world_width: 32,
            world_height: 32,
            food_density: 0.2,
            food_value: 40,
            regrow_ticks: 8,
        };

        let control_out = ex2::run(EngineKind::Gpu, &adoption_cfg, &starting2, &control_genes);
        let extended_out = ex2::run(EngineKind::Gpu, &adoption_cfg, &starting2, &extended_genes);

        println!(
            "control:  final_pop={} total_births={}",
            control_out.final_population, control_out.total_births
        );
        println!(
            "extended: final_pop={} total_births={}",
            extended_out.final_population, extended_out.total_births
        );
        println!(
            "NOTE: control vs. extended is NOT a clean isolate-one-variable comparison — a \
             larger pool changes which index every swap draw lands on from the very first \
             mutation event onward, so the two runs' populations diverge immediately. \
             Reported as a secondary, explicitly-caveated signal; the within-run comparison \
             below is the primary one."
        );

        let composed_births: Vec<_> = extended_out
            .births
            .iter()
            .filter(|b| b.sense_move as usize >= base_movement_len)
            .collect();
        println!(
            "\nextended run: {} / {} births carry a composed `sense_move` gene ({:.2}%)",
            composed_births.len(),
            extended_out.births.len(),
            100.0 * composed_births.len() as f64 / extended_out.births.len().max(1) as f64
        );

        if composed_births.is_empty() {
            println!("no adoption this run — composed candidates were available but never selected by a swap draw.");
        } else {
            // Within-run fitness proxy: direct children of composed-gene carriers vs.
            // disk-gene carriers — no pool-size confound, since both groups exist in the
            // same run under the same RNG stream.
            let child_count = |parent_id: u32| {
                extended_out
                    .births
                    .iter()
                    .filter(|b| b.parent_id == parent_id)
                    .count()
            };
            let composed_children: Vec<usize> = composed_births
                .iter()
                .map(|b| child_count(b.child_id))
                .collect();
            let disk_children: Vec<usize> = extended_out
                .births
                .iter()
                .filter(|b| (b.sense_move as usize) < base_movement_len)
                .map(|b| child_count(b.child_id))
                .collect();
            let avg = |v: &[usize]| v.iter().sum::<usize>() as f64 / v.len().max(1) as f64;
            println!(
                "avg direct children — composed-gene carriers: {:.3} (n={})  disk-gene carriers: {:.3} (n={})",
                avg(&composed_children),
                composed_children.len(),
                avg(&disk_children),
                disk_children.len()
            );
        }
    }
}
