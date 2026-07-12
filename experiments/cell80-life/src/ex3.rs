//! EX-3's tick engine (`experiments/deterministic-ecology.md`): a second species sharing
//! the world — predators hunting prey, ported from `main.rs`'s original CPU-only mechanic
//! into the GPU-batchable engine EX-1/EX-2 built, resolving predation via the same
//! order-independent contention discipline `contention.rs` already established for
//! eat-tile contests ("tournament dispatch... the GPU-scale version of pairwise contests,
//! batched," per the design doc).
//!
//! Species is orthogonal to genome content — it sits *beside* `OrgGenome`, not inside it,
//! heritable but never mutated (`main.rs`'s own discipline: only numeric thresholds and
//! role-cell choices evolve within a species). Both species reuse the exact same
//! `ex2::GenePools`/`ex2::mutate` machinery unchanged: `decay`/`eat`/`split` stay
//! fixed/shared, and the promoter/movement pools are species-agnostic (a predator and
//! grazer can land on the same pool index for a role) — species only changes (a) which
//! world query feeds `sense_move`/`hungry_promoter` (grazer: food; predator: prey, via
//! `predation::PreyIndex`, an O(1)-lookup replacement for `main.rs`'s O(n)-per-lookup
//! `prey_at`, required at this engine's population scale — see `predation.rs`), and (b) how
//! the result is interpreted (grazer action 0 = eat food; predator action 0 = attack).
//!
//! **A deliberate clarification from `main.rs`, not a literal port**: a predation kill
//! overrides everything else that tick for the victim (no eat, no reproduction, simply
//! removed) — `main.rs`'s sequential `Vec`-order processing let a victim act *if* processed
//! before its killer, an accident of iteration order with no batched-engine equivalent.
use crate::contention;
use crate::ex2::{mutate, GenePools, OrgGenome, Overrides};
use crate::genes::{batch_run, batch_run_grouped, sum_steps, EngineKind};
use crate::history::{BirthEventEco, HistoryHasher, OrgSnapshot2DEco, Species, TickRecord2DEco};
use crate::predation::PreyIndex;
use crate::world2d::World2D;

/// A predator with zero sensed prey (`argmax3(0,0,0)==0`, "stay") could sit frozen forever
/// since prey mostly camp at food tiles rather than roam — the same cause `main.rs`
/// documented, unaffected by moving to 2D (more directions doesn't help when every sensed
/// value is still 0). Cycles a small deterministic nudge through all 4 cardinal directions,
/// a pure function of `tick`, negligible next to any real prey signal.
const EXPLORE_BIAS: u16 = 1;
const EXPLORE_HALF_PERIOD: u32 = 20;

/// The starting genome for one species — every genesis organism of that species starts
/// identical; diversity only emerges from mutation on reproduction. `species` is fixed for
/// the whole lineage founded here.
pub struct StartingGenome3 {
    pub species: Species,
    pub initial_energy: u16,
    pub decay_amount: u16,
    pub repro_threshold: u16,
    pub repro_give_pct: u16,
    pub hungry_promoter: u16,
    pub repro_promoter: u16,
    pub sense_move: u16,
}

pub struct RunConfig3 {
    pub seed: u64,
    pub ticks: u32,
    pub initial_grazers: usize,
    pub initial_predators: usize,
    pub world_width: usize,
    pub world_height: usize,
    pub food_density: f64,
    pub food_value: u16,
    pub regrow_ticks: u16,
    /// The pre-registered control: skip `mutate()` entirely at birth (clone the parent's
    /// genome verbatim) — isolates "more room" (world size/population ratio) from
    /// "evolution" as the explanation for any population stability seen.
    pub mutation_enabled: bool,
    /// Ticks a predator must wait after a kill before it can attack again (0 disables the
    /// mechanic entirely). Checkpoint B: the mutation-off control replicated a 10/10-seed
    /// predator extinction at two otherwise fully robust configs — but the *same*
    /// satiation-less mechanic sustains healthy 10/10-seed coexistence with mutation on, so
    /// the mechanic wasn't the suspected cause. Built anyway per explicit decision, to rule
    /// out any residual overhunting confound rather than argue it away. A fixed constant,
    /// not a genome field — `main.rs`'s own discipline (species-level, not individual-level,
    /// unlike the swappable/numeric roles).
    pub predator_satiation_ticks: u32,
}

pub struct RunOutput3 {
    pub history_hash: [u8; 32],
    pub ticks: Vec<TickRecord2DEco>,
    pub births: Vec<BirthEventEco>,
    pub final_population: usize,
    pub final_grazers: usize,
    pub final_predators: usize,
    pub total_births: u32,
    pub total_starved: u32,
    pub total_predation_kills: u32,
}

struct Org {
    id: u32,
    pos: usize,
    energy: u16,
    species: Species,
    genome: OrgGenome,
    /// Predators only: the tick number before which this organism cannot attack again
    /// (0 = no active cooldown). Ecological state, not genome — never inherited, never
    /// mutated, never snapshotted (mirrors `world2d.rs`'s `regrow_at` idiom).
    satiation_until: u32,
}

fn genome_of(s: &StartingGenome3) -> OrgGenome {
    OrgGenome {
        decay_amount: s.decay_amount,
        repro_threshold: s.repro_threshold,
        repro_give_pct: s.repro_give_pct,
        hungry_promoter: s.hungry_promoter,
        repro_promoter: s.repro_promoter,
        sense_move: s.sense_move,
    }
}

/// A predation kill overrides everything else that tick for the victim — extracted so the
/// mapping from winning predator ids back to victim ids (the one place a predator/victim id
/// mix-up could hide) is independently testable, separate from where it gets applied.
fn killed_victims_from(
    predation_candidates: &[(u32, u32)],
    predation_winners: &std::collections::HashSet<u32>,
) -> std::collections::HashSet<u32> {
    predation_candidates
        .iter()
        .filter(|(pred_id, _)| predation_winners.contains(pred_id))
        .map(|(_, victim_id)| *victim_id)
        .collect()
}

/// The original entry point — unchanged signature/behavior (delegates to `run_impl` with no
/// overrides, a provable no-op path: every override lookup below becomes `None`).
pub fn run(
    engine: EngineKind,
    cfg: &RunConfig3,
    grazer_starting: &StartingGenome3,
    predator_starting: &StartingGenome3,
    pools: &GenePools,
) -> RunOutput3 {
    run_impl(engine, cfg, grazer_starting, predator_starting, pools, None)
}

/// EX-3's counterfactual entry point, the same discipline EX-4's `ex2::run_with_overrides`
/// established: revert one specific birth's one specific mutated field (keyed by child id,
/// itself a pure function of `(seed, cfg, startings, pools)` — see `ex2::run_with_overrides`'s
/// doc comment) and replay, to confirm a traced coupled-trait-change event is causal rather
/// than coincidental. Meaningful only when `cfg.mutation_enabled` — reverting a mutation in a
/// run that never mutates is a no-op by construction.
pub fn run_with_overrides(
    engine: EngineKind,
    cfg: &RunConfig3,
    grazer_starting: &StartingGenome3,
    predator_starting: &StartingGenome3,
    pools: &GenePools,
    overrides: &Overrides,
) -> RunOutput3 {
    run_impl(engine, cfg, grazer_starting, predator_starting, pools, Some(overrides))
}

fn run_impl(
    engine: EngineKind,
    cfg: &RunConfig3,
    grazer_starting: &StartingGenome3,
    predator_starting: &StartingGenome3,
    pools: &GenePools,
    overrides: Option<&Overrides>,
) -> RunOutput3 {
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

    let grazer_genome = genome_of(grazer_starting);
    let predator_genome = genome_of(predator_starting);
    let n_grazers = cfg.initial_grazers.max(1);
    let n_predators = cfg.initial_predators.max(1);

    let mut orgs: Vec<Org> = Vec::with_capacity(cfg.initial_grazers + cfg.initial_predators);
    for i in 0..cfg.initial_grazers {
        let id = next_id;
        next_id += 1;
        orgs.push(Org {
            id,
            pos: (i * world_len / n_grazers) % world_len,
            energy: grazer_starting.initial_energy,
            species: Species::Grazer,
            genome: grazer_genome.clone(),
            satiation_until: 0,
        });
    }
    for i in 0..cfg.initial_predators {
        let id = next_id;
        next_id += 1;
        orgs.push(Org {
            id,
            pos: (i * world_len / n_predators) % world_len,
            energy: predator_starting.initial_energy,
            species: Species::Predator,
            genome: predator_genome.clone(),
            satiation_until: 0,
        });
    }

    let hungry_pool_len = pools.hungry_pool.len() as u16;
    let repro_pool_len = pools.repro_pool.len() as u16;
    let sense_pool_len = pools.sense_pool.len() as u16;

    let mut hasher = HistoryHasher::new();
    let mut records = Vec::with_capacity(cfg.ticks as usize);
    let mut all_births: Vec<BirthEventEco> = Vec::new();
    let mut total_births = 0u32;
    let mut total_starved = 0u32;
    let mut total_contention_losses = 0u32;
    let mut total_predation_kills = 0u32;

    for tick in 0..cfg.ticks {
        if orgs.is_empty() {
            break;
        }
        let food_snapshot = world.food.clone();
        // Built once per tick from the tick-start snapshot's grazers only — O(n) build,
        // O(1) lookups per predator, instead of `main.rs`'s O(n)-per-lookup `prey_at`.
        let prey_index = PreyIndex::build(
            orgs.iter()
                .filter(|o| o.species == Species::Grazer)
                .map(|o| (o.id, o.pos, o.energy)),
        );
        let explore_phase = (tick / EXPLORE_HALF_PERIOD) % 4; // 0=W,1=E,2=N,3=S

        let sense_x = |o: &Org| -> (u16, u16, u16) {
            match o.species {
                Species::Grazer => {
                    let here = food_snapshot[o.pos];
                    let west = food_snapshot[world.neighbor_index(o.pos, -1, 0)];
                    let east = food_snapshot[world.neighbor_index(o.pos, 1, 0)];
                    (here, west, east)
                }
                Species::Predator => {
                    let here = prey_index.energy_at(o.pos);
                    let mut west = prey_index.energy_at(world.neighbor_index(o.pos, -1, 0));
                    let mut east = prey_index.energy_at(world.neighbor_index(o.pos, 1, 0));
                    if explore_phase == 0 {
                        west = west.saturating_add(EXPLORE_BIAS);
                    } else if explore_phase == 1 {
                        east = east.saturating_add(EXPLORE_BIAS);
                    }
                    (here, west, east)
                }
            }
        };
        let sense_y = |o: &Org| -> (u16, u16, u16) {
            match o.species {
                Species::Grazer => {
                    let here = food_snapshot[o.pos];
                    let north = food_snapshot[world.neighbor_index(o.pos, 0, -1)];
                    let south = food_snapshot[world.neighbor_index(o.pos, 0, 1)];
                    (here, north, south)
                }
                Species::Predator => {
                    let here = prey_index.energy_at(o.pos);
                    let mut north = prey_index.energy_at(world.neighbor_index(o.pos, 0, -1));
                    let mut south = prey_index.energy_at(world.neighbor_index(o.pos, 0, 1));
                    if explore_phase == 2 {
                        north = north.saturating_add(EXPLORE_BIAS);
                    } else if explore_phase == 3 {
                        south = south.saturating_add(EXPLORE_BIAS);
                    }
                    (here, north, south)
                }
            }
        };
        let senses_x: Vec<(u16, u16, u16)> = orgs.iter().map(sense_x).collect();
        let senses_y: Vec<(u16, u16, u16)> = orgs.iter().map(sense_y).collect();

        // Stage 1: decay — fixed/shared cell, species-agnostic.
        let decay_in: Vec<[u16; 3]> = orgs
            .iter()
            .map(|o| [o.energy, o.genome.decay_amount, 0])
            .collect();
        let decay_out = batch_run(engine, &pools.decay, &decay_in);

        // Stage 2/3: sense_move, once per axis — one shared pool, one batched dispatch per
        // axis covering both species; only the *inputs* (already computed above) differ by
        // species, matching `ex1.rs`'s axis-decomposition discipline exactly.
        let sense_role_idx: Vec<u16> = orgs.iter().map(|o| o.genome.sense_move).collect();
        let sense_x_in: Vec<[u16; 3]> = senses_x.iter().map(|&(h, w, e)| [h, w, e]).collect();
        let action_x = batch_run_grouped(engine, &pools.sense_pool, &sense_role_idx, &sense_x_in);
        let sense_y_in: Vec<[u16; 3]> = senses_y.iter().map(|&(h, n, s)| [h, n, s]).collect();
        let action_y = batch_run_grouped(engine, &pools.sense_pool, &sense_role_idx, &sense_y_in);

        // Stage 4: hungry_promoter — grazer: "hungry enough to eat"; predator: "attack now"
        // (the exact reuse `main.rs` established — no new gene role for predation).
        let hungry_role_idx: Vec<u16> = orgs.iter().map(|o| o.genome.hungry_promoter).collect();
        let hungry_in: Vec<[u16; 3]> = senses_x.iter().map(|&(h, _, _)| [h, 0, 0]).collect();
        let hungry_out = batch_run_grouped(engine, &pools.hungry_pool, &hungry_role_idx, &hungry_in);

        // Stage 5: eat — grazer: food_here becomes energy; predator: prey_here (the
        // victim's energy, per `PreyIndex`) becomes energy. Computed uniformly for
        // everyone; contention (below) decides who actually applies it.
        let eat_in: Vec<[u16; 3]> = senses_x
            .iter()
            .zip(&decay_out)
            .map(|(&(h, _, _), &(e1, _))| [e1, h, 0])
            .collect();
        let eat_out = batch_run(engine, &pools.eat, &eat_in);

        // Resolve: apply decay + the axis-priority movement rule (identical control flow
        // to ex1.rs/ex2.rs); build both species' contention candidate lists in the same
        // pass — the structural condition (both axes say "stay" and hungry_promoter fires)
        // is shared, only what the candidate *means* differs by species.
        let mut eat_candidates: Vec<(u32, usize)> = Vec::new();
        let mut predation_candidates: Vec<(u32, u32)> = Vec::new(); // (predator_id, victim_id)
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
                    match o.species {
                        Species::Grazer => eat_candidates.push((o.id, o.pos)),
                        Species::Predator => {
                            // Satiated predators can still sense/stay (movement is
                            // unaffected) but don't register as an attacker this tick.
                            if tick >= o.satiation_until {
                                if let Some((victim_id, _)) = prey_index.at(o.pos) {
                                    predation_candidates.push((o.id, victim_id));
                                }
                            }
                        }
                    }
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

        // Grazer eat-tile contention — exactly ex1.rs/ex2.rs's mechanism, unchanged.
        let eat_winners = contention::resolve_eat_contention(cfg.seed, tick, &eat_candidates);
        total_contention_losses += (eat_candidates.len() - eat_winners.len()) as u32;
        let mut ate_positions: Vec<usize> = Vec::new();
        for (i, o) in orgs.iter().enumerate() {
            if o.species == Species::Grazer && eat_winners.contains(&o.id) {
                resolved_energy[i] = eat_out[i].0;
                ate_positions.push(o.pos);
            }
        }
        for pos in ate_positions {
            world.eat_at(pos);
        }

        // Predation-kill contention — the design doc's "tournament dispatch," the same
        // order-independent mechanism keyed by victim id instead of tile position.
        let predation_winners =
            contention::resolve_contention(cfg.seed, tick, &predation_candidates, contention::PREDATION_CONTENTION_STREAM);
        let killed_victims = killed_victims_from(&predation_candidates, &predation_winners);
        total_predation_kills += killed_victims.len() as u32;
        for (i, o) in orgs.iter_mut().enumerate() {
            if o.species == Species::Predator && predation_winners.contains(&o.id) {
                resolved_energy[i] = eat_out[i].0;
                o.satiation_until = tick + cfg.predator_satiation_ticks;
            }
        }
        // A predation kill overrides everything else that tick for the victim — no eat, no
        // reproduction, simply removed (a deliberate clarification from `main.rs`'s own
        // order-dependent accident; see the module doc).
        for (i, o) in orgs.iter().enumerate() {
            if killed_victims.contains(&o.id) {
                resolved_energy[i] = 0;
            }
        }

        // Starvation (natural, or a predation kill applied above — same removal pathway).
        let mut survivors: Vec<usize> = Vec::new();
        for (i, &energy) in resolved_energy.iter().enumerate() {
            if energy == 0 {
                total_starved += 1;
            } else {
                orgs[i].energy = energy;
                survivors.push(i);
            }
        }

        // repro_promoter / split — species-agnostic (`main.rs`'s own framing), survivors only.
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
                let child_genome = if cfg.mutation_enabled {
                    mutate(
                        cfg.seed, tick, id, &orgs[i].genome,
                        hungry_pool_len, repro_pool_len, sense_pool_len,
                        overrides.and_then(|o| o.get(&id)),
                    )
                } else {
                    orgs[i].genome.clone()
                };
                all_births.push(BirthEventEco {
                    child_id: id,
                    parent_id: orgs[i].id,
                    tick,
                    species: orgs[i].species,
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
                    species: orgs[i].species,
                    genome: child_genome,
                    satiation_until: 0,
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
            species: Species::Grazer,
            genome: grazer_genome.clone(),
            satiation_until: 0,
        };
        let mut new_orgs: Vec<Org> = survivors
            .into_iter()
            .map(|i| std::mem::replace(&mut orgs[i], placeholder()))
            .collect();
        new_orgs.extend(children);
        orgs = new_orgs;

        world.tick_regrow();

        let mut snap: Vec<OrgSnapshot2DEco> = orgs
            .iter()
            .map(|o| {
                let (x, y) = world.xy(o.pos);
                OrgSnapshot2DEco {
                    id: o.id,
                    x: x as u16,
                    y: y as u16,
                    energy: o.energy,
                    species: o.species,
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

        let record = TickRecord2DEco {
            tick,
            organisms: snap,
            food: world.food.clone(),
            births: total_births,
            starved: total_starved,
            contention_losses: total_contention_losses,
            predation_kills: total_predation_kills,
            total_ir_steps,
        };
        hasher.absorb2d_eco(&record);
        records.push(record);
    }

    let final_grazers = orgs.iter().filter(|o| o.species == Species::Grazer).count();
    let final_predators = orgs.iter().filter(|o| o.species == Species::Predator).count();

    RunOutput3 {
        history_hash: hasher.finish(),
        ticks: records,
        births: all_births,
        final_population: orgs.len(),
        final_grazers,
        final_predators,
        total_births,
        total_starved,
        total_predation_kills,
    }
}

#[cfg(test)]
mod ordering_tests {
    use super::*;
    use std::collections::HashMap;

    /// Two predators (10, 11) both target victim grazer 99 this tick; grazer 99
    /// simultaneously "wins" its own eat-tile contention (it intended to eat food too, on an
    /// uncontested tile). The kill must still apply — a predation kill overrides everything
    /// else that tick for the victim, independent of whatever else it resolved to. This pins
    /// down the one place a predator/victim id mix-up could hide (`killed_victims_from`),
    /// plus the "would-have-eaten" override the design doc calls out as a deliberate
    /// clarification from `main.rs`'s order-dependent original.
    #[test]
    fn predation_kill_overrides_regardless_of_eat_contention_outcome() {
        let seed = 0xE3_5EED;
        let tick = 7;

        let eat_candidates: Vec<(u32, usize)> = vec![(99, 4)];
        let eat_winners = contention::resolve_eat_contention(seed, tick, &eat_candidates);
        assert!(eat_winners.contains(&99), "sanity: uncontested tile, grazer 99 wins its own eat");

        let predation_candidates: Vec<(u32, u32)> = vec![(10, 99), (11, 99)];
        let predation_winners = contention::resolve_contention(
            seed,
            tick,
            &predation_candidates,
            contention::PREDATION_CONTENTION_STREAM,
        );
        assert_eq!(predation_winners.len(), 1, "exactly one predator wins the contested victim");

        let killed = killed_victims_from(&predation_candidates, &predation_winners);
        assert!(killed.contains(&99), "victim must be marked killed regardless of its own eat outcome");
        assert_eq!(killed.len(), 1);

        // The override itself: even though grazer 99 won its eat-tile contention above, a
        // killed organism's resolved energy is forced to 0 (no eat, no reproduction).
        let mut resolved_energy: HashMap<u32, u16> = HashMap::from([(99u32, 500u16)]);
        for id in &killed {
            resolved_energy.insert(*id, 0);
        }
        assert_eq!(resolved_energy[&99], 0);
    }

    #[test]
    fn uncontested_predation_kills_its_one_victim() {
        let predation_candidates: Vec<(u32, u32)> = vec![(1, 50)];
        let winners = std::collections::HashSet::from([1u32]);
        let killed = killed_victims_from(&predation_candidates, &winners);
        assert_eq!(killed, std::collections::HashSet::from([50u32]));
    }

    #[test]
    fn losing_predators_kill_nobody() {
        let predation_candidates: Vec<(u32, u32)> = vec![(1, 50), (2, 50)];
        let winners = std::collections::HashSet::from([1u32]); // predator 2 lost the contention
        let killed = killed_victims_from(&predation_candidates, &winners);
        assert_eq!(killed, std::collections::HashSet::from([50u32]), "still exactly one dead victim");
    }
}
