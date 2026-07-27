//! EX-6's replay assertion plus the two mode-semantics checks
//! (`experiments/deterministic-ecology-ex6-preregistration.md`). Mirrors
//! `tests/ex2_cpu_replay.rs` for the mutation-mode factor: same `(seed, genome, mode)` run
//! twice on the CPU reference interpreter must produce byte-identical history, and each mode
//! must actually do what its name claims — otherwise EX-6's whole comparison is measuring
//! something other than the operator it says it is.
use std::path::{Path, PathBuf};

use cell80_life::ex2::{self, GenePools, MutationMode, RunConfig2DGenome, StartingGenome2};
use cell80_life::genes::EngineKind;
use cell80_life::load_starting_genome;
use cell80_life::pools::discover_pools;

fn cells_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cell80/cells")
}

fn run_config(mode: MutationMode) -> RunConfig2DGenome {
    RunConfig2DGenome {
        seed: 0x5eed_1234_c311_80ff,
        ticks: 150,
        initial_organisms: 8,
        world_width: 32,
        world_height: 32,
        food_density: 0.2,
        food_value: 40,
        regrow_ticks: 8,
        mutation_mode: mode,
    }
}

fn setup() -> (cell80_life::pools::Pools, StartingGenome2, GenePools) {
    let role_pools = discover_pools(&cells_dir());
    let starting =
        load_starting_genome(&Path::new(env!("CARGO_MANIFEST_DIR")).join("genomes/grazer.json"));
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
    (role_pools, starting2, genes)
}

#[test]
fn every_mutation_mode_replays_bit_exact() {
    let (_pools, starting2, genes) = setup();
    for mode in [
        MutationMode::Off,
        MutationMode::NumericOnly,
        MutationMode::Full,
    ] {
        let cfg = run_config(mode);
        let a = ex2::run(EngineKind::CpuReference, &cfg, &starting2, &genes);
        let b = ex2::run(EngineKind::CpuReference, &cfg, &starting2, &genes);
        assert_eq!(a.history_hash, b.history_hash, "{mode:?} history hash");
        assert_eq!(a.births, b.births, "{mode:?} birth log");
        assert_eq!(
            a.final_population, b.final_population,
            "{mode:?} final population"
        );
    }
}

/// `Off` must be a genuinely flat floor: children clone parents, so no genome field can
/// differ from the starting genome anywhere in the birth log. EX-6's recovery numbers are
/// only interpretable against a floor that is actually flat — if this drifts, the reported
/// "recovery 0.000" baseline is measuring the wrong thing.
#[test]
fn mutation_off_freezes_the_genome() {
    let (_pools, starting2, genes) = setup();
    let out = ex2::run(
        EngineKind::CpuReference,
        &run_config(MutationMode::Off),
        &starting2,
        &genes,
    );
    assert!(
        !out.births.is_empty(),
        "need births for this assertion to mean anything"
    );
    for b in &out.births {
        assert_eq!(b.decay_amount, starting2.decay_amount, "decay drifted");
        assert_eq!(
            b.repro_threshold, starting2.repro_threshold,
            "threshold drifted"
        );
        assert_eq!(
            b.repro_give_pct, starting2.repro_give_pct,
            "give_pct drifted"
        );
        assert_eq!(
            b.hungry_promoter, starting2.hungry_promoter,
            "hungry swapped"
        );
        assert_eq!(b.repro_promoter, starting2.repro_promoter, "repro swapped");
        assert_eq!(b.sense_move, starting2.sense_move, "sense swapped");
    }
}

/// `NumericOnly` must suppress every swap branch while leaving numeric jitter alone. Both
/// halves matter: EX-6's headline reading is that the numeric ratchet is swamped by swap
/// load, which is only meaningful if this mode really does isolate the numeric operator.
#[test]
fn numeric_only_suppresses_swaps_but_keeps_jitter() {
    let (_pools, starting2, genes) = setup();
    let out = ex2::run(
        EngineKind::CpuReference,
        &run_config(MutationMode::NumericOnly),
        &starting2,
        &genes,
    );
    assert!(!out.births.is_empty(), "need births");
    for b in &out.births {
        assert_eq!(
            b.hungry_promoter, starting2.hungry_promoter,
            "hungry swapped"
        );
        assert_eq!(b.repro_promoter, starting2.repro_promoter, "repro swapped");
        assert_eq!(b.sense_move, starting2.sense_move, "sense swapped");
    }
    let jittered = out.births.iter().any(|b| {
        b.decay_amount != starting2.decay_amount
            || b.repro_threshold != starting2.repro_threshold
            || b.repro_give_pct != starting2.repro_give_pct
    });
    assert!(
        jittered,
        "numeric jitter never fired — this mode would be indistinguishable from Off"
    );
}
