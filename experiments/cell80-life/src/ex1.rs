//! EX-1's 2D tick engine (`experiments/deterministic-ecology.md`): the same decision/resolve
//! discipline as `ex0.rs`, ported to a toroidal `World2D`, with a genuinely 2D movement
//! decision built from the genome's existing, unmodified `sense_move` cell (`argmax3`)
//! called twice per tick — once per axis — rather than authoring a new 4/5-way selector.
//!
//! Why axis decomposition, not a real 5-way (stay/N/S/E/W) cell: every arity-4+
//! ranking/selection cell in the stdlib (`argmax4`, `argmax4_u32`, `argmin4`,
//! `choose_best4`) is a **state cell** (`&mut self`, zero conventional args), and
//! `rustmsl::codegen::IN_STRIDE = 3` hard-caps a plain-function GPU batch cell at 3 scalar
//! inputs anyway. There is no plain-function 4/5-way selector to reuse, and authoring one
//! would itself violate "port the genomes unchanged" — so `sense_move` (`argmax3` in both
//! `grazer.json` and `rapid_reproducer.json`, completely unmodified) is simply called
//! twice: once against `(food_here, food_west, food_east)`, once against `(food_here,
//! food_north, food_south)`.
//!
//! Movement resolution: `argmax3`'s own doc says ties go to the lowest index ("stay"), so a
//! real move is always a *strict* improvement over staying — meaning "does this axis want
//! to move" and "by how much" are the same question. If neither axis wants to move, the
//! organism attempts to eat (contention-resolved exactly as in `ex0.rs`, via
//! `contention::resolve_eat_contention` — shared, not duplicated). If exactly one axis
//! wants to move, it moves on that axis. If both do, it moves on whichever axis has the
//! larger sensed food differential (free — no new cell, no new GPU work, just comparing
//! values already computed this tick), falling back to a fixed X-priority only on an exact
//! tie — a named, minor anisotropy, not a hidden one (see the findings doc for the
//! oscillator-rate diagnostic that checks whether this rule gets organisms stuck
//! ping-ponging between tiles).
use std::collections::HashMap;

use crate::contention;
use crate::genes::{batch_run, sum_steps, EngineKind, GeneSet};
use crate::history::{HistoryHasher, OrgSnapshot2D, TickRecord2D};
use crate::rng::{self, MUTATION_STREAM};
use crate::world2d::World2D;
use crate::StartingGenome;

pub struct RunConfig2D {
    pub seed: u64,
    pub ticks: u32,
    pub initial_organisms: usize,
    pub world_width: usize,
    pub world_height: usize,
    pub food_density: f64,
    pub food_value: u16,
    pub regrow_ticks: u16,
}

pub struct RunOutput2D {
    pub history_hash: [u8; 32],
    pub ticks: Vec<TickRecord2D>,
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
    cfg: &RunConfig2D,
    genome: &StartingGenome,
    genes: &GeneSet,
) -> RunOutput2D {
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
    let mut orgs: Vec<Org> = (0..cfg.initial_organisms)
        .map(|i| {
            let id = next_id;
            next_id += 1;
            Org {
                id,
                pos: (i * world_len / n0) % world_len,
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

        // Stage 1: decay.
        let decay_in: Vec<[u16; 3]> = orgs
            .iter()
            .map(|o| [o.energy, genome.decay_amount, 0])
            .collect();
        let decay_out = batch_run(engine, &genes.decay, &decay_in);

        // Stage 2/3: sense_move, once per axis — the genome's one unmodified cell, called
        // twice with different inputs (see module doc).
        let sense_x_in: Vec<[u16; 3]> = senses_x.iter().map(|&(h, w, e)| [h, w, e]).collect();
        let action_x = batch_run(engine, &genes.sense_move, &sense_x_in);
        let sense_y_in: Vec<[u16; 3]> = senses_y.iter().map(|&(h, n, s)| [h, n, s]).collect();
        let action_y = batch_run(engine, &genes.sense_move, &sense_y_in);

        // Stage 4: hungry_promoter, computed for everyone uniformly (applied only where
        // both axes say "stay").
        let hungry_in: Vec<[u16; 3]> = senses_x.iter().map(|&(h, _, _)| [h, 0, 0]).collect();
        let hungry_out = batch_run(engine, &genes.hungry_promoter, &hungry_in);

        // Stage 5: eat, against post-decay energy.
        let eat_in: Vec<[u16; 3]> = senses_x
            .iter()
            .zip(&decay_out)
            .map(|(&(h, _, _), &(e1, _))| [e1, h, 0])
            .collect();
        let eat_out = batch_run(engine, &genes.eat, &eat_in);

        // Resolve: apply decay + the axis-priority movement rule now; an eat-intent (both
        // axes say "stay" and hungry_promoter passes) only becomes a contention candidate.
        let mut eat_candidates: Vec<(u32, usize)> = Vec::new();
        let mut resolved_energy = vec![0u16; orgs.len()];
        for (i, o) in orgs.iter_mut().enumerate() {
            resolved_energy[i] = decay_out[i].0;
            let ax = action_x[i].0;
            let ay = action_y[i].0;
            let (h_x, w, e) = senses_x[i];
            let (h_y, n, s) = senses_y[i];
            // argmax3 ties go to index 0 ("stay"), so a nonzero action is always a strict
            // improvement — diff is 0 exactly when that axis doesn't want to move.
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
        // branching anything (EX-1 is single-genome; see the design doc for EX-2).
        let mutation_draws: Vec<(u32, u32)> = survivors
            .iter()
            .map(|&i| {
                (
                    orgs[i].id,
                    rng::draw(cfg.seed, tick, orgs[i].id, MUTATION_STREAM),
                )
            })
            .collect();

        // repro_promoter / split — survivors only.
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
                // Toroidal wraparound means a birth placement is always defined — unlike
                // the 1D `World`'s edge-aware fallback, no boundary case to handle here.
                let child_pos = world.neighbor_index(orgs[i].pos, 1, 0);
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
            &action_x,
            &action_y,
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
        let mut snap: Vec<OrgSnapshot2D> = orgs
            .iter()
            .map(|o| {
                let (x, y) = world.xy(o.pos);
                OrgSnapshot2D {
                    id: o.id,
                    x: x as u16,
                    y: y as u16,
                    energy: o.energy,
                }
            })
            .collect();
        snap.sort_by_key(|s| s.id);
        let mut draws = mutation_draws;
        draws.sort_by_key(|&(id, _)| id);

        let record = TickRecord2D {
            tick,
            organisms: snap,
            mutation_draws: draws,
            food: world.food.clone(),
            births: total_births,
            starved: total_starved,
            contention_losses: total_contention_losses,
            total_ir_steps,
        };
        hasher.absorb2d(&record);
        records.push(record);
    }

    RunOutput2D {
        history_hash: hasher.finish(),
        ticks: records,
        final_population: orgs.len(),
        births: total_births,
        starved: total_starved,
    }
}

/// Fraction of organisms alive for at least `window` consecutive ticks whose position
/// shows a period-2 cycle (`pos[t] == pos[t+2]` and `pos[t] != pos[t+1]` across the whole
/// window) — a direct check for the "does axis-decomposed movement get an organism stuck
/// ping-ponging between two tiles" risk named in the module doc, not just an assumption
/// that it doesn't happen. Only meaningful on a run that retained full `TickRecord2D`
/// history (small-scale/calibration runs, not the large sweep's lightweight summaries).
pub fn oscillator_rate(output: &RunOutput2D, window: usize) -> f64 {
    if window < 3 || output.ticks.len() < window {
        return 0.0;
    }
    let tail = &output.ticks[output.ticks.len() - window..];

    let mut per_id: HashMap<u32, Vec<(u16, u16)>> = HashMap::new();
    for rec in tail {
        for o in &rec.organisms {
            per_id.entry(o.id).or_default().push((o.x, o.y));
        }
    }
    // Only ids present at every tick in the window — an id can't reappear after death, so
    // "present in all `window` ticks" means a contiguous, uninterrupted lifetime here.
    let long_lived: Vec<&Vec<(u16, u16)>> = per_id
        .values()
        .filter(|positions| positions.len() == window)
        .collect();
    if long_lived.is_empty() {
        return 0.0;
    }
    let oscillating = long_lived
        .iter()
        .filter(|positions| {
            (0..positions.len() - 2)
                .all(|t| positions[t] == positions[t + 2] && positions[t] != positions[t + 1])
        })
        .count();
    oscillating as f64 / long_lived.len() as f64
}
