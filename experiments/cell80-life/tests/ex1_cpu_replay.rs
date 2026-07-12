//! EX-1's replay assertion, mirroring `tests/ex0_cpu_replay.rs` for the 2D engine: the same
//! `(seed, genome)` run twice on the CPU reference interpreter must produce byte-identical
//! history. No platform gate — `cell80_core::Interp` is pure `std`.
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
fn replay_is_bit_exact_2d() {
    let genome = load_starting_genome(&grazer_genome_path());
    let genes = GeneSet::load(&cells_dir(), &genome.genes).expect("compiling grazer genes");
    let cfg = run_config();

    let run1 = ex1::run(EngineKind::CpuReference, &cfg, &genome, &genes);
    let run2 = ex1::run(EngineKind::CpuReference, &cfg, &genome, &genes);

    assert_eq!(run1.ticks.len(), run2.ticks.len());
    for (a, b) in run1.ticks.iter().zip(&run2.ticks) {
        assert_eq!(a, b, "tick {} diverged between two identical-seed 2D runs", a.tick);
    }
    assert_eq!(run1.history_hash, run2.history_hash);
    assert!(run1.births > 0, "expected at least one birth over {} ticks", cfg.ticks);
}
