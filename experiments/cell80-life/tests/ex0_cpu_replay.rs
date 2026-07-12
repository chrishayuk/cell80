//! EX-0's first assertion (`experiments/deterministic-ecology.md`): the same `(seed,
//! genome)` run twice, on the CPU reference interpreter, must produce byte-identical
//! history. No platform gate — `cell80_core::Interp` is pure `std`, so this runs anywhere.
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
fn replay_is_bit_exact() {
    let genome = load_starting_genome(&grazer_genome_path());
    let genes = GeneSet::load(&cells_dir(), &genome.genes).expect("compiling grazer genes");
    let cfg = run_config();

    let run1 = ex0::run(EngineKind::CpuReference, &cfg, &genome, &genes);
    let run2 = ex0::run(EngineKind::CpuReference, &cfg, &genome, &genes);

    assert_eq!(
        run1.ticks.len(),
        run2.ticks.len(),
        "the two runs recorded a different number of ticks"
    );
    for (a, b) in run1.ticks.iter().zip(&run2.ticks) {
        assert_eq!(a, b, "tick {} diverged between two identical-seed runs", a.tick);
    }
    assert_eq!(
        run1.history_hash, run2.history_hash,
        "history hash diverged between two identical-seed CPU-reference runs"
    );

    // A sanity check that this run actually does something (births/deaths/starvation),
    // not a degenerate no-op — a replay gate on an empty run would prove nothing.
    assert!(run1.births > 0, "expected at least one birth over {} ticks", cfg.ticks);
}
