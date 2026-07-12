//! EX-4's counterfactual mechanism, proven correct on its own terms before ever pointing it
//! at a real detected event: pick one real birth's one mutated field, revert exactly that
//! field via `ex2::run_with_overrides`, and confirm (a) every tick strictly before the
//! overridden birth is byte-identical to the baseline, (b) the reverted field now matches
//! the parent's value, and (c) the run's history hash diverges from that point on (proving
//! the override wasn't a silent no-op).
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use cell80_life::ex2::{self, FieldOverride, GenePools, RunConfig2DGenome, StartingGenome2};
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
        ticks: 300,
        initial_organisms: 8,
        world_width: 32,
        world_height: 32,
        food_density: 0.2,
        food_value: 40,
        regrow_ticks: 8,
    }
}

#[test]
fn reverting_one_field_forks_the_run_from_exactly_that_birth() {
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

    let baseline = ex2::run(EngineKind::CpuReference, &cfg, &starting2, &genes);

    // Find a birth whose parent is a genesis organism (so the parent's genome is trivially
    // the starting genome, no lineage lookup needed for this self-contained test) and which
    // differs from the starting genome in at least one field.
    let target = baseline
        .births
        .iter()
        .find(|b| {
            (b.parent_id as usize) < cfg.initial_organisms
                && (b.decay_amount != starting2.decay_amount
                    || b.repro_threshold != starting2.repro_threshold
                    || b.repro_give_pct != starting2.repro_give_pct
                    || b.hungry_promoter != starting2.hungry_promoter
                    || b.repro_promoter != starting2.repro_promoter
                    || b.sense_move != starting2.sense_move)
        })
        .unwrap_or_else(|| panic!("no birth with an observable mutation from a genesis parent over {} ticks", cfg.ticks));

    let (field_name, override_) = if target.hungry_promoter != starting2.hungry_promoter {
        ("hungry_promoter", FieldOverride { skip_hungry_swap: true, ..Default::default() })
    } else if target.repro_promoter != starting2.repro_promoter {
        ("repro_promoter", FieldOverride { skip_repro_swap: true, ..Default::default() })
    } else if target.sense_move != starting2.sense_move {
        ("sense_move", FieldOverride { skip_sense_swap: true, ..Default::default() })
    } else if target.decay_amount != starting2.decay_amount {
        ("decay_amount", FieldOverride { skip_decay: true, ..Default::default() })
    } else if target.repro_threshold != starting2.repro_threshold {
        ("repro_threshold", FieldOverride { skip_threshold: true, ..Default::default() })
    } else {
        ("repro_give_pct", FieldOverride { skip_give_pct: true, ..Default::default() })
    };

    let target_child_id = target.child_id;
    let target_tick = target.tick;

    let mut overrides = HashMap::new();
    overrides.insert(target_child_id, override_);
    let counterfactual = ex2::run_with_overrides(EngineKind::CpuReference, &cfg, &starting2, &genes, &overrides);

    // (a) Every tick strictly before the overridden birth is byte-identical — nothing
    // upstream of the fork changed.
    for (a, b) in baseline.ticks.iter().zip(&counterfactual.ticks) {
        if a.tick >= target_tick {
            break;
        }
        assert_eq!(a, b, "tick {} diverged before the overridden birth (tick {target_tick})", a.tick);
    }

    // (b) The reverted field on that specific child now matches the parent's (starting)
    // value in the counterfactual run's own birth log.
    let reran = counterfactual
        .births
        .iter()
        .find(|b| b.child_id == target_child_id)
        .unwrap_or_else(|| panic!("child {target_child_id} missing from the counterfactual run's birth log"));
    let (before, after) = match field_name {
        "hungry_promoter" => (target.hungry_promoter, reran.hungry_promoter),
        "repro_promoter" => (target.repro_promoter, reran.repro_promoter),
        "sense_move" => (target.sense_move, reran.sense_move),
        "decay_amount" => (target.decay_amount, reran.decay_amount),
        "repro_threshold" => (target.repro_threshold, reran.repro_threshold),
        _ => (target.repro_give_pct, reran.repro_give_pct),
    };
    assert_ne!(before, after, "the override had no effect on `{field_name}`");
    let starting_value = match field_name {
        "hungry_promoter" => starting2.hungry_promoter,
        "repro_promoter" => starting2.repro_promoter,
        "sense_move" => starting2.sense_move,
        "decay_amount" => starting2.decay_amount,
        "repro_threshold" => starting2.repro_threshold,
        _ => starting2.repro_give_pct,
    };
    assert_eq!(after, starting_value, "reverted `{field_name}` should now match the (genesis) parent's value");

    // (c) The override was not a silent no-op: the two runs' overall history diverges.
    assert_ne!(baseline.history_hash, counterfactual.history_hash);
}
