//! EX-3: a position-keyed lookup for prey sensing (`experiments/deterministic-ecology.md`),
//! built once per tick from the same immutable tick-start snapshot everything else in this
//! engine reads sensing from. Replaces `main.rs`'s original `prey_at` — an O(n) linear scan
//! *per lookup*, ~5 lookups/predator/tick (here + 4 neighbors in 2D) — which would cost
//! O(n_predators × n_total) per tick: plausibly seconds at EX-1's demonstrated 10⁴–10⁵
//! scale, dominating everything else by orders of magnitude. `PreyIndex` instead does one
//! O(n) build pass per tick, then O(1) lookups — not a general spatial index (no radius
//! beyond 0/1 is ever queried here), just the minimum needed to avoid the naive-port cost.
use std::collections::HashMap;

/// One tile's occupant, for prey-sensing purposes: the lowest-id grazer at that position — a
/// deliberate, explicit tie-break for co-located grazers. `main.rs`'s original `prey_at`
/// picked whichever its `Vec` scan happened to reach first, an accident of iteration order,
/// not a designed choice; this makes the same kind of decision `contention.rs` already makes
/// explicit rather than inheriting an incidental one.
pub struct PreyIndex {
    by_pos: HashMap<usize, (u32, u16)>,
}

impl PreyIndex {
    /// Build from a tick-start snapshot: every living grazer's `(id, pos, energy)`. Callers
    /// pass only grazers — `Species::Grazer` is the only valid prey, the fix `main.rs`'s own
    /// history already established for the "predators sensing each other" bug, so this type
    /// doesn't need to know about `Species` at all.
    pub fn build(grazers: impl Iterator<Item = (u32, usize, u16)>) -> Self {
        let mut by_pos: HashMap<usize, (u32, u16)> = HashMap::new();
        for (id, pos, energy) in grazers {
            by_pos
                .entry(pos)
                .and_modify(|(cur_id, cur_energy)| {
                    if id < *cur_id {
                        *cur_id = id;
                        *cur_energy = energy;
                    }
                })
                .or_insert((id, energy));
        }
        PreyIndex { by_pos }
    }

    /// `(victim_id, victim_energy)` at `pos`, if a grazer occupies it.
    pub fn at(&self, pos: usize) -> Option<(u32, u16)> {
        self.by_pos.get(&pos).copied()
    }

    /// Just the energy, for sensing (0 if no prey there) — matches `main.rs`'s
    /// `prey_at(...).map_or(0, |(_, e)| e)` convention.
    pub fn energy_at(&self, pos: usize) -> u16 {
        self.by_pos.get(&pos).map_or(0, |&(_, e)| e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_index_senses_nothing() {
        let idx = PreyIndex::build(std::iter::empty());
        assert_eq!(idx.at(5), None);
        assert_eq!(idx.energy_at(5), 0);
    }

    #[test]
    fn single_grazer_is_found() {
        let idx = PreyIndex::build([(3u32, 10usize, 50u16)].into_iter());
        assert_eq!(idx.at(10), Some((3, 50)));
        assert_eq!(idx.energy_at(10), 50);
        assert_eq!(idx.at(11), None);
    }

    #[test]
    fn co_located_grazers_resolve_to_lowest_id_regardless_of_insertion_order() {
        let a = [
            (9u32, 4usize, 10u16),
            (2u32, 4usize, 99u16),
            (5u32, 4usize, 42u16),
        ];
        let b = [
            (5u32, 4usize, 42u16),
            (2u32, 4usize, 99u16),
            (9u32, 4usize, 10u16),
        ];
        let idx_a = PreyIndex::build(a.into_iter());
        let idx_b = PreyIndex::build(b.into_iter());
        assert_eq!(idx_a.at(4), Some((2, 99)));
        assert_eq!(idx_b.at(4), Some((2, 99)));
    }

    #[test]
    fn distinct_positions_dont_interfere() {
        let idx = PreyIndex::build([(1u32, 0usize, 10u16), (2u32, 1usize, 20u16)].into_iter());
        assert_eq!(idx.at(0), Some((1, 10)));
        assert_eq!(idx.at(1), Some((2, 20)));
    }
}
