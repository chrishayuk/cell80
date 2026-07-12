//! EX-2's replay assertion, mirroring `tests/ex1_cpu_replay.rs` for the mutation-capable
//! engine: the same `(seed, genome)` run twice on the CPU reference interpreter must
//! produce byte-identical history, including per-organism genome drift from mutation.
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

fn load_starting2(role_pools: &cell80_life::pools::Pools) -> (StartingGenome2, GenePools) {
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
        role_pools,
    )
    .expect("compiling gene pools");
    (starting2, genes)
}

#[test]
fn replay_is_bit_exact_with_mutation() {
    let role_pools = discover_pools(&cells_dir());
    let (starting, genes) = load_starting2(&role_pools);
    let cfg = run_config();

    let run1 = ex2::run(EngineKind::CpuReference, &cfg, &starting, &genes);
    let run2 = ex2::run(EngineKind::CpuReference, &cfg, &starting, &genes);

    assert_eq!(run1.ticks.len(), run2.ticks.len());
    for (a, b) in run1.ticks.iter().zip(&run2.ticks) {
        assert_eq!(
            a, b,
            "tick {} diverged between two identical-seed runs",
            a.tick
        );
    }
    assert_eq!(run1.births, run2.births);
    assert_eq!(run1.history_hash, run2.history_hash);
    assert!(
        run1.total_births > 0,
        "expected at least one birth over {} ticks",
        cfg.ticks
    );

    // The whole point of this experiment: genome diversity should actually emerge. If every
    // birth's role indices matched the parent's exactly, mutation would be a no-op.
    let any_swap = run1.births.iter().any(|b| {
        b.hungry_promoter != starting.hungry_promoter
            || b.repro_promoter != starting.repro_promoter
            || b.sense_move != starting.sense_move
    });
    let any_numeric_drift = run1.births.iter().any(|b| {
        b.decay_amount != starting.decay_amount
            || b.repro_threshold != starting.repro_threshold
            || b.repro_give_pct != starting.repro_give_pct
    });
    assert!(
        any_swap || any_numeric_drift,
        "no mutation observed over {} births",
        run1.births.len()
    );
}
