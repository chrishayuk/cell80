//! EX-3's GPU-parity assertion, mirroring `tests/ex2_gpu_vs_cpu.rs` for the two-species
//! engine: the same `(seed, both genomes)` run on the CPU reference interpreter and on the
//! Metal GPU body must agree byte-for-byte, including predation-kill contention and the
//! trap-folding `genes.rs::run_cpu` already established.
#![cfg(target_os = "macos")]

use std::path::{Path, PathBuf};

use cell80_life::ex2::GenePools;
use cell80_life::ex3::{self, RunConfig3, StartingGenome3};
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
        predator_satiation_ticks: 20,
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

#[test]
fn gpu_matches_cpu_reference_with_two_species() {
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
    let cfg = run_config();

    let cpu = ex3::run(EngineKind::CpuReference, &cfg, &grazer, &predator, &genes);
    let gpu = ex3::run(EngineKind::Gpu, &cfg, &grazer, &predator, &genes);

    assert_eq!(cpu.ticks.len(), gpu.ticks.len());
    for (a, b) in cpu.ticks.iter().zip(&gpu.ticks) {
        assert_eq!(a, b, "tick {} diverged between CPU-reference and GPU (two-species)", a.tick);
    }
    assert_eq!(cpu.births, gpu.births);
    assert_eq!(cpu.history_hash, gpu.history_hash);
    assert!(cpu.total_predation_kills > 0, "expected predation to actually engage");
}
