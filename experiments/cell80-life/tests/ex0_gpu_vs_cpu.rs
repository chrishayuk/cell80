//! EX-0's second assertion (`experiments/deterministic-ecology.md`): the same `(seed,
//! genome)` run on the CPU reference interpreter and on the Metal GPU body must agree
//! byte-for-byte. macOS-only (Metal), matching `cell80/tests/msl_battery.rs`'s file-level
//! `#[cfg(target_os = "macos")]` convention.
#![cfg(target_os = "macos")]

use std::path::{Path, PathBuf};

use cell80_life::ex0::{self, EngineKind, RunConfig};
use cell80_life::genes::GeneSet;
use cell80_life::load_starting_genome;

fn cells_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cell80/cells")
}

fn grazer_genome_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("genomes/grazer.json")
}

fn run_config() -> RunConfig {
    RunConfig {
        seed: 0x5eed_1234_c311_80ff,
        ticks: 200,
        initial_organisms: 8,
        world_len: 24,
        food_value: 40,
        regrow_ticks: 8,
    }
}

#[test]
fn gpu_matches_cpu_reference() {
    let genome = load_starting_genome(&grazer_genome_path());
    let genes = GeneSet::load(&cells_dir(), &genome.genes).expect("compiling grazer genes");
    let cfg = run_config();

    let cpu = ex0::run(EngineKind::CpuReference, &cfg, &genome, &genes);
    let gpu = ex0::run(EngineKind::Gpu, &cfg, &genome, &genes);

    assert_eq!(
        cpu.ticks.len(),
        gpu.ticks.len(),
        "CPU-reference and GPU runs recorded a different number of ticks"
    );
    for (a, b) in cpu.ticks.iter().zip(&gpu.ticks) {
        // Comparing the whole record (not just the final hash) localizes a disagreement to
        // a tick/organism/role, matching `msl_battery.rs`'s disagreement-localizing ethos.
        assert_eq!(a, b, "tick {} diverged between CPU-reference and GPU", a.tick);
    }
    assert_eq!(
        cpu.history_hash, gpu.history_hash,
        "history hash diverged between CPU-reference and GPU"
    );
    assert!(cpu.births > 0, "expected at least one birth over {} ticks", cfg.ticks);
}
