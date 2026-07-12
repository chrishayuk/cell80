//! EX-2's GPU-parity assertion, mirroring `tests/ex1_gpu_vs_cpu.rs` for the mutation-capable
//! engine: the same `(seed, genome)` run on the CPU reference interpreter and on the Metal
//! GPU body must agree byte-for-byte — including the grouped-by-pool-index dispatch for
//! swappable roles, and trap folding for any pool member that halts/div-by-zeros/exhausts
//! fuel on a given organism's inputs (see `genes.rs::run_cpu`'s doc comment).
#![cfg(target_os = "macos")]

use std::path::{Path, PathBuf};

use cell80_life::ex2::{self, GenePools, RunConfig2DGenome, StartingGenome2};
use cell80_life::genes::EngineKind;
use cell80_life::load_starting_genome;
use cell80_life::pools::discover_pools;

fn cells_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cell80/cells")
}

fn grazer_genome_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("genomes/grazer.json")
}

fn run_config() -> RunConfig2DGenome {
    RunConfig2DGenome {
        seed: 0x5eed_1234_c311_80ff,
        ticks: 150,
        initial_organisms: 8,
        world_width: 32,
        world_height: 32,
        food_density: 0.2,
        food_value: 40,
        regrow_ticks: 8,
    }
}

#[test]
fn gpu_matches_cpu_reference_with_mutation() {
    let role_pools = discover_pools(&cells_dir());
    let starting = load_starting_genome(&grazer_genome_path());
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
    let cfg = run_config();

    let cpu = ex2::run(EngineKind::CpuReference, &cfg, &starting2, &genes);
    let gpu = ex2::run(EngineKind::Gpu, &cfg, &starting2, &genes);

    assert_eq!(cpu.ticks.len(), gpu.ticks.len());
    for (a, b) in cpu.ticks.iter().zip(&gpu.ticks) {
        assert_eq!(a, b, "tick {} diverged between CPU-reference and GPU (mutation-capable)", a.tick);
    }
    assert_eq!(cpu.births, gpu.births);
    assert_eq!(cpu.history_hash, gpu.history_hash);
    assert!(cpu.total_births > 0, "expected at least one birth over {} ticks", cfg.ticks);
}
