//! EX-0's tick engine (`experiments/deterministic-ecology.md`): the decision/resolve split
//! that makes a CPU-reference run and a GPU run of the same world comparable byte-for-byte.
//!
//! Every gene call this tick is computed against an immutable tick-start snapshot (each
//! organism's own energy/position, plus the world's food array as of tick start) — never
//! against another organism's this-tick output — so results don't depend on organism
//! processing order. That's what makes it safe to run each stage as a single N-wide batch
//! dispatch (a GPU dispatch's thread order is undefined) instead of the original binary's
//! sequential per-organism loop.
//!
//! Deliberately scoped narrow (see the design doc): one homogeneous genome for the whole
//! run (no species, no mutation-driven genome diversity — that's EX-2/EX-3), the existing
//! 1D world (2D/scale is EX-1's job). A contested eat-tile (more than one organism
//! intending to eat there this tick) is resolved by `contention::resolve_eat_contention` —
//! an order-independent, RNG-picked single winner gets the real `eat` output; everyone
//! else keeps their post-decay energy. "Who processed first" isn't a well-posed question
//! once a tick is a batch dispatch rather than a `Vec` loop, so this replaces
//! `cell80-life`'s sequential first-wins rule; an earlier version of this file granted
//! food to every contestant uncontested, which caused a population explosion — see the
//! findings doc.
use crate::contention;
pub use crate::genes::EngineKind;
use crate::genes::{batch_run, sum_steps, GeneSet};
use crate::history::{HistoryHasher, OrgSnapshot, TickRecord};
use crate::rng::{self, MUTATION_STREAM};
use crate::{StartingGenome, World};

pub struct RunConfig {
    pub seed: u64,
    pub ticks: u32,
    pub initial_organisms: usize,
    pub world_len: usize,
    pub food_value: u16,
    pub regrow_ticks: u16,
}

pub struct RunOutput {
    pub history_hash: [u8; 32],
    pub ticks: Vec<TickRecord>,
    pub final_population: usize,
    pub births: u32,
    pub starved: u32,
}

struct Org {
    id: u32,
    pos: usize,
    energy: u16,
}

pub fn run(
    engine: EngineKind,
    cfg: &RunConfig,
    genome: &StartingGenome,
    genes: &GeneSet,
) -> RunOutput {
    let mut world = World::new(cfg.world_len, cfg.food_value, cfg.regrow_ticks);
    let mut next_id: u32 = 0;
    let n0 = cfg.initial_organisms.max(1);
    let mut orgs: Vec<Org> = (0..cfg.initial_organisms)
        .map(|i| {
            let id = next_id;
            next_id += 1;
            Org {
                id,
                pos: i * cfg.world_len / n0,
                energy: genome.initial_energy,
            }
        })
        .collect();

    let mut hasher = HistoryHasher::new();
    let mut records = Vec::with_capacity(cfg.ticks as usize);
    let mut total_births = 0u32;
    let mut total_starved = 0u32;
    let mut total_contention_losses = 0u32;

    for tick in 0..cfg.ticks {
        if orgs.is_empty() {
            break;
        }
        let food_snapshot = world.food.clone();
        let sense_at = |pos: usize| -> (u16, u16, u16) {
            let here = food_snapshot[pos];
            let left = if pos > 0 { food_snapshot[pos - 1] } else { 0 };
            let right = if pos + 1 < food_snapshot.len() {
                food_snapshot[pos + 1]
            } else {
                0
            };
            (here, left, right)
        };
        let senses: Vec<(u16, u16, u16)> = orgs.iter().map(|o| sense_at(o.pos)).collect();

        // Stage 1: decay.
        let decay_in: Vec<[u16; 3]> = orgs
            .iter()
            .map(|o| [o.energy, genome.decay_amount, 0])
            .collect();
        let decay_out = batch_run(engine, &genes.decay, &decay_in);

        // Stage 2: sense_move — action: 0 = eat here, 1 = move left, 2 = move right.
        let sense_in: Vec<[u16; 3]> = senses.iter().map(|&(h, l, r)| [h, l, r]).collect();
        let sense_out = batch_run(engine, &genes.sense_move, &sense_in);

        // Stage 3: hungry_promoter, computed for everyone uniformly (applied only where
        // action == 0) — keeps every stage's batch shape the same N as the population,
        // rather than compacting to a variable-size subset mid-tick.
        let hungry_in: Vec<[u16; 3]> = senses.iter().map(|&(h, _, _)| [h, 0, 0]).collect();
        let hungry_out = batch_run(engine, &genes.hungry_promoter, &hungry_in);

        // Stage 4: eat, against post-decay energy — applied only where action == 0 &&
        // hungry.
        let eat_in: Vec<[u16; 3]> = senses
            .iter()
            .zip(&decay_out)
            .map(|(&(h, _, _), &(e1, _))| [e1, h, 0])
            .collect();
        let eat_out = batch_run(engine, &genes.eat, &eat_in);

        // Resolve stages 1-4: apply decay and movement now; an eat-intent only becomes a
        // candidate here — whether it actually lands depends on contention resolution
        // below, since more than one organism can intend to eat the same tile this tick.
        let mut eat_candidates: Vec<(u32, usize)> = Vec::new();
        let mut resolved_energy = vec![0u16; orgs.len()];
        for (i, o) in orgs.iter_mut().enumerate() {
            let action = sense_out[i].0;
            resolved_energy[i] = decay_out[i].0;
            match action {
                0 if hungry_out[i].0 == 1 => eat_candidates.push((o.id, o.pos)),
                1 if o.pos > 0 => o.pos -= 1,
                2 if o.pos + 1 < world.len() => o.pos += 1,
                _ => {}
            }
        }

        // Exactly one winner per contested tile (order-independent — see contention.rs);
        // everyone else keeps the post-decay energy already recorded above.
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

        // Mutation draws — computed and recorded for every surviving organism; not yet
        // branching anything (EX-0 is single-genome; see the design doc for EX-2, where
        // this starts to decide something).
        let mutation_draws: Vec<(u32, u32)> = survivors
            .iter()
            .map(|&i| {
                (
                    orgs[i].id,
                    rng::draw(cfg.seed, tick, orgs[i].id, MUTATION_STREAM),
                )
            })
            .collect();

        // Stage 5: repro_promoter, stage 6: split — survivors only.
        let repro_in: Vec<[u16; 3]> = survivors
            .iter()
            .map(|&i| [orgs[i].energy, genome.repro_threshold, 0])
            .collect();
        let repro_out = batch_run(engine, &genes.repro_promoter, &repro_in);
        let split_in: Vec<[u16; 3]> = survivors
            .iter()
            .map(|&i| [orgs[i].energy, genome.repro_give_pct, 0])
            .collect();
        let split_out = batch_run(engine, &genes.split, &split_in);

        let mut children: Vec<Org> = Vec::new();
        for (k, &i) in survivors.iter().enumerate() {
            if repro_out[k].0 == 1 {
                let parent_keep = split_out[k].0;
                let child_energy = orgs[i].energy.saturating_sub(parent_keep);
                orgs[i].energy = parent_keep;
                let child_pos = if orgs[i].pos + 1 < world.len() {
                    orgs[i].pos + 1
                } else {
                    orgs[i].pos.saturating_sub(1)
                };
                let id = next_id;
                next_id += 1;
                children.push(Org {
                    id,
                    pos: child_pos,
                    energy: child_energy,
                });
                total_births += 1;
            }
        }

        let total_ir_steps = sum_steps(&[
            &decay_out,
            &sense_out,
            &hungry_out,
            &eat_out,
            &repro_out,
            &split_out,
        ]);

        // Rebuild the living population: survivors (each `orgs[i]` isn't referenced again
        // after this loop, so this just takes ownership rather than cloning) + new children.
        let placeholder = || Org {
            id: 0,
            pos: 0,
            energy: 0,
        };
        let mut new_orgs: Vec<Org> = survivors
            .into_iter()
            .map(|i| std::mem::replace(&mut orgs[i], placeholder()))
            .collect();
        new_orgs.extend(children);
        orgs = new_orgs;

        world.tick_regrow();

        // Record this tick, canonical id-sorted — independent of the `Vec` order above.
        let mut snap: Vec<OrgSnapshot> = orgs
            .iter()
            .map(|o| OrgSnapshot {
                id: o.id,
                pos: o.pos as u16,
                energy: o.energy,
            })
            .collect();
        snap.sort_by_key(|s| s.id);
        let mut draws = mutation_draws;
        draws.sort_by_key(|&(id, _)| id);

        let record = TickRecord {
            tick,
            organisms: snap,
            mutation_draws: draws,
            food: world.food.clone(),
            births: total_births,
            starved: total_starved,
            contention_losses: total_contention_losses,
            total_ir_steps,
        };
        hasher.absorb(&record);
        records.push(record);
    }

    RunOutput {
        history_hash: hasher.finish(),
        ticks: records,
        final_population: orgs.len(),
        births: total_births,
        starved: total_starved,
    }
}
