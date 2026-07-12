//! Counter-based RNG for EX-0 (`experiments/deterministic-ecology.md`'s replay gate): a
//! pure function of `(seed, tick, organism_id, stream)`, not a mutable stream — so a draw
//! never depends on what order organisms are processed in. That's the property a GPU batch
//! dispatch (undefined thread order) needs from any randomness it touches; the existing
//! `cell80-life` binary's `Rng` (a single mutable xorshift64* stream consumed in `Vec`
//! iteration order) does not have it, which is exactly why EX-0 needs a different one.

/// A per-organism-per-tick mutation roll (EX-0: computed and recorded but not yet
/// branching anything — see `experiments/deterministic-ecology.md` EX-0's scope). Distinct
/// experiments/roles get distinct `stream` values so their draws never collide; see also
/// `contention::EAT_CONTENTION_STREAM` (EX-1) and the `MUTATE_*` streams below (EX-2), the
/// RNG's other real, branching uses.
pub const MUTATION_STREAM: u8 = 0;

// EX-2 (`experiments/deterministic-ecology.md`): one stream per independent mutation
// decision, keyed `draw(seed, tick, child_id, STREAM)` — the child's id, since the child is
// what's being generated. Split chance/magnitude (and swap chance/target) into separate
// streams rather than packing them into one draw via mod/div: each stream constant then has
// exactly one nameable meaning, which matters for auditing a specific mutation event later.
pub const MUTATE_DECAY_CHANCE_STREAM: u8 = 3;
pub const MUTATE_DECAY_MAGNITUDE_STREAM: u8 = 4;
pub const MUTATE_THRESHOLD_CHANCE_STREAM: u8 = 5;
pub const MUTATE_THRESHOLD_MAGNITUDE_STREAM: u8 = 6;
pub const MUTATE_GIVE_PCT_CHANCE_STREAM: u8 = 7;
pub const MUTATE_GIVE_PCT_MAGNITUDE_STREAM: u8 = 8;
pub const MUTATE_HUNGRY_SWAP_CHANCE_STREAM: u8 = 9;
pub const MUTATE_HUNGRY_SWAP_TARGET_STREAM: u8 = 10;
pub const MUTATE_REPRO_SWAP_CHANCE_STREAM: u8 = 11;
pub const MUTATE_REPRO_SWAP_TARGET_STREAM: u8 = 12;
pub const MUTATE_SENSE_SWAP_CHANCE_STREAM: u8 = 13;
pub const MUTATE_SENSE_SWAP_TARGET_STREAM: u8 = 14;

/// A draw decides "does this mutation fire" at `pct` percent — `draw(...) % 100 < pct`,
/// the counter-based analog of `main.rs`'s `Rng::chance`.
pub fn chance(seed: u64, tick: u32, child_id: u32, stream: u8, pct: u64) -> bool {
    (draw(seed, tick, child_id, stream) as u64) % 100 < pct
}

/// Pick a pool member other than `current`, uniformly over the remaining `n - 1` — a pure
/// function of the four fixed inputs, no retry loop, faithfully generalizing `main.rs`'s
/// `pick_other` (a rejection loop over a mutable stream) to counter-based determinism.
/// `n` must be `>= 2` (a pool of 1 has no "other" member to pick).
pub fn pick_other_index(
    seed: u64,
    tick: u32,
    child_id: u32,
    stream: u8,
    current: u16,
    n: u16,
) -> u16 {
    debug_assert!(n >= 2, "pick_other_index needs at least 2 pool members");
    // Modulo in u32 space (the draw's native width) before narrowing, so the full draw
    // contributes entropy rather than just its low 16 bits.
    let raw = (draw(seed, tick, child_id, stream) % (n as u32 - 1)) as u16;
    if raw >= current {
        raw + 1
    } else {
        raw
    }
}

/// `draw(seed, tick, organism_id, stream)` — same four inputs always produce the same u32,
/// regardless of call order relative to any other `(tick, organism_id)` pair. A SplitMix64
/// finalizer over a linear combination of the inputs: this is "counter-based in shape," not
/// a full Philox/Threefry (fewer/weaker mixing rounds) — sufficient rigor for proving the
/// determinism/order-independence *contract* EX-0 tests; real statistical quality is a later
/// concern (swap the internals behind this same signature) if EX-2/EX-3 need it at scale.
pub fn draw(seed: u64, tick: u32, organism_id: u32, stream: u8) -> u32 {
    let mut z = seed
        ^ (tick as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (organism_id as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ (stream as u64).wrapping_mul(0x94D0_49BB_1331_11EB);
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 32) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_same_inputs_same_output() {
        assert_eq!(draw(1, 2, 3, 0), draw(1, 2, 3, 0));
    }

    #[test]
    fn order_independent_across_organism_ids() {
        // The property a GPU dispatch actually needs: computing draws for a fixed
        // (seed, tick) in ascending id order vs. some other order must land on the exact
        // same id -> value map, since real dispatch order is undefined.
        let seed = 0x5eed_1234;
        let tick = 7;
        let ascending: Vec<u32> = (0..64u32)
            .map(|id| draw(seed, tick, id, MUTATION_STREAM))
            .collect();

        let mut shuffled_ids: Vec<u32> = (0..64u32).collect();
        shuffled_ids.sort_by_key(|&id| (id.wrapping_mul(37).wrapping_add(11)) % 64);
        assert_ne!(
            shuffled_ids,
            (0..64u32).collect::<Vec<_>>(),
            "the permutation used for this test is a no-op — pick a different one"
        );

        for &id in &shuffled_ids {
            let v = draw(seed, tick, id, MUTATION_STREAM);
            assert_eq!(
                v, ascending[id as usize],
                "draw for id={id} depends on call order"
            );
        }
    }

    #[test]
    fn streams_diverge() {
        assert_ne!(draw(1, 2, 3, 0), draw(1, 2, 3, 1));
    }

    #[test]
    fn ticks_and_ids_diverge() {
        assert_ne!(draw(1, 2, 3, 0), draw(1, 3, 3, 0));
        assert_ne!(draw(1, 2, 3, 0), draw(1, 2, 4, 0));
    }

    #[test]
    fn chance_roughly_matches_requested_pct_over_many_children() {
        let hits = (0..10_000u32)
            .filter(|&id| chance(0x5eed, 1, id, MUTATE_DECAY_CHANCE_STREAM, 25))
            .count();
        let observed = hits as f64 / 10_000.0;
        assert!(
            (observed - 0.25).abs() < 0.02,
            "observed {observed}, want near 0.25"
        );
    }

    #[test]
    fn pick_other_index_never_returns_current() {
        for current in 0..10u16 {
            for id in 0..200u32 {
                let picked =
                    pick_other_index(0x5eed, 1, id, MUTATE_SENSE_SWAP_TARGET_STREAM, current, 10);
                assert_ne!(picked, current);
                assert!(picked < 10);
            }
        }
    }

    #[test]
    fn pick_other_index_covers_every_other_member() {
        // Over enough children, every one of the n-1 valid targets should show up at least
        // once — a cheap check that the exclusion mapping isn't secretly biased toward a
        // subset (e.g. always landing on 0 or n-1).
        let n = 6u16;
        let current = 2u16;
        let mut seen = std::collections::HashSet::new();
        for id in 0..2_000u32 {
            seen.insert(pick_other_index(
                0x5eed,
                1,
                id,
                MUTATE_SENSE_SWAP_TARGET_STREAM,
                current,
                n,
            ));
        }
        assert_eq!(
            seen.len(),
            (n - 1) as usize,
            "expected all {} other members reachable, saw {:?}",
            n - 1,
            seen
        );
    }

    #[test]
    fn pick_other_index_order_independent() {
        // Same property draw() itself guarantees, one level up: recomputing for a fixed
        // (current, n) in a different id order must not change any individual result.
        let current = 3u16;
        let n = 8u16;
        for id in [0u32, 50, 999, 7] {
            let a = pick_other_index(0x5eed, 4, id, MUTATE_HUNGRY_SWAP_TARGET_STREAM, current, n);
            let b = pick_other_index(0x5eed, 4, id, MUTATE_HUNGRY_SWAP_TARGET_STREAM, current, n);
            assert_eq!(a, b);
        }
    }
}
