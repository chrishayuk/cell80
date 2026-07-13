//! EX-4 fitness counterfactual (`experiments/deterministic-ecology.md`). Not a `#[test]` —
//! reports numbers for the findings doc, matching `ex4_lineage_report.rs`'s convention.
//!
//! EX-2 operator (b) left one question genuinely unanswered: the *aggregate* showed
//! composed-gene carriers averaging fewer children than disk-gene carriers (0.829 vs.
//! 1.067), but that is an *observational* comparison across different organisms in
//! different micro-contexts — a correlation, not a causal read. The design doc's literal
//! criterion is "occasionally fitter than its *specific* parent," which is a
//! *counterfactual*: take one organism that acquired a composed `sense_move` gene by
//! mutation, revert exactly that one swap so the *same* organism instead inherits its
//! parent's disk gene, replay the world, and compare that organism's own reproductive
//! output with vs. without the composed gene. Because `child_id` assignment is a pure
//! function of `(seed, cfg, starting, pools)` and every tick strictly before the reverted
//! birth is byte-identical between the two runs (see `ex2::run_with_overrides`'s contract),
//! the focal organism has the same id, birth tick, position and energy in both — the only
//! difference at the fork is its `sense_move` gene. Direct-children count (the same metric
//! the aggregate used) is the primary signal; transitive descendant count is a secondary,
//! more sensitive one that also picks up divergence the fork rippled into the wider world.

#[cfg(not(target_os = "macos"))]
fn main() {
    println!(
        "ex4_fitness_counterfactual needs macOS (Metal) for the GPU engine path; the \
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
    use std::collections::{HashMap, VecDeque};
    use std::path::{Path, PathBuf};
    use std::time::Instant;

    use cell80_life::composition::{fingerprint_pool_member, grow_pool, ComposablePool};
    use cell80_life::ex2::{self, GenePools, Overrides, RunConfig2DGenome, StartingGenome2};
    use cell80_life::genes::{CompiledGene, EngineKind};
    use cell80_life::history::BirthEvent;
    use cell80_life::lineage::Role;
    use cell80_life::load_starting_genome;
    use cell80_life::pools::discover_pools;

    fn cells_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cell80/cells")
    }

    fn genome_path(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("genomes/{name}.json"))
    }

    /// Direct children of `id` in one run's birth log.
    fn direct_children(births: &[BirthEvent], id: u32) -> usize {
        births.iter().filter(|b| b.parent_id == id).count()
    }

    /// Transitive descendant count (subtree size under `id`) within one run's births.
    fn descendants(births: &[BirthEvent], id: u32) -> usize {
        let mut kids: HashMap<u32, Vec<u32>> = HashMap::new();
        for b in births {
            kids.entry(b.parent_id).or_default().push(b.child_id);
        }
        let mut queue = VecDeque::from([id]);
        let mut count = 0usize;
        while let Some(cur) = queue.pop_front() {
            if let Some(cs) = kids.get(&cur) {
                for &c in cs {
                    count += 1;
                    queue.push_back(c);
                }
            }
        }
        count
    }

    pub fn main() {
        let role_pools = discover_pools(&cells_dir());
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

        // Build the extended movement pool exactly as `ex2_mutation_report.rs` does: the same
        // composition sweep (seed, attempts), the same viable set appended by name.
        let movement_pool = ComposablePool::discover(&cells_dir(), &role_pools.movement, 3);
        let movement_fps: Vec<_> = role_pools
            .movement
            .iter()
            .filter_map(|n| fingerprint_pool_member(&cells_dir(), n, 3))
            .collect();
        let sweep_seed = 0x5eed_c0de_c0de_5eedu64;
        let movement_growth = grow_pool(&movement_pool, &movement_fps, sweep_seed, 300);
        if movement_growth.viable.is_empty() {
            println!("no viable composed candidates from the sweep — nothing to test.");
            return;
        }

        let base_len = role_pools.movement.len();
        let mut extended_genes = GenePools::load(
            &cells_dir(),
            &starting.genes.decay,
            &starting.genes.eat,
            &starting.genes.split,
            &role_pools,
        )
        .expect("compiling gene pools");
        for c in &movement_growth.viable {
            let name = format!("{}∘{}[slot{}]", c.f_name, c.g_name, c.slot);
            let compiled = CompiledGene::from_funcs(&name, c.funcs.clone(), Vec::new())
                .expect("compiling a composed candidate");
            extended_genes.sense_pool.push(compiled);
        }

        // Same config and seed as EX-2 operator (b)'s adoption experiment — this is the exact
        // run whose 29.6% adoption / 0.829-vs-1.067 aggregate we are now dissecting causally.
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

        println!(
            "== baseline (extended pool: {base_len} disk + {} composed = {} movement genes) ==",
            movement_growth.viable.len(),
            extended_genes.sense_pool.len()
        );
        let baseline = ex2::run(EngineKind::Gpu, &cfg, &starting2, &extended_genes);
        println!(
            "   final_pop={} total_births={}",
            baseline.final_population, baseline.total_births
        );

        // child_id -> its own sense_move; founders (id < initial_organisms) carry the starting
        // genome's (disk) gene.
        let sense_of: HashMap<u32, u16> = baseline
            .births
            .iter()
            .map(|b| (b.child_id, b.sense_move))
            .collect();
        let parent_sense = |parent_id: u32| -> u16 {
            if (parent_id as usize) < cfg.initial_organisms {
                starting2.sense_move
            } else {
                *sense_of.get(&parent_id).unwrap_or(&starting2.sense_move)
            }
        };

        // Focal set: births whose swap took a *disk* parent gene to a *composed* child gene —
        // the clean "composed gene vs. the specific disk gene it replaced in the parent"
        // contrast. Reverting the swap makes the child inherit that disk gene.
        let focal: Vec<&BirthEvent> = baseline
            .births
            .iter()
            .filter(|b| {
                (b.sense_move as usize) >= base_len
                    && (parent_sense(b.parent_id) as usize) < base_len
            })
            .collect();
        println!(
            "   composed-origin births (disk parent -> composed child): {}",
            focal.len()
        );
        if focal.is_empty() {
            println!("no composed-origin births with a disk parent — nothing to revert.");
            return;
        }

        // Sample up to N, evenly spread across the run (deterministic — no RNG in this binary).
        let n_target = 15usize;
        let step = (focal.len() / n_target).max(1);
        let sampled: Vec<&BirthEvent> =
            focal.iter().step_by(step).take(n_target).copied().collect();
        println!(
            "   testing {} focal births (every {}th of {}), one counterfactual replay each\n",
            sampled.len(),
            step,
            focal.len()
        );
        println!(
            "   per-org: children(parent-disk gene) vs children(composed gene), Δ = composed − disk"
        );

        let mut d_children: Vec<i64> = Vec::new();
        let mut d_desc: Vec<i64> = Vec::new();
        let t0 = Instant::now();
        for &fb in sampled.iter() {
            let mut ov = Overrides::new();
            ov.insert(fb.child_id, Role::Sense.skip_override());
            let cf =
                ex2::run_with_overrides(EngineKind::Gpu, &cfg, &starting2, &extended_genes, &ov);

            let composed_k = direct_children(&baseline.births, fb.child_id) as i64; // baseline: has composed gene
            let disk_k = direct_children(&cf.births, fb.child_id) as i64; // counterfactual: reverted to disk
            let composed_d = descendants(&baseline.births, fb.child_id) as i64;
            let disk_d = descendants(&cf.births, fb.child_id) as i64;
            d_children.push(composed_k - disk_k);
            d_desc.push(composed_d - disk_d);

            println!(
                "   org {:>6} @tick {:>4}: gene {:>3} vs parent disk {:>2} | children {} vs {} (Δ{:+}) | descendants {} vs {} (Δ{:+})",
                fb.child_id,
                fb.tick,
                fb.sense_move,
                parent_sense(fb.parent_id),
                disk_k,
                composed_k,
                composed_k - disk_k,
                disk_d,
                composed_d,
                composed_d - disk_d,
            );
        }

        let n = d_children.len() as f64;
        let mean = |v: &[i64]| v.iter().sum::<i64>() as f64 / v.len().max(1) as f64;
        let pos = |v: &[i64]| v.iter().filter(|&&x| x > 0).count();
        let zero = |v: &[i64]| v.iter().filter(|&&x| x == 0).count();
        let neg = |v: &[i64]| v.iter().filter(|&&x| x < 0).count();

        println!(
            "\n== RESULT ({} focal births, {:.1}s) ==",
            sampled.len(),
            t0.elapsed().as_secs_f64()
        );
        println!(
            "direct-children Δ (composed − parent disk): mean {:+.3}  |  fitter {}, equal {}, worse {}  (of {})",
            mean(&d_children),
            pos(&d_children),
            zero(&d_children),
            neg(&d_children),
            n as usize
        );
        println!(
            "descendants Δ    (composed − parent disk): mean {:+.3}  |  fitter {}, equal {}, worse {}  (of {})",
            mean(&d_desc),
            pos(&d_desc),
            zero(&d_desc),
            neg(&d_desc),
            n as usize
        );
        if pos(&d_children) > 0 || pos(&d_desc) > 0 {
            println!(
                "\n== VERDICT: at least one specific composed gene is *occasionally fitter than its \
                 specific parent* — the design doc's literal criterion is met on a real \
                 counterfactual, even though the population aggregate selects mildly against \
                 composed genes. \"Occasionally fitter\" and \"suppressed below the neutral \
                 baseline on average\" are both true and not in tension."
            );
        } else {
            println!(
                "\n== VERDICT: no focal composed gene beat its specific parent on either metric in \
                 this sample — consistent with uniform (not just aggregate) purifying selection \
                 against the composed candidates in this world. Reported plainly."
            );
        }
    }
}
