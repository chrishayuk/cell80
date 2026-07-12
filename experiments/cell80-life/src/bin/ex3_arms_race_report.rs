//! EX-3 coupled-trait-change detection (`experiments/deterministic-ecology.md`). Not a
//! `#[test]` — exploratory: dumps the real chronological, cross-species plurality-event
//! timeline from a long flagship run before any pattern-matching heuristic gets written,
//! matching this project's own "measure, don't assume" discipline (`ex1_sweep.rs`'s
//! precedent). The pre-registered bar (user decision) requires BOTH a multi-volley
//! alternating temporal pattern (predator event -> prey response -> a further predator
//! event, across >=2-3 rounds — not a single coincidental pair) AND a counterfactual-replay
//! confirmation (EX-4's revert-and-replay discipline, reused via `ex3::run_with_overrides`).

#[cfg(not(target_os = "macos"))]
fn main() {
    println!(
        "ex3_arms_race_report needs macOS (Metal) for the GPU engine path; the CPU-reference \
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
    use std::path::{Path, PathBuf};

    use cell80_life::ex2::GenePools;
    use cell80_life::ex3::{self, RunConfig3, StartingGenome3};
    use cell80_life::genes::EngineKind;
    use cell80_life::history::Species;
    use cell80_life::lineage::{detect_plurality_events, eco_ticks_to_genome, PluralityEvent, Role};
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

    const ROLES: [Role; 3] = [Role::Hungry, Role::Repro, Role::Sense];
    const SAMPLE_EVERY: u32 = 20;
    const SUSTAIN_K: usize = 5;

    fn role_name(role: Role) -> &'static str {
        match role {
            Role::Hungry => "hungry_promoter",
            Role::Repro => "repro_promoter",
            Role::Sense => "sense_move",
        }
    }

    fn species_name(s: Species) -> &'static str {
        match s {
            Species::Grazer => "grazer",
            Species::Predator => "predator",
        }
    }

    struct TaggedEvent {
        species: Species,
        role: Role,
        event: PluralityEvent,
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

        // The calibrated, satiation-mechanic-verified flagship config (part 1/2/3 of
        // ex3_predator_prey_report.rs): 10/10 seeds sustained coexistence here.
        let seeds = [42u64, 1, 2, 3, 999, 0x5eed_1234_c311_80ff];
        let cfg_for = |seed: u64| RunConfig3 {
            seed,
            ticks: 10_000,
            initial_grazers: 60,
            initial_predators: 10,
            world_width: 48,
            world_height: 48,
            food_density: 0.3,
            food_value: 40,
            regrow_ticks: 8,
            mutation_enabled: true,
            predator_satiation_ticks: 20,
        };

        for &seed in &seeds {
            let cfg = cfg_for(seed);
            println!("\n== seed {seed:#x}: running flagship ({} ticks, GPU) ==", cfg.ticks);
            let out = ex3::run(EngineKind::Gpu, &cfg, &grazer, &predator, &genes);
            println!(
                "  final: grazers={} predators={}  total_births={}  total_predation_kills={}",
                out.final_grazers, out.final_predators, out.total_births, out.total_predation_kills
            );
            if out.final_grazers == 0 || out.final_predators == 0 {
                println!("  one-sided collapse this seed — skipping event analysis, trying next seed.");
                continue;
            }

            let grazer_ticks = eco_ticks_to_genome(&out.ticks, Species::Grazer);
            let predator_ticks = eco_ticks_to_genome(&out.ticks, Species::Predator);

            let mut tagged: Vec<TaggedEvent> = Vec::new();
            for &role in &ROLES {
                for event in detect_plurality_events(&grazer_ticks, role, SAMPLE_EVERY, SUSTAIN_K) {
                    tagged.push(TaggedEvent { species: Species::Grazer, role, event });
                }
                for event in detect_plurality_events(&predator_ticks, role, SAMPLE_EVERY, SUSTAIN_K) {
                    tagged.push(TaggedEvent { species: Species::Predator, role, event });
                }
            }
            tagged.sort_by_key(|t| t.event.shift_tick);

            println!(
                "  {} total events detected (K={SUSTAIN_K}); chronological cross-species timeline:",
                tagged.len()
            );
            for t in &tagged {
                println!(
                    "    tick {:>6}  {:<9} {:<16} {} -> {}  (share {:.2} -> peak {:.2})",
                    t.event.shift_tick,
                    species_name(t.species),
                    role_name(t.role),
                    t.event.from,
                    t.event.to,
                    t.event.share_at_shift,
                    t.event.peak_share_in_window
                );
            }

            // Longest strictly species-alternating run in the chronological timeline — the
            // raw signature of "predator event -> prey response -> a further predator
            // event" repeated, without yet distinguishing role or asserting causal
            // linkage (that's what the counterfactual replay is for).
            let labels: Vec<Species> = tagged.iter().map(|t| t.species).collect();
            let observed_run = longest_alternating_run(&labels);
            println!("  longest species-alternating chronological run: {observed_run} events");

            // Both event streams are dense and low-vote-share (0.2-0.5) — near-ties that
            // flip from ordinary demographic churn, not obvious directional selection (see
            // the module doc). With 6 independent (species, role) streams firing this
            // often, SOME cross-species alternation is close to guaranteed by chance alone,
            // so a raw run length isn't yet evidence of real temporal coupling. Null model:
            // shuffle the same species-label multiset across the same tick positions many
            // times (Fisher-Yates via this project's own deterministic counter-based draw,
            // not a new RNG dependency) and see how often chance alone reaches or exceeds
            // the observed run — an empirical p-value, not an eyeballed impression.
            const TRIALS: u32 = 20_000;
            let p = permutation_p_value(&labels, observed_run, seed, TRIALS);
            println!(
                "  permutation null check ({TRIALS} shuffles): P(run >= {observed_run} by chance alone) = {p:.4}"
            );
        }
    }

    fn longest_alternating_run(labels: &[Species]) -> usize {
        let mut best_run = 1usize.min(labels.len());
        let mut cur_run = if labels.is_empty() { 0 } else { 1 };
        for w in labels.windows(2) {
            if w[0] != w[1] {
                cur_run += 1;
                best_run = best_run.max(cur_run);
            } else {
                cur_run = 1;
            }
        }
        best_run
    }

    /// Fraction of `trials` random relabelings (same species-label multiset, same tick
    /// positions — only the assignment of which label goes where is shuffled) whose longest
    /// alternating run is `>= observed_run`. A small fraction means the observed alternation
    /// is genuinely unusual, not just what mixing two frequent, independent event streams
    /// produces anyway.
    fn permutation_p_value(labels: &[Species], observed_run: usize, seed: u64, trials: u32) -> f64 {
        if labels.len() < 2 {
            return 1.0;
        }
        let mut shuffled = labels.to_vec();
        let mut at_least_as_long = 0u32;
        for trial in 0..trials {
            for i in (1..shuffled.len()).rev() {
                let j = (cell80_life::rng::draw(seed, trial, i as u32, 255) as usize) % (i + 1);
                shuffled.swap(i, j);
            }
            if longest_alternating_run(&shuffled) >= observed_run {
                at_least_as_long += 1;
            }
        }
        at_least_as_long as f64 / trials as f64
    }
}
