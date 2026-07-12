//! EX-4 lineage report (`experiments/deterministic-ecology.md`). Not a `#[test]` — reports
//! numbers for the findings doc, matching `ex1_sweep.rs`/`ex2_mutation_report.rs`'s
//! convention. Runs a real EX-2 population, builds the lineage tree, detects a real
//! sustained plurality-change event for one of the three swappable roles, traces its
//! causal origin(s), and confirms (or honestly refutes) causation by replaying with
//! exactly that mutation reverted.

#[cfg(not(target_os = "macos"))]
fn main() {
    println!(
        "ex4_lineage_report needs macOS (Metal) for the GPU engine path; the CPU-reference \
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
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    use cell80_life::ex2::{self, FieldOverride, GenePools, RunConfig2DGenome, StartingGenome2};
    use cell80_life::genes::EngineKind;
    use cell80_life::history::TickRecord2DGenome;
    use cell80_life::lineage::{
        detect_plurality_events, find_origins, GenomeFields, LineageTree, OriginKind,
        PluralityEvent, Role,
    };
    use cell80_life::load_starting_genome;
    use cell80_life::pools::discover_pools;

    fn cells_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cell80/cells")
    }

    fn genome_path(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("genomes/{name}.json"))
    }

    const ROLES: [Role; 3] = [Role::Hungry, Role::Repro, Role::Sense];
    const SAMPLE_EVERY: u32 = 20;

    fn role_name(role: Role) -> &'static str {
        match role {
            Role::Hungry => "hungry_promoter",
            Role::Repro => "repro_promoter",
            Role::Sense => "sense_move",
        }
    }

    fn diff_line(label: &str, parent: u16, child: u16) {
        if parent == child {
            println!("    {label}: {parent} (unchanged)");
        } else {
            println!("    {label}: {parent} -> {child}  <-- changed");
        }
    }

    fn print_origin(origin: &OriginKind) {
        match origin {
            OriginKind::Genesis { genesis_id } => {
                println!(
                    "  origin: GENESIS (id {genesis_id}) — this value was the starting genome's \
                     all along; the event is a reversion to baseline after a transient \
                     displacement, not a novel fixation."
                );
            }
            OriginKind::Mutated {
                origin_child_id,
                origin_parent_id,
                origin_tick,
                parent_genome,
                child_genome,
            } => {
                println!(
                    "  origin: birth of organism {origin_child_id} (parent {origin_parent_id}) \
                     at tick {origin_tick} — full 6-field diff:"
                );
                diff_line("decay_amount", parent_genome.decay_amount, child_genome.decay_amount);
                diff_line("repro_threshold", parent_genome.repro_threshold, child_genome.repro_threshold);
                diff_line("repro_give_pct", parent_genome.repro_give_pct, child_genome.repro_give_pct);
                diff_line("hungry_promoter", parent_genome.hungry_promoter, child_genome.hungry_promoter);
                diff_line("repro_promoter", parent_genome.repro_promoter, child_genome.repro_promoter);
                diff_line("sense_move", parent_genome.sense_move, child_genome.sense_move);
            }
        }
    }

    /// Does an event matching `(role, to)` still show up as a sustained plurality winner
    /// anywhere within +/- two sample intervals of `near_tick` in another run's history?
    fn still_reaches_plurality(
        ticks: &[TickRecord2DGenome],
        role: Role,
        to: u16,
        near_tick: u32,
        sustain_k: usize,
    ) -> bool {
        detect_plurality_events(ticks, role, SAMPLE_EVERY, sustain_k)
            .iter()
            .any(|e| e.to == to && e.shift_tick.abs_diff(near_tick) <= SAMPLE_EVERY * 2)
    }

    /// Best candidate found for one seed's baseline: the event/origins pair to report,
    /// and whether it actually has a mutation to revert (vs. a genesis-only reversion).
    struct Candidate {
        event: PluralityEvent,
        origins: Vec<OriginKind>,
        k: usize,
        has_mutation: bool,
    }

    /// Scan every (K, role) combination against one seed's tick history and pick the best
    /// candidate: prefer a single traceable mutation over several, prefer any mutation over
    /// a genesis-only reversion, prefer a smaller K (a tighter, more conservative bar) when
    /// otherwise equal.
    fn best_candidate(tree: &LineageTree, ticks: &[TickRecord2DGenome]) -> Option<Candidate> {
        let mut best: Option<Candidate> = None;
        for &k in &[5usize, 3, 10] {
            for &role in &ROLES {
                for event in detect_plurality_events(ticks, role, SAMPLE_EVERY, k) {
                    let origins = find_origins(tree, ticks, &event);
                    let has_mutation = origins.iter().any(|o| matches!(o, OriginKind::Mutated { .. }));
                    let better = match &best {
                        None => true,
                        Some(b) => (has_mutation && !b.has_mutation)
                            || (has_mutation == b.has_mutation && origins.len() < b.origins.len()),
                    };
                    if better {
                        best = Some(Candidate { event, origins, k, has_mutation });
                    }
                }
            }
        }
        best
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
        let genes = GenePools::load(
            &cells_dir(),
            &starting.genes.decay,
            &starting.genes.eat,
            &starting.genes.split,
            &role_pools,
        )
        .expect("compiling gene pools");
        let starting_fields = GenomeFields::from_starting(&starting2);

        // Same seeds `cell80-life-findings.md`'s Finding 3 swept, same order — the primary
        // config first, then fall back through the rest looking for a mutation-bearing event.
        let seeds = [0x5eed_1234_c311_80ffu64, 1, 2, 3, 4, 5, 42, 999, 123456];
        let cfg_for = |seed: u64| RunConfig2DGenome {
            seed,
            ticks: 2000,
            initial_organisms: 8,
            world_width: 32,
            world_height: 32,
            food_density: 0.2,
            food_value: 40,
            regrow_ticks: 8,
        };

        let mut chosen: Option<(u64, ex2::RunOutput2DGenome, Candidate)> = None;
        for &seed in &seeds {
            let cfg = cfg_for(seed);
            println!("== seed {seed:#x}: running baseline ({} ticks, GPU) ==", cfg.ticks);
            let baseline = ex2::run(EngineKind::Gpu, &cfg, &starting2, &genes);
            let tree = LineageTree::build(&starting_fields, cfg.initial_organisms, &baseline.births);

            match best_candidate(&tree, &baseline.ticks) {
                Some(c) if c.has_mutation => {
                    println!(
                        "  found a mutation-bearing event: {} {} -> {} at tick {} (K={})\n",
                        role_name(c.event.role), c.event.from, c.event.to, c.event.shift_tick, c.k
                    );
                    chosen = Some((seed, baseline, c));
                    break;
                }
                Some(c) => {
                    println!(
                        "  only a genesis-only reversion found ({} -> {} at tick {}); trying the \
                         next seed for a mutation-bearing event.\n",
                        role_name(c.event.role), c.event.to, c.event.shift_tick
                    );
                    if chosen.is_none() {
                        chosen = Some((seed, baseline, c));
                    }
                }
                None => println!("  no sustained plurality event at any K/role; trying the next seed.\n"),
            }
        }

        let Some((seed, baseline, candidate)) = chosen else {
            println!(
                "\nNo sustained plurality event (any K) found in any of {} seeds — the actual \
                 kill condition for this experiment, reported plainly.",
                seeds.len()
            );
            return;
        };

        let Candidate { event, origins, k, has_mutation } = candidate;

        println!(
            "== event: seed {seed:#x}, role {}, {} -> {} at tick {} (K={k}) ==",
            role_name(event.role), event.from, event.to, event.shift_tick
        );
        println!(
            "   share at shift: {:.1}%   peak share in the sustained window: {:.1}%",
            event.share_at_shift * 100.0,
            event.peak_share_in_window * 100.0
        );
        println!("\n== origin(s): {} found ==", origins.len());
        for o in &origins {
            print_origin(o);
        }

        if !has_mutation {
            println!(
                "\nEvery origin traced to genesis — there is no mutation to revert; this event \
                 is a reversion to the starting genome, not something a counterfactual replay \
                 can meaningfully test. (Searched all {} seeds; none produced a mutation-bearing \
                 event to demonstrate the counterfactual on.)",
                seeds.len()
            );
            return;
        }

        let mut overrides: HashMap<u32, FieldOverride> = HashMap::new();
        for o in &origins {
            if let OriginKind::Mutated { origin_child_id, .. } = o {
                overrides.insert(*origin_child_id, event.role.skip_override());
            }
        }

        let cfg = cfg_for(seed);
        println!(
            "\n== counterfactual: reverting {} origin(s)' `{}` mutation, replaying seed {seed:#x} ==",
            overrides.len(),
            role_name(event.role)
        );
        let counterfactual = ex2::run_with_overrides(EngineKind::Gpu, &cfg, &starting2, &genes, &overrides);

        let earliest_origin_tick = origins
            .iter()
            .filter_map(|o| match o {
                OriginKind::Mutated { origin_tick, .. } => Some(*origin_tick),
                OriginKind::Genesis { .. } => None,
            })
            .min()
            .unwrap_or(0);
        let pre_fork_identical = baseline
            .ticks
            .iter()
            .zip(&counterfactual.ticks)
            .take_while(|(a, _)| a.tick < earliest_origin_tick)
            .all(|(a, b)| a == b);
        println!("   ticks strictly before the earliest reverted birth byte-identical: {pre_fork_identical}");

        let still_happens =
            still_reaches_plurality(&counterfactual.ticks, event.role, event.to, event.shift_tick, k);
        if still_happens {
            println!(
                "\n== RESULT: the event still occurs in the counterfactual replay — the \
                 detected origin(s) were NOT the sole cause. Either a redundant/later \
                 independent origin reaches the same value, or this value was reachable \
                 through some other path this pass didn't isolate. An honest, real finding \
                 about redundant origins, not a failure of the mechanism."
            );
        } else {
            println!(
                "\n== RESULT: the event no longer occurs in the counterfactual replay — \
                 causation confirmed. Reverting exactly the traced mutation(s) removed the \
                 sustained plurality shift, and nothing else in the run changed before that \
                 point (pre-fork ticks identical: {pre_fork_identical})."
            );
        }
    }
}
