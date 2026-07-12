//! EX-3's replay assertion, mirroring `tests/ex2_cpu_replay.rs` for the two-species engine:
//! the same `(seed, both genomes)` run twice on the CPU reference interpreter must produce
//! byte-identical history, and predation must actually engage (not immediate one-sided
//! collapse) — otherwise this isn't testing the mechanic this experiment is about.
use std::path::{Path, PathBuf};

use cell80_life::ex3::{self, RunConfig3, StartingGenome3};
use cell80_life::ex2::GenePools;
use cell80_life::genes::EngineKind;
use cell80_life::history::Species;
use cell80_life::load_starting_genome;
use cell80_life::pools::discover_pools;

fn cells_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cell80/cells")
}

fn genome_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("genomes/{name}.json"))
}

fn run_config() -> RunConfig3 {
    RunConfig3 {
        seed: 0x5eed_1234_c311_80ff,
        ticks: 400,
        initial_grazers: 40,
        initial_predators: 8,
        world_width: 48,
        world_height: 48,
        food_density: 0.25,
        food_value: 40,
        regrow_ticks: 8,
        mutation_enabled: true,
    }
}

fn load_starting3(
    name: &str,
    species: Species,
    role_pools: &cell80_life::pools::Pools,
) -> StartingGenome3 {
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

fn load_fixtures() -> (StartingGenome3, StartingGenome3, GenePools) {
    let role_pools = discover_pools(&cells_dir());
    let grazer = load_starting3("grazer", Species::Grazer, &role_pools);
    let predator = load_starting3("predator", Species::Predator, &role_pools);
    let grazer_genes = load_starting_genome(&genome_path("grazer"));
    let genes = GenePools::load(
        &cells_dir(),
        &grazer_genes.genes.decay,
        &grazer_genes.genes.eat,
        &grazer_genes.genes.split,
        &role_pools,
    )
    .expect("compiling gene pools");
    (grazer, predator, genes)
}

#[test]
fn replay_is_bit_exact_with_two_species() {
    let (grazer, predator, genes) = load_fixtures();
    let cfg = run_config();

    let run1 = ex3::run(EngineKind::CpuReference, &cfg, &grazer, &predator, &genes);
    let run2 = ex3::run(EngineKind::CpuReference, &cfg, &grazer, &predator, &genes);

    assert_eq!(run1.ticks.len(), run2.ticks.len());
    for (a, b) in run1.ticks.iter().zip(&run2.ticks) {
        assert_eq!(a, b, "tick {} diverged between two identical-seed runs", a.tick);
    }
    assert_eq!(run1.births, run2.births);
    assert_eq!(run1.history_hash, run2.history_hash);

    assert!(
        run1.total_predation_kills > 0,
        "expected predation to actually engage over {} ticks, got 0 kills",
        cfg.ticks
    );
    assert!(run1.total_births > 0, "expected at least one birth over {} ticks", cfg.ticks);
    assert!(
        run1.final_grazers > 0 && run1.final_predators > 0,
        "expected both species to survive to tick {} (grazers={}, predators={}), not a one-sided collapse",
        cfg.ticks, run1.final_grazers, run1.final_predators
    );
}
