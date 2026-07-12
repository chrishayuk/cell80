//! EX-1's GPU-parity assertion, mirroring `tests/ex0_gpu_vs_cpu.rs` for the 2D engine: the
//! same `(seed, genome)` run on the CPU reference interpreter and on the Metal GPU body
//! must agree byte-for-byte, including the two-axis `sense_move` decomposition.
#![cfg(target_os = "macos")]

use std::path::{Path, PathBuf};

use cell80_life::ex1::{self, RunConfig2D};
use cell80_life::genes::{EngineKind, GeneSet};
use cell80_life::load_starting_genome;

fn cells_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cell80/cells")
}

fn grazer_genome_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("genomes/grazer.json")
}

fn run_config() -> RunConfig2D {
    RunConfig2D {
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
fn gpu_matches_cpu_reference_2d() {
    let genome = load_starting_genome(&grazer_genome_path());
    let genes = GeneSet::load(&cells_dir(), &genome.genes).expect("compiling grazer genes");
    let cfg = run_config();

    let cpu = ex1::run(EngineKind::CpuReference, &cfg, &genome, &genes);
    let gpu = ex1::run(EngineKind::Gpu, &cfg, &genome, &genes);

    assert_eq!(cpu.ticks.len(), gpu.ticks.len());
    for (a, b) in cpu.ticks.iter().zip(&gpu.ticks) {
        assert_eq!(a, b, "tick {} diverged between CPU-reference and GPU (2D)", a.tick);
    }
    assert_eq!(cpu.history_hash, gpu.history_hash);
    assert!(cpu.births > 0, "expected at least one birth over {} ticks", cfg.ticks);
}
