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
        self.0.update(rec.total_ir_steps.to_le_bytes());
    }

    pub fn finish(self) -> [u8; 32] {
        self.0.finalize().into()
    }
}
