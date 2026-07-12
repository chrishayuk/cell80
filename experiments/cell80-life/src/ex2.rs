//! EX-2's tick engine (`experiments/deterministic-ecology.md`): per-organism genome
//! diversity via mutation on reproduction — operator (a), parametric jitter (numeric
//! thresholds) + cell-swap (which pool member fills a role) — ported from `main.rs`'s
//! original CPU-only mutation model into the GPU-batchable engine EX-0/EX-1 built.
//!
//! A new file, not a modification of `ex1.rs`: `ex1.rs`'s tests are built around one
//! population-shared `StartingGenome`/`GeneSet`; EX-2 changes what a genome *is*
//! (per-organism) and what the gene machinery provides (pools, not fixed cells) — not a
//! signature-compatible extension. `ex1.rs`/its tests stay untouched.
//!
//! Heterogeneous *numeric* parameters need no new dispatch mechanism — different values in
//! the same `[u16;3]` batch input are already free (`decay`/`eat`/`split` read each
//! organism's own numeric fields directly, still one dispatch per tick against the one
//! shared cell for each of those three fixed roles). Heterogeneous *cell choice* (which
//! pool member backs `hungry_promoter`/`repro_promoter`/`sense_move`) goes through
//! `genes::batch_run_grouped`: one dispatch per distinct pool index in use, not one per
//! organism — a micro-benchmark found `CompiledGene::run_gpu_batch`'s per-call cost is
//! ~0.2ms regardless of batch size (fixed launch overhead dominates), so even the worst
//! case (every pool member represented — up to ~40 promoters, ~43 movement candidates)
//! stays comfortably cheap at this pass's population scale.
//!
//! Movement/contention/world are otherwise unchanged from `ex1.rs` — EX-2 only makes genome
//! *content* heterogeneous, not the tick's control-flow shape.
use std::collections::HashMap;
use std::path::Path;

use crate::contention;
use crate::genes::{batch_run, batch_run_grouped, sum_steps, CompiledGene, EngineKind};
use crate::history::{BirthEvent, HistoryHasher, OrgSnapshot2DGenome, TickRecord2DGenome};
use crate::pools::Pools;
use crate::rng;
use crate::world2d::World2D;

// Same bounds/rates as `main.rs`'s proven mutation model, for continuity with the one
// reference implementation this ports.
const DECAY_BOUNDS: (i32, i32) = (1, 6);
const THRESHOLD_BOUNDS: (i32, i32) = (50, 400);
const GIVE_PCT_BOUNDS: (i32, i32) = (10, 90);
const NUMERIC_MUTATE_PCT: u64 = 25;
const SWAP_MUTATE_PCT: u64 = 8;

/// The three fixed roles (population-shared, never mutated — no same-scale alternative
/// cells exist for them, same reasoning `main.rs` established) plus the three swappable
/// roles' discovered candidate pools. `hungry_pool`/`repro_pool` are independently
/// compiled from the same promoter-name list (`Pools::promoters`) — two separate
/// `CompiledGene` sets, not a shared/cloned one, since `CompiledGene` owns a `GpuBatch`.
pub struct GenePools {
    pub decay: CompiledGene,
    pub eat: CompiledGene,
    pub split: CompiledGene,
    pub hungry_pool: Vec<CompiledGene>,
    pub repro_pool: Vec<CompiledGene>,
    pub sense_pool: Vec<CompiledGene>,
}

impl GenePools {
    pub fn load(
        cells_dir: &Path,
        decay_name: &str,
        eat_name: &str,
        split_name: &str,
        role_pools: &Pools,
    ) -> Result<Self, String> {
        let load_all = |names: &[String]| -> Result<Vec<CompiledGene>, String> {
            names.iter().map(|n| CompiledGene::load(cells_dir, n)).collect()
        };
        Ok(GenePools {
            decay: CompiledGene::load(cells_dir, decay_name)?,
            eat: CompiledGene::load(cells_dir, eat_name)?,
            split: CompiledGene::load(cells_dir, split_name)?,
            hungry_pool: load_all(&role_pools.promoters)?,
            repro_pool: load_all(&role_pools.promoters)?,
            sense_pool: load_all(&role_pools.movement)?,
        })
    }
}

/// The starting population's shared genome — every initial organism starts identical;
/// diversity only emerges from mutation on reproduction (matching `main.rs`'s own
/// initialization discipline). Role fields are pool indices — build via
/// `Pools::promoter_index`/`Pools::movement_index` from a `StartingGenome`'s named cells.
pub struct StartingGenome2 {
    pub initial_energy: u16,
    pub decay_amount: u16,
    pub repro_threshold: u16,
    pub repro_give_pct: u16,
    pub hungry_promoter: u16,
    pub repro_promoter: u16,
    pub sense_move: u16,
}

pub struct RunConfig2DGenome {
    pub seed: u64,
    pub ticks: u32,
    pub initial_organisms: usize,
    pub world_width: usize,
    pub world_height: usize,
    pub food_density: f64,
    pub food_value: u16,
    pub regrow_ticks: u16,
}

pub struct RunOutput2DGenome {
    pub history_hash: [u8; 32],
    pub ticks: Vec<TickRecord2DGenome>,
    pub births: Vec<BirthEvent>,
    pub final_population: usize,
    pub total_births: u32,
    pub total_starved: u32,
}

/// `pub(crate)` (not private) so `ex3.rs` can reuse this type and `mutate()` directly for
/// its own (species-tagged) organisms, rather than duplicating an identical struct.
#[derive(Clone)]
pub(crate) struct OrgGenome {
    pub(crate) decay_amount: u16,
    pub(crate) repro_threshold: u16,
    pub(crate) repro_give_pct: u16,
    pub(crate) hungry_promoter: u16,
    pub(crate) repro_promoter: u16,
    pub(crate) sense_move: u16,
}

struct Org {
    id: u32,
    pos: usize,
    energy: u16,
    genome: OrgGenome,
}

/// EX-4's counterfactual mechanism: force one specific birth's `mutate()` call to skip one
/// specific field, so it inherits the parent's value there instead of the mutated one —
/// "vary the *one* mutation," not the whole birth (`mutate()`'s 6 branches are
/// independently RNG-gated, so a single birth can flip more than one field at once;
/// reverting the whole call would confound attribution to just the field under test).
#[derive(Debug, Clone, Copy, Default)]
pub struct FieldOverride {
    pub skip_decay: bool,
    pub skip_threshold: bool,
    pub skip_give_pct: bool,
    pub skip_hungry_swap: bool,
    pub skip_repro_swap: bool,
    pub skip_sense_swap: bool,
}

/// Per-birth overrides, keyed by the child's id (deterministic given `(seed, cfg, starting,
/// pools)` — see `run_with_overrides`'s doc comment for why the same id names the same
/// birth across a baseline run and its counterfactual replay).
pub type Overrides = HashMap<u32, FieldOverride>;

fn clamp_u16(v: i32, bounds: (i32, i32)) -> u16 {
    v.clamp(bounds.0, bounds.1) as u16
}

/// Build a child's genome from its parent's, applying operator (a)'s two mutation kinds —
/// numeric jitter and cell-swap — each keyed by its own RNG stream and the CHILD's id (the
/// new organism is what's being generated), a pure function of `(seed, tick, child_id,
/// stream)`. Numeric step spans (`%3-1`, `%21-10`, `%11-5`) match `main.rs`'s
/// `Rng::step(1|10|5)` exactly, for continuity with the one proven reference.
///
/// `override_`, when `Some`, forces specific fields to skip their mutation branch entirely
/// (the child inherits the parent's value there) — EX-4's counterfactual mechanism. Every
/// stream is a pure function of its four inputs with no shared cursor, so skipping one
/// field's effect has zero impact on any other field's draw.
///
/// `pub(crate)` (not private) so `ex3.rs` can call this directly for its own organisms —
/// mutation itself doesn't know or care about species, matching `main.rs`'s own discipline
/// (only numeric thresholds and role-cell choices evolve within a species, never species
/// itself).
#[allow(clippy::too_many_arguments)]
pub(crate) fn mutate(
    seed: u64,
    tick: u32,
    child_id: u32,
    parent: &OrgGenome,
    hungry_pool_len: u16,
    repro_pool_len: u16,
    sense_pool_len: u16,
    override_: Option<&FieldOverride>,
) -> OrgGenome {
    let mut child = parent.clone();
    let ov = override_.copied().unwrap_or_default();

    if !ov.skip_decay
        && rng::chance(seed, tick, child_id, rng::MUTATE_DECAY_CHANCE_STREAM, NUMERIC_MUTATE_PCT)
    {
        let step = (rng::draw(seed, tick, child_id, rng::MUTATE_DECAY_MAGNITUDE_STREAM) % 3) as i32 - 1;
        child.decay_amount = clamp_u16(child.decay_amount as i32 + step, DECAY_BOUNDS);
    }
    if !ov.skip_threshold
        && rng::chance(seed, tick, child_id, rng::MUTATE_THRESHOLD_CHANCE_STREAM, NUMERIC_MUTATE_PCT)
    {
        let step = (rng::draw(seed, tick, child_id, rng::MUTATE_THRESHOLD_MAGNITUDE_STREAM) % 21) as i32 - 10;
        child.repro_threshold = clamp_u16(child.repro_threshold as i32 + step, THRESHOLD_BOUNDS);
    }
    if !ov.skip_give_pct
        && rng::chance(seed, tick, child_id, rng::MUTATE_GIVE_PCT_CHANCE_STREAM, NUMERIC_MUTATE_PCT)
    {
        let step = (rng::draw(seed, tick, child_id, rng::MUTATE_GIVE_PCT_MAGNITUDE_STREAM) % 11) as i32 - 5;
        child.repro_give_pct = clamp_u16(child.repro_give_pct as i32 + step, GIVE_PCT_BOUNDS);
    }

    if !ov.skip_hungry_swap
        && hungry_pool_len >= 2
        && rng::chance(seed, tick, child_id, rng::MUTATE_HUNGRY_SWAP_CHANCE_STREAM, SWAP_MUTATE_PCT)
    {
        child.hungry_promoter = rng::pick_other_index(
            seed, tick, child_id, rng::MUTATE_HUNGRY_SWAP_TARGET_STREAM,
            child.hungry_promoter, hungry_pool_len,
        );
    }
    if !ov.skip_repro_swap
        && repro_pool_len >= 2
        && rng::chance(seed, tick, child_id, rng::MUTATE_REPRO_SWAP_CHANCE_STREAM, SWAP_MUTATE_PCT)
    {
        child.repro_promoter = rng::pick_other_index(
            seed, tick, child_id, rng::MUTATE_REPRO_SWAP_TARGET_STREAM,
            child.repro_promoter, repro_pool_len,
        );
    }
    if !ov.skip_sense_swap
        && sense_pool_len >= 2
        && rng::chance(seed, tick, child_id, rng::MUTATE_SENSE_SWAP_CHANCE_STREAM, SWAP_MUTATE_PCT)
    {
        child.sense_move = rng::pick_other_index(
            seed, tick, child_id, rng::MUTATE_SENSE_SWAP_TARGET_STREAM,
            child.sense_move, sense_pool_len,
        );
    }

    child
}

/// The original, single-genome-per-run engine — unchanged signature and behavior from
/// before EX-4 (delegates to `run_impl` with no overrides, provably a no-op path: every
/// override lookup below becomes `None`).
pub fn run(
    engine: EngineKind,
    cfg: &RunConfig2DGenome,
    starting: &StartingGenome2,
    pools: &GenePools,
) -> RunOutput2DGenome {
    run_impl(engine, cfg, starting, pools, None)
}

/// EX-4's counterfactual entry point: identical to `run`, except specific births (keyed by
/// child id) skip specific mutation fields per `overrides`. Because `child_id` assignment
/// is a pure function of `(seed, cfg, starting, pools)` — a plain monotonic counter,
/// incremented only while iterating organisms by `Vec` index, never via a `HashMap`/
/// `HashSet` — the same id from a baseline `run()` names the same birth event here. Every
/// tick strictly before an overridden birth is therefore byte-identical between the two
/// calls; ticks at and after it can genuinely diverge (the reverted organism now behaves
/// differently, which can ripple into who else reproduces/dies/contests a tile) — compare
/// post-fork state by genome value / population statistics, never by raw `child_id`
/// equality across the two runs.
pub fn run_with_overrides(
    engine: EngineKind,
    cfg: &RunConfig2DGenome,
    starting: &StartingGenome2,
    pools: &GenePools,
    overrides: &Overrides,
) -> RunOutput2DGenome {
    run_impl(engine, cfg, starting, pools, Some(overrides))
}

fn run_impl(
    engine: EngineKind,
    cfg: &RunConfig2DGenome,
    starting: &StartingGenome2,
    pools: &GenePools,
    overrides: Option<&Overrides>,
) -> RunOutput2DGenome {
    let mut world = World2D::new(
        cfg.seed,
        cfg.world_width,
        cfg.world_height,
        cfg.food_density,
        cfg.food_value,
        cfg.regrow_ticks,
    );
    let world_len = world.len().max(1);
    let mut next_id: u32 = 0;
    let n0 = cfg.initial_organisms.max(1);
    let starting_genome = OrgGenome {
        decay_amount: starting.decay_amount,
        repro_threshold: starting.repro_threshold,
        repro_give_pct: starting.repro_give_pct,
        hungry_promoter: starting.hungry_promoter,
        repro_promoter: starting.repro_promoter,
        sense_move: starting.sense_move,
    };
    let mut orgs: Vec<Org> = (0..cfg.initial_organisms)
        .map(|i| {
            let id = next_id;
            next_id += 1;
            Org {
                id,
                pos: (i * world_len / n0) % world_len,
                energy: starting.initial_energy,
                genome: starting_genome.clone(),
            }
        })
        .collect();

    let hungry_pool_len = pools.hungry_pool.len() as u16;
    let repro_pool_len = pools.repro_pool.len() as u16;
    let sense_pool_len = pools.sense_pool.len() as u16;

    let mut hasher = HistoryHasher::new();
    let mut records = Vec::with_capacity(cfg.ticks as usize);
    let mut all_births: Vec<BirthEvent> = Vec::new();
    let mut total_births = 0u32;
    let mut total_starved = 0u32;
    let mut total_contention_losses = 0u32;

    for tick in 0..cfg.ticks {
        if orgs.is_empty() {
            break;
        }
        let food_snapshot = world.food.clone();
        let sense_x = |pos: usize| -> (u16, u16, u16) {
            let here = food_snapshot[pos];
            let west = food_snapshot[world.neighbor_index(pos, -1, 0)];
            let east = food_snapshot[world.neighbor_index(pos, 1, 0)];
            (here, west, east)
        };
        let sense_y = |pos: usize| -> (u16, u16, u16) {
            let here = food_snapshot[pos];
            let north = food_snapshot[world.neighbor_index(pos, 0, -1)];
            let south = food_snapshot[world.neighbor_index(pos, 0, 1)];
            (here, north, south)
        };
        let senses_x: Vec<(u16, u16, u16)> = orgs.iter().map(|o| sense_x(o.pos)).collect();
        let senses_y: Vec<(u16, u16, u16)> = orgs.iter().map(|o| sense_y(o.pos)).collect();

        // Stage 1: decay — fixed/shared cell, heterogeneous numeric input (already free).
        let decay_in: Vec<[u16; 3]> = orgs
            .iter()
            .map(|o| [o.energy, o.genome.decay_amount, 0])
            .collect();
        let decay_out = batch_run(engine, &pools.decay, &decay_in);

        // Stage 2/3: sense_move, once per axis — swappable role, grouped by pool index.
        let sense_role_idx: Vec<u16> = orgs.iter().map(|o| o.genome.sense_move).collect();
        let sense_x_in: Vec<[u16; 3]> = senses_x.iter().map(|&(h, w, e)| [h, w, e]).collect();
        let action_x = batch_run_grouped(engine, &pools.sense_pool, &sense_role_idx, &sense_x_in);
        let sense_y_in: Vec<[u16; 3]> = senses_y.iter().map(|&(h, n, s)| [h, n, s]).collect();
        let action_y = batch_run_grouped(engine, &pools.sense_pool, &sense_role_idx, &sense_y_in);

        // Stage 4: hungry_promoter — swappable role, grouped by pool index.
        let hungry_role_idx: Vec<u16> = orgs.iter().map(|o| o.genome.hungry_promoter).collect();
        let hungry_in: Vec<[u16; 3]> = senses_x.iter().map(|&(h, _, _)| [h, 0, 0]).collect();
        let hungry_out = batch_run_grouped(engine, &pools.hungry_pool, &hungry_role_idx, &hungry_in);

        // Stage 5: eat — fixed/shared cell, against post-decay energy.
        let eat_in: Vec<[u16; 3]> = senses_x
            .iter()
            .zip(&decay_out)
            .map(|(&(h, _, _), &(e1, _))| [e1, h, 0])
            .collect();
        let eat_out = batch_run(engine, &pools.eat, &eat_in);

        // Resolve: apply decay + the axis-priority movement rule now (identical control
        // flow to ex1.rs — genome heterogeneity only changed which cells computed the
        // batches above, not how their outputs get applied).
        let mut eat_candidates: Vec<(u32, usize)> = Vec::new();
        let mut resolved_energy = vec![0u16; orgs.len()];
        for (i, o) in orgs.iter_mut().enumerate() {
            resolved_energy[i] = decay_out[i].0;
            let ax = action_x[i].0;
            let ay = action_y[i].0;
            let (h_x, w, e) = senses_x[i];
            let (h_y, n, s) = senses_y[i];
            let diff_x = match ax {
                1 => w as i32 - h_x as i32,
                2 => e as i32 - h_x as i32,
                _ => 0,
            };
            let diff_y = match ay {
                1 => n as i32 - h_y as i32,
                2 => s as i32 - h_y as i32,
                _ => 0,
            };
            let move_x = ax != 0;
            let move_y = ay != 0;
            if !move_x && !move_y {
                if hungry_out[i].0 == 1 {
                    eat_candidates.push((o.id, o.pos));
                }
            } else if move_x && (!move_y || diff_x >= diff_y) {
                o.pos = match ax {
                    1 => world.neighbor_index(o.pos, -1, 0),
                    2 => world.neighbor_index(o.pos, 1, 0),
                    _ => o.pos,
                };
            } else {
                o.pos = match ay {
                    1 => world.neighbor_index(o.pos, 0, -1),
                    2 => world.neighbor_index(o.pos, 0, 1),
                    _ => o.pos,
                };
            }
        }

        let winners = contention::resolve_eat_contention(cfg.seed, tick, &eat_candidates);
        total_contention_losses += (eat_candidates.len() - winners.len()) as u32;
        let mut ate_positions: Vec<usize> = Vec::new();
        for (i, o) in orgs.iter().enumerate() {
            if winners.contains(&o.id) {
                resolved_energy[i] = eat_out[i].0;
                ate_positions.push(o.pos);
            }
        }
        for pos in ate_positions {
            world.eat_at(pos);
        }

        // Starvation.
        let mut survivors: Vec<usize> = Vec::new();
        for (i, &energy) in resolved_energy.iter().enumerate() {
            if energy == 0 {
                total_starved += 1;
            } else {
                orgs[i].energy = energy;
                survivors.push(i);
            }
        }

        // repro_promoter (swappable, grouped) / split (fixed) — survivors only.
        let repro_role_idx: Vec<u16> = survivors.iter().map(|&i| orgs[i].genome.repro_promoter).collect();
        let repro_in: Vec<[u16; 3]> = survivors
            .iter()
            .map(|&i| [orgs[i].energy, orgs[i].genome.repro_threshold, 0])
            .collect();
        let repro_out = batch_run_grouped(engine, &pools.repro_pool, &repro_role_idx, &repro_in);
        let split_in: Vec<[u16; 3]> = survivors
            .iter()
            .map(|&i| [orgs[i].energy, orgs[i].genome.repro_give_pct, 0])
            .collect();
        let split_out = batch_run(engine, &pools.split, &split_in);

        let mut children: Vec<Org> = Vec::new();
        for (k, &i) in survivors.iter().enumerate() {
            if repro_out[k].0 == 1 {
                let parent_keep = split_out[k].0;
                let child_energy = orgs[i].energy.saturating_sub(parent_keep);
                orgs[i].energy = parent_keep;
                let child_pos = world.neighbor_index(orgs[i].pos, 1, 0);
                let id = next_id;
                next_id += 1;
                let child_genome = mutate(
                    cfg.seed, tick, id, &orgs[i].genome,
                    hungry_pool_len, repro_pool_len, sense_pool_len,
                    overrides.and_then(|o| o.get(&id)),
                );
                all_births.push(BirthEvent {
                    child_id: id,
                    parent_id: orgs[i].id,
                    tick,
                    decay_amount: child_genome.decay_amount,
                    repro_threshold: child_genome.repro_threshold,
                    repro_give_pct: child_genome.repro_give_pct,
                    hungry_promoter: child_genome.hungry_promoter,
                    repro_promoter: child_genome.repro_promoter,
                    sense_move: child_genome.sense_move,
                });
                children.push(Org {
                    id,
                    pos: child_pos,
                    energy: child_energy,
                    genome: child_genome,
                });
                total_births += 1;
            }
        }

        let total_ir_steps = sum_steps(&[
            &decay_out, &action_x, &action_y, &hungry_out, &eat_out, &repro_out, &split_out,
        ]);

        let placeholder = || Org {
            id: 0,
            pos: 0,
            energy: 0,
            genome: starting_genome.clone(),
        };
        let mut new_orgs: Vec<Org> = survivors
            .into_iter()
            .map(|i| std::mem::replace(&mut orgs[i], placeholder()))
            .collect();
        new_orgs.extend(children);
        orgs = new_orgs;

        world.tick_regrow();

        let mut snap: Vec<OrgSnapshot2DGenome> = orgs
            .iter()
            .map(|o| {
                let (x, y) = world.xy(o.pos);
                OrgSnapshot2DGenome {
                    id: o.id,
                    x: x as u16,
                    y: y as u16,
                    energy: o.energy,
                    decay_amount: o.genome.decay_amount,
                    repro_threshold: o.genome.repro_threshold,
                    repro_give_pct: o.genome.repro_give_pct,
                    hungry_promoter: o.genome.hungry_promoter,
                    repro_promoter: o.genome.repro_promoter,
                    sense_move: o.genome.sense_move,
                }
            })
            .collect();
        snap.sort_by_key(|s| s.id);

        let record = TickRecord2DGenome {
            tick,
            organisms: snap,
            food: world.food.clone(),
            births: total_births,
            starved: total_starved,
            contention_losses: total_contention_losses,
            total_ir_steps,
        };
        hasher.absorb2d_genome(&record);
        records.push(record);
    }

    RunOutput2DGenome {
        history_hash: hasher.finish(),
        ticks: records,
        births: all_births,
        final_population: orgs.len(),
        total_births,
        total_starved,
    }
}
