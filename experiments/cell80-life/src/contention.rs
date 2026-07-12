//! Order-independent resolution of contested resources — currently, an eat-tile with more
//! than one organism intending to eat there this tick. Shared by `ex0.rs` (1D) and `ex1.rs`
//! (2D): whichever contestant draws the lowest `rng::draw(seed, tick, organism_id,
//! EAT_CONTENTION_STREAM)` at a contested position wins the tile outright — the real `eat`
//! output, not a fractional split (splitting `food_here` before the `eat` cell sees it would
//! change what "eating" numerically means relative to the cell's own semantics). This is
//! the RNG's first real, branching use — EX-0 only recorded a mutation draw without letting
//! it decide anything yet.
use std::collections::{HashMap, HashSet};

use crate::rng;

pub const EAT_CONTENTION_STREAM: u8 = 1;
/// EX-3: predation-kill contention (multiple predators targeting the same prey organism in
/// one tick) — its own stream, distinct from `EAT_CONTENTION_STREAM`, one stream per
/// independent decision.
pub const PREDATION_CONTENTION_STREAM: u8 = 18;

/// `candidates`: every `(organism_id, position)` pair that both chose "eat here" and passed
/// `hungry_promoter` this tick — `position` is an opaque, `Eq + Hash` key (a flat 1D index
/// or a 2D `(x, y)` pair, whatever the caller's world uses). Returns the set of organism
/// ids that win their tile; everyone else keeps their post-decay energy unchanged, no eat
/// applied. A drawn-value tie (vanishingly unlikely) falls back to the lower organism id —
/// a last-resort tie-break, not the primary rule, so it doesn't reintroduce a
/// founder-always-wins bias. A thin wrapper over `resolve_contention` fixed to
/// `EAT_CONTENTION_STREAM` — unchanged signature/behavior for `ex0.rs`/`ex1.rs`/`ex2.rs`.
pub fn resolve_eat_contention<P: Eq + std::hash::Hash + Copy>(
    seed: u64,
    tick: u32,
    candidates: &[(u32, P)],
) -> HashSet<u32> {
    resolve_contention(seed, tick, candidates, EAT_CONTENTION_STREAM)
}

/// The general form: same order-independent, lowest-draw-wins resolution, over any caller-
/// chosen `stream` — EX-3's predation-kill contention needs its own stream
/// (`PREDATION_CONTENTION_STREAM`) distinct from eat-tile contention, since both can be live
/// in the same tick and must draw from independent streams.
pub fn resolve_contention<P: Eq + std::hash::Hash + Copy>(
    seed: u64,
    tick: u32,
    candidates: &[(u32, P)],
    stream: u8,
) -> HashSet<u32> {
    let mut by_pos: HashMap<P, Vec<u32>> = HashMap::new();
    for &(id, pos) in candidates {
        by_pos.entry(pos).or_default().push(id);
    }

    let mut winners = HashSet::with_capacity(by_pos.len());
    for ids in by_pos.values() {
        let winner = ids
            .iter()
            .copied()
            .min_by_key(|&id| (rng::draw(seed, tick, id, stream), id))
            .expect("non-empty contestant group");
        winners.insert(winner);
    }
    winners
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_contestant_always_wins() {
        let winners = resolve_eat_contention(1, 2, &[(7, 0usize)]);
        assert!(winners.contains(&7));
        assert_eq!(winners.len(), 1);
    }

    #[test]
    fn exactly_one_winner_per_contested_position() {
        let candidates: Vec<(u32, usize)> = vec![(1, 5), (2, 5), (3, 5), (4, 9), (5, 9)];
        let winners = resolve_eat_contention(0x5eed, 3, &candidates);
        assert_eq!(winners.len(), 2, "one winner per contested position");
        let pos5_winners = [1u32, 2, 3].iter().filter(|id| winners.contains(id)).count();
        let pos9_winners = [4u32, 5].iter().filter(|id| winners.contains(id)).count();
        assert_eq!(pos5_winners, 1);
        assert_eq!(pos9_winners, 1);
    }

    #[test]
    fn order_independent_regardless_of_candidate_list_order() {
        let candidates: Vec<(u32, usize)> = vec![(1, 5), (2, 5), (3, 5)];
        let mut reversed = candidates.clone();
        reversed.reverse();
        assert_eq!(
            resolve_eat_contention(42, 10, &candidates),
            resolve_eat_contention(42, 10, &reversed),
            "the winner must not depend on candidate list order"
        );
    }

    #[test]
    fn deterministic_across_repeated_calls() {
        let candidates: Vec<(u32, usize)> = vec![(10, 1), (11, 1), (12, 2)];
        assert_eq!(
            resolve_eat_contention(99, 4, &candidates),
            resolve_eat_contention(99, 4, &candidates)
        );
    }

    #[test]
    fn uncontested_positions_all_win() {
        let candidates: Vec<(u32, usize)> = vec![(1, 1), (2, 2), (3, 3)];
        let winners = resolve_eat_contention(7, 1, &candidates);
        assert_eq!(winners, HashSet::from([1, 2, 3]));
    }
}
