//! EX-6 degraded-seed report (`experiments/deterministic-ecology-ex6-preregistration.md`).
//! Not a `#[test]` — reports numbers for the findings doc, matching the convention of
//! `ex2_mutation_report.rs` / `ex4_lineage_report.rs`.
//!
//! The question the doc-17 programme never asked: the ecology was seeded with hand-curated
//! cells, so was it started *at* a local optimum with nowhere to climb? Seed it below the
//! optimum and see whether selection climbs back.
//!
//! Primary axis is `decay_amount`, because it is the one clean case in the genome: monotone
//! (less decay is strictly better), its optimum sits at the bound of `DECAY_BOUNDS` (1, 6),
//! and 6 -> 1 is five single steps under an operator that draws {-1, 0, +1}. If selection
//! cannot climb that, no weaker ratchet claim survives.
//!
//! The metric is the *genome* (mean `decay_amount` across the final living population), not
//! population size — population confounds "the genome improved" with "this run's world was
//! easier". The bar is against a lineage-depth-matched clamped-random-walk null, never
//! against the starting value: an unselected walk on [1, 6] drifts toward ~3.5 on its own,
//! so reading 3.5 as recovery would be exactly the wrong-sign error this programme has
//! already made once.

#[cfg(not(target_os = "macos"))]
fn main() {
    println!(
        "ex6_degraded_seed_report needs macOS (Metal) for the GPU engine path; the \
         CPU-reference engine works everywhere, but this binary specifically reports on the \
         GPU-dispatched run (EX-0 established GPU == interpreter byte-for-byte)."
    );
}

#[cfg(target_os = "macos")]
fn main() {
    macos::main();
}

#[cfg(target_os = "macos")]
mod macos {
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::time::Instant;

    use cell80_life::ex2::{
        self, GenePools, MutationMode, RunConfig2DGenome, RunOutput2DGenome, StartingGenome2,
    };
    use cell80_life::genes::EngineKind;
    use cell80_life::history::BirthEvent;
    use cell80_life::load_starting_genome;
    use cell80_life::pools::discover_pools;

    /// Mirrors `ex2.rs`'s own `DECAY_BOUNDS` / `NUMERIC_MUTATE_PCT`. Duplicated here rather
    /// than exported because the null must model the operator *as pre-registered*; if the
    /// operator ever changes, this null should fail loudly rather than silently track it.
    const DECAY_BOUNDS: (i32, i32) = (1, 6);
    const NUMERIC_MUTATE_PCT: u64 = 25;
    const NULL_REPLICATES: usize = 10_000;
    const NULL_SEED: u64 = 0x6060_ec06_d31f_7a11;

    const BASE_SEED: u64 = 0x5eed_1234_c311_80ff;
    const SEEDS: usize = 5;
    const TICKS: u32 = 2000;
    const INITIAL_ORGANISMS: usize = 8;

    fn cells_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cell80/cells")
    }

    struct Rung {
        name: &'static str,
        decay: u16,
        threshold: Option<u16>,
        /// Replace `sense_move` with this named cell (D4's categorical degradation).
        sense_cell: Option<&'static str>,
    }

    const RUNGS: &[Rung] = &[
        Rung {
            name: "D0 curated",
            decay: 1,
            threshold: None,
            sense_cell: None,
        },
        Rung {
            name: "D1 decay=3",
            decay: 3,
            threshold: None,
            sense_cell: None,
        },
        Rung {
            name: "D2 decay=6",
            decay: 6,
            threshold: None,
            sense_cell: None,
        },
        Rung {
            name: "D3 decay=6,thr=400",
            decay: 6,
            threshold: Some(400),
            sense_cell: None,
        },
        Rung {
            name: "D4 argmin3",
            decay: 1,
            threshold: None,
            sense_cell: Some("argmin3"),
        },
    ];

    const MODES: &[(&str, MutationMode)] = &[
        ("off", MutationMode::Off),
        ("numeric", MutationMode::NumericOnly),
        ("full", MutationMode::Full),
    ];

    /// Deterministic PRNG for the null's Monte Carlo — fixed seed, no external crate, so the
    /// null is as replayable as the world it is a null for.
    struct Lcg(u64);
    impl Lcg {
        fn next_u64(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            self.0 ^ (self.0 >> 31)
        }
        fn below(&mut self, n: u64) -> u64 {
            self.next_u64() % n
        }
    }

    /// Depth of every organism in the realized lineage tree. Founders (the initial
    /// organisms, which take the first `n0` ids) are depth 0; every other organism is one
    /// deeper than its parent. Iterative, not recursive — lineages run thousands deep.
    fn depths(births: &[BirthEvent], n0: usize) -> HashMap<u32, u32> {
        let parent: HashMap<u32, u32> = births.iter().map(|b| (b.child_id, b.parent_id)).collect();
        let mut memo: HashMap<u32, u32> = HashMap::new();
        for id in 0..n0 as u32 {
            memo.insert(id, 0);
        }
        for b in births {
            // Walk up to the nearest known ancestor, then unwind assigning depths.
            let mut chain: Vec<u32> = Vec::new();
            let mut cur = b.child_id;
            while !memo.contains_key(&cur) {
                chain.push(cur);
                match parent.get(&cur) {
                    Some(&p) => cur = p,
                    // Unreachable for a well-formed birth log; treat as a root rather than
                    // panicking inside a reporting binary.
                    None => {
                        memo.insert(cur, 0);
                        break;
                    }
                }
            }
            let mut d = memo[&cur];
            for &id in chain.iter().rev() {
                d += 1;
                memo.insert(id, d);
            }
        }
        memo
    }

    /// One clamped random walk of `steps` mutation opportunities from `start`, modelling the
    /// numeric operator with selection removed: each opportunity fires with probability
    /// `NUMERIC_MUTATE_PCT`, and when it fires the step is uniform over {-1, 0, +1}.
    fn drift_walk(rng: &mut Lcg, start: u16, steps: u32) -> u16 {
        let mut v = start as i32;
        for _ in 0..steps {
            if rng.below(100) < NUMERIC_MUTATE_PCT {
                let step = rng.below(3) as i32 - 1;
                v = (v + step).clamp(DECAY_BOUNDS.0, DECAY_BOUNDS.1);
            }
        }
        v as u16
    }

    /// Monte-Carlo the null mean and its 95% interval over the *observed* depth
    /// distribution, so the null is matched to how much mutational opportunity this
    /// population actually had rather than to a nominal generation count.
    fn drift_null(start: u16, survivor_depths: &[u32]) -> (f64, f64, f64) {
        if survivor_depths.is_empty() {
            return (f64::NAN, f64::NAN, f64::NAN);
        }
        let mut rng = Lcg(NULL_SEED);
        let mut means: Vec<f64> = Vec::with_capacity(NULL_REPLICATES);
        for _ in 0..NULL_REPLICATES {
            let sum: u32 = survivor_depths
                .iter()
                .map(|&d| drift_walk(&mut rng, start, d) as u32)
                .sum();
            means.push(sum as f64 / survivor_depths.len() as f64);
        }
        means.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mean = means.iter().sum::<f64>() / means.len() as f64;
        let lo = means[(NULL_REPLICATES as f64 * 0.025) as usize];
        let hi = means[(NULL_REPLICATES as f64 * 0.975) as usize];
        (mean, lo, hi)
    }

    struct Outcome {
        extinct: bool,
        survivors: usize,
        mean_decay: f64,
        null_mean: f64,
        null_lo: f64,
        recovery: f64,
        below_null: bool,
        argmax_frac: f64,
        births: u32,
        starved: u32,
        mean_depth: f64,
        /// Modal role-cell index in the final population, with its share — the diagnostic
        /// that separates "swaps are deleterious noise" from "swaps unlock a degenerate
        /// role cell that games the tick loop".
        modal_repro: (u16, f64),
        modal_hungry: (u16, f64),
        modal_sense: (u16, f64),
    }

    fn modal(values: impl Iterator<Item = u16>) -> (u16, f64) {
        let vals: Vec<u16> = values.collect();
        if vals.is_empty() {
            return (u16::MAX, f64::NAN);
        }
        let mut counts: HashMap<u16, usize> = HashMap::new();
        for v in &vals {
            *counts.entry(*v).or_default() += 1;
        }
        let (&idx, &n) = counts.iter().max_by_key(|(_, &n)| n).unwrap();
        (idx, n as f64 / vals.len() as f64)
    }

    fn analyse(out: &RunOutput2DGenome, start_decay: u16, argmax_idx: u16) -> Outcome {
        let final_tick = out.ticks.last();
        let orgs = final_tick.map(|t| t.organisms.as_slice()).unwrap_or(&[]);
        if orgs.is_empty() {
            return Outcome {
                extinct: true,
                survivors: 0,
                mean_decay: f64::NAN,
                null_mean: f64::NAN,
                null_lo: f64::NAN,
                recovery: f64::NAN,
                below_null: false,
                argmax_frac: f64::NAN,
                births: out.total_births,
                starved: out.total_starved,
                mean_depth: f64::NAN,
                modal_repro: (u16::MAX, f64::NAN),
                modal_hungry: (u16::MAX, f64::NAN),
                modal_sense: (u16::MAX, f64::NAN),
            };
        }
        let mean_decay =
            orgs.iter().map(|o| o.decay_amount as f64).sum::<f64>() / orgs.len() as f64;
        let argmax_frac =
            orgs.iter().filter(|o| o.sense_move == argmax_idx).count() as f64 / orgs.len() as f64;

        let d = depths(&out.births, INITIAL_ORGANISMS);
        let survivor_depths: Vec<u32> = orgs.iter().map(|o| *d.get(&o.id).unwrap_or(&0)).collect();
        let (null_mean, null_lo, _null_hi) = drift_null(start_decay, &survivor_depths);

        let span = (start_decay as f64) - (DECAY_BOUNDS.0 as f64);
        let recovery = if span > 0.0 {
            (start_decay as f64 - mean_decay) / span
        } else {
            f64::NAN
        };

        Outcome {
            extinct: false,
            survivors: orgs.len(),
            mean_decay,
            null_mean,
            null_lo,
            recovery,
            below_null: mean_decay < null_lo,
            argmax_frac,
            births: out.total_births,
            starved: out.total_starved,
            mean_depth: survivor_depths.iter().map(|&d| d as f64).sum::<f64>()
                / survivor_depths.len() as f64,
            modal_repro: modal(orgs.iter().map(|o| o.repro_promoter)),
            modal_hungry: modal(orgs.iter().map(|o| o.hungry_promoter)),
            modal_sense: modal(orgs.iter().map(|o| o.sense_move)),
        }
    }

    pub fn main() {
        let t_all = Instant::now();
        let role_pools = discover_pools(&cells_dir());
        println!(
            "discovered pools: {} promoters, {} movement candidates",
            role_pools.promoters.len(),
            role_pools.movement.len()
        );

        let starting = load_starting_genome(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("genomes/grazer.json"),
        );
        let argmax_idx = role_pools.movement_index(&starting.genes.sense_move);
        let argmin_idx = role_pools
            .movement
            .iter()
            .position(|n| n == "argmin3")
            .map(|i| i as u16);
        println!(
            "curated sense_move `{}` = pool index {argmax_idx}; argmin3 = {:?}",
            starting.genes.sense_move, argmin_idx
        );
        println!(
            "NOTE: a swap lands on 1 of {} movement cells; returning to `{}` specifically is \
             ~1/{} per swap event (pre-registered as P3).\n",
            role_pools.movement.len(),
            starting.genes.sense_move,
            role_pools.movement.len()
        );

        let genes = GenePools::load(
            &cells_dir(),
            &starting.genes.decay,
            &starting.genes.eat,
            &starting.genes.split,
            &role_pools,
        )
        .expect("compiling gene pools");

        println!(
            "{:<20} {:<8} {:>6} {:>7} {:>9} {:>9} {:>9} {:>6} {:>8}",
            "rung",
            "mode",
            "seeds",
            "extinct",
            "mean_dec",
            "null_mean",
            "null_lo",
            "below",
            "recov"
        );
        println!("{}", "-".repeat(96));

        for rung in RUNGS {
            let sense_idx = match rung.sense_cell {
                None => Some(argmax_idx),
                Some("argmin3") => argmin_idx,
                Some(other) => role_pools
                    .movement
                    .iter()
                    .position(|n| n == other)
                    .map(|i| i as u16),
            };
            let Some(sense_idx) = sense_idx else {
                println!(
                    "{:<20} SKIPPED — `{}` not in the movement pool",
                    rung.name,
                    rung.sense_cell.unwrap_or("?")
                );
                continue;
            };

            let starting2 = StartingGenome2 {
                initial_energy: starting.initial_energy,
                decay_amount: rung.decay,
                repro_threshold: rung.threshold.unwrap_or(starting.repro_threshold),
                repro_give_pct: starting.repro_give_pct,
                hungry_promoter: role_pools.promoter_index(&starting.genes.hungry_promoter),
                repro_promoter: role_pools.promoter_index(&starting.genes.repro_promoter),
                sense_move: sense_idx,
            };

            for (mode_name, mode) in MODES {
                let mut outcomes: Vec<Outcome> = Vec::new();
                for s in 0..SEEDS {
                    let cfg = RunConfig2DGenome {
                        seed: BASE_SEED.wrapping_add(s as u64),
                        ticks: TICKS,
                        initial_organisms: INITIAL_ORGANISMS,
                        world_width: 32,
                        world_height: 32,
                        food_density: 0.2,
                        food_value: 40,
                        regrow_ticks: 8,
                        mutation_mode: *mode,
                    };
                    let out = ex2::run(EngineKind::Gpu, &cfg, &starting2, &genes);
                    outcomes.push(analyse(&out, rung.decay, argmax_idx));
                }

                let extinct = outcomes.iter().filter(|o| o.extinct).count();
                let live: Vec<&Outcome> = outcomes.iter().filter(|o| !o.extinct).collect();
                let avg = |f: &dyn Fn(&Outcome) -> f64| -> f64 {
                    if live.is_empty() {
                        f64::NAN
                    } else {
                        live.iter().map(|o| f(o)).sum::<f64>() / live.len() as f64
                    }
                };
                let below = live.iter().filter(|o| o.below_null).count();

                println!(
                    "{:<20} {:<8} {:>6} {:>7} {:>9.3} {:>9.3} {:>9.3} {:>4}/{:<1} {:>8.3}",
                    rung.name,
                    mode_name,
                    SEEDS,
                    extinct,
                    avg(&|o| o.mean_decay),
                    avg(&|o| o.null_mean),
                    avg(&|o| o.null_lo),
                    below,
                    live.len(),
                    avg(&|o| o.recovery),
                );

                if rung.sense_cell.is_some() {
                    println!(
                        "{:<20} {:<8} {:>6} argmax3 fraction in final population: {:.4}",
                        "",
                        "",
                        "",
                        avg(&|o| o.argmax_frac)
                    );
                }
                if !live.is_empty() {
                    let name = |pool: &[String], i: u16| -> String {
                        pool.get(i as usize).cloned().unwrap_or_else(|| "?".into())
                    };
                    let r = live[0];
                    println!(
                        "{:>29} survivors {:.0}, births {:.0}, starved {:.0}, mean lineage depth {:.0}",
                        "",
                        avg(&|o| o.survivors as f64),
                        avg(&|o| o.births as f64),
                        avg(&|o| o.starved as f64),
                        avg(&|o| o.mean_depth),
                    );
                    println!(
                        "{:>29} modal roles (seed 0): repro `{}` {:.0}%, hungry `{}` {:.0}%, sense `{}` {:.0}%",
                        "",
                        name(&role_pools.promoters, r.modal_repro.0),
                        r.modal_repro.1 * 100.0,
                        name(&role_pools.promoters, r.modal_hungry.0),
                        r.modal_hungry.1 * 100.0,
                        name(&role_pools.movement, r.modal_sense.0),
                        r.modal_sense.1 * 100.0,
                    );
                }
            }
            println!();
        }

        println!("total wall clock: {:.1}s", t_all.elapsed().as_secs_f64());
        println!(
            "\nBar (pre-registered): D2/full needs mean decay below the null's 95% lower bound \
             in >=4 of 5 seeds AND mean recovery >=0.50."
        );
    }
}
