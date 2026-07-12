//! Per-tick history recording for EX-0: a canonical, fixed-width byte encoding (not
//! JSON/serde, so the encoding itself can't introduce formatting nondeterminism) folded
//! into a running SHA-256 — the single "history hash" two runs (same seed twice, or
//! CPU-reference vs GPU) are compared by.
use sha2::{Digest, Sha256};

/// One living organism as of a tick's end, canonically ordered by `id` — never by `Vec`
/// position (which shifts as organisms die/are born) or any `HashMap` iteration order
/// (which `cell80-life`'s own `genome_stats` shows is a live risk: `HashMap::max_by_key`'s
/// tie-break depends on `RandomState`'s per-process seed).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrgSnapshot {
    pub id: u32,
    pub pos: u16,
    pub energy: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TickRecord {
    pub tick: u32,
    /// Every living organism, sorted by `id`.
    pub organisms: Vec<OrgSnapshot>,
    /// `(organism_id, draw)` pairs, sorted by id — the mutation-decision RNG draw taken
    /// this tick per survivor (computed and recorded, not yet branching anything; see the
    /// design doc's EX-0 scope).
    pub mutation_draws: Vec<(u32, u32)>,
    pub food: Vec<u16>,
    /// Cumulative counts as of this tick (matches the original binary's semantics).
    pub births: u32,
    pub starved: u32,
    /// Cumulative count of organisms that lost a contested eat-tile this tick
    /// (`contention::resolve_eat_contention`'s losers) — a direct receipt for how often the
    /// mechanic actually engages, not just that it exists.
    pub contention_losses: u32,
    /// Summed IR-step cost of every gene call this tick, across all organisms and roles —
    /// an aggregate, not a per-organism-per-role breakdown (a stated simplification; see
    /// the findings doc). Any GPU/interpreter step-count divergence still changes this
    /// number even if it happens not to change any organism's final energy/position.
    pub total_ir_steps: u64,
}

pub struct HistoryHasher(Sha256);

impl Default for HistoryHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl HistoryHasher {
    pub fn new() -> Self {
        HistoryHasher(Sha256::new())
    }

    pub fn absorb(&mut self, rec: &TickRecord) {
        self.0.update(rec.tick.to_le_bytes());
        self.0.update((rec.organisms.len() as u32).to_le_bytes());
        for o in &rec.organisms {
            self.0.update(o.id.to_le_bytes());
            self.0.update(o.pos.to_le_bytes());
            self.0.update(o.energy.to_le_bytes());
        }
        self.0
            .update((rec.mutation_draws.len() as u32).to_le_bytes());
        for (id, draw) in &rec.mutation_draws {
            self.0.update(id.to_le_bytes());
            self.0.update(draw.to_le_bytes());
        }
        for f in &rec.food {
            self.0.update(f.to_le_bytes());
        }
        self.0.update(rec.births.to_le_bytes());
        self.0.update(rec.starved.to_le_bytes());
        self.0.update(rec.contention_losses.to_le_bytes());
        self.0.update(rec.total_ir_steps.to_le_bytes());
    }

    pub fn finish(self) -> [u8; 32] {
        self.0.finalize().into()
    }

    /// EX-1's 2D counterpart to `absorb` — kept as a separate method (not an overload) so
    /// `absorb`/`TickRecord`/`OrgSnapshot` stay untouched for EX-0's existing tests.
    pub fn absorb2d(&mut self, rec: &TickRecord2D) {
        self.0.update(rec.tick.to_le_bytes());
        self.0.update((rec.organisms.len() as u32).to_le_bytes());
        for o in &rec.organisms {
            self.0.update(o.id.to_le_bytes());
            self.0.update(o.x.to_le_bytes());
            self.0.update(o.y.to_le_bytes());
            self.0.update(o.energy.to_le_bytes());
        }
        self.0
            .update((rec.mutation_draws.len() as u32).to_le_bytes());
        for (id, draw) in &rec.mutation_draws {
            self.0.update(id.to_le_bytes());
            self.0.update(draw.to_le_bytes());
        }
        for f in &rec.food {
            self.0.update(f.to_le_bytes());
        }
        self.0.update(rec.births.to_le_bytes());
        self.0.update(rec.starved.to_le_bytes());
        self.0.update(rec.contention_losses.to_le_bytes());
        self.0.update(rec.total_ir_steps.to_le_bytes());
    }

    /// EX-2's counterpart to `absorb2d` — `absorb`/`absorb2d` and their `TickRecord` types
    /// stay untouched, so EX-0/EX-1's existing hash contracts don't move.
    pub fn absorb2d_genome(&mut self, rec: &TickRecord2DGenome) {
        self.0.update(rec.tick.to_le_bytes());
        self.0.update((rec.organisms.len() as u32).to_le_bytes());
        for o in &rec.organisms {
            self.0.update(o.id.to_le_bytes());
            self.0.update(o.x.to_le_bytes());
            self.0.update(o.y.to_le_bytes());
            self.0.update(o.energy.to_le_bytes());
            self.0.update(o.decay_amount.to_le_bytes());
            self.0.update(o.repro_threshold.to_le_bytes());
            self.0.update(o.repro_give_pct.to_le_bytes());
            self.0.update(o.hungry_promoter.to_le_bytes());
            self.0.update(o.repro_promoter.to_le_bytes());
            self.0.update(o.sense_move.to_le_bytes());
        }
        for f in &rec.food {
            self.0.update(f.to_le_bytes());
        }
        self.0.update(rec.births.to_le_bytes());
        self.0.update(rec.starved.to_le_bytes());
        self.0.update(rec.contention_losses.to_le_bytes());
        self.0.update(rec.total_ir_steps.to_le_bytes());
    }

    /// EX-3's counterpart to `absorb2d_genome` — `absorb`/`absorb2d`/`absorb2d_genome` and
    /// their `TickRecord` types stay untouched, so EX-0/EX-1/EX-2's existing hash contracts
    /// don't move.
    pub fn absorb2d_eco(&mut self, rec: &TickRecord2DEco) {
        self.0.update(rec.tick.to_le_bytes());
        self.0.update((rec.organisms.len() as u32).to_le_bytes());
        for o in &rec.organisms {
            self.0.update(o.id.to_le_bytes());
            self.0.update(o.x.to_le_bytes());
            self.0.update(o.y.to_le_bytes());
            self.0.update(o.energy.to_le_bytes());
            self.0.update([match o.species {
                Species::Grazer => 0u8,
                Species::Predator => 1u8,
            }]);
            self.0.update(o.decay_amount.to_le_bytes());
            self.0.update(o.repro_threshold.to_le_bytes());
            self.0.update(o.repro_give_pct.to_le_bytes());
            self.0.update(o.hungry_promoter.to_le_bytes());
            self.0.update(o.repro_promoter.to_le_bytes());
            self.0.update(o.sense_move.to_le_bytes());
        }
        for f in &rec.food {
            self.0.update(f.to_le_bytes());
        }
        self.0.update(rec.births.to_le_bytes());
        self.0.update(rec.starved.to_le_bytes());
        self.0.update(rec.contention_losses.to_le_bytes());
        self.0.update(rec.predation_kills.to_le_bytes());
        self.0.update(rec.total_ir_steps.to_le_bytes());
    }
}

/// EX-1's 2D counterpart to `OrgSnapshot` — two axis coordinates (each safely within a
/// `u16`'s range for any realistic world side length) instead of one flat index, which a
/// world sized to host 10⁴–10⁵ organisms can exceed (e.g. 320×320 = 102,400 tiles > 65,535).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrgSnapshot2D {
    pub id: u32,
    pub x: u16,
    pub y: u16,
    pub energy: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TickRecord2D {
    pub tick: u32,
    /// Every living organism, sorted by `id`.
    pub organisms: Vec<OrgSnapshot2D>,
    /// `(organism_id, draw)` pairs, sorted by id.
    pub mutation_draws: Vec<(u32, u32)>,
    pub food: Vec<u16>,
    pub births: u32,
    pub starved: u32,
    pub contention_losses: u32,
    pub total_ir_steps: u64,
}

/// EX-2's counterpart to `OrgSnapshot2D` — a per-organism genome now varies across the
/// population, so a snapshot carries it too: numeric fields plus pool-index cell choices
/// for the three swappable roles (`decay`/`eat`/`split` stay fixed/shared, so aren't
/// per-organism state to snapshot).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrgSnapshot2DGenome {
    pub id: u32,
    pub x: u16,
    pub y: u16,
    pub energy: u16,
    pub decay_amount: u16,
    pub repro_threshold: u16,
    pub repro_give_pct: u16,
    pub hungry_promoter: u16,
    pub repro_promoter: u16,
    pub sense_move: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TickRecord2DGenome {
    pub tick: u32,
    /// Every living organism, sorted by `id`.
    pub organisms: Vec<OrgSnapshot2DGenome>,
    pub food: Vec<u16>,
    pub births: u32,
    pub starved: u32,
    pub contention_losses: u32,
    pub total_ir_steps: u64,
}

/// A single birth event — EX-2's mutation record. Deliberately light (post-mutation role
/// indices and numeric fields, not a diff against the parent) so every birth can be logged
/// without per-tick full-snapshot overhead; a diff against `parent_id`'s own recorded event
/// is a simple lookup, not something this event needs to precompute. Reusable by EX-4's
/// lineage instrumentation later, not duplicated there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BirthEvent {
    pub child_id: u32,
    pub parent_id: u32,
    pub tick: u32,
    pub decay_amount: u16,
    pub repro_threshold: u16,
    pub repro_give_pct: u16,
    pub hungry_promoter: u16,
    pub repro_promoter: u16,
    pub sense_move: u16,
}

/// EX-3: a structurally different pipeline (which world query feeds `sense_move`/
/// `hungry_promoter`, and how the result gets interpreted) — not just different parameter
/// values, and not itself part of the mutable genome. Fixed per lineage, copied verbatim
/// from parent to child, never touched by `mutate()`; matches `main.rs`'s own discipline
/// (only numeric thresholds and role-cell choices evolve within a species).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Species {
    Grazer,
    Predator,
}

/// EX-3's counterpart to `OrgSnapshot2DGenome` — adds `species`, otherwise identical.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrgSnapshot2DEco {
    pub id: u32,
    pub x: u16,
    pub y: u16,
    pub energy: u16,
    pub species: Species,
    pub decay_amount: u16,
    pub repro_threshold: u16,
    pub repro_give_pct: u16,
    pub hungry_promoter: u16,
    pub repro_promoter: u16,
    pub sense_move: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TickRecord2DEco {
    pub tick: u32,
    /// Every living organism, sorted by `id` — both species interleaved, since id is the
    /// one global, cross-species order.
    pub organisms: Vec<OrgSnapshot2DEco>,
    pub food: Vec<u16>,
    pub births: u32,
    pub starved: u32,
    pub contention_losses: u32,
    /// Cumulative count of prey killed by a predator this run — the receipt analogue of
    /// `contention_losses`: how often predation actually engages, not just that it exists.
    pub predation_kills: u32,
    pub total_ir_steps: u64,
}

/// EX-3's counterpart to `BirthEvent` — adds `species` (inherited, never mutated).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BirthEventEco {
    pub child_id: u32,
    pub parent_id: u32,
    pub tick: u32,
    pub species: Species,
    pub decay_amount: u16,
    pub repro_threshold: u16,
    pub repro_give_pct: u16,
    pub hungry_promoter: u16,
    pub repro_promoter: u16,
    pub sense_move: u16,
}
