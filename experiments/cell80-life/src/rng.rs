//! Counter-based RNG for EX-0 (`experiments/deterministic-ecology.md`'s replay gate): a
//! pure function of `(seed, tick, organism_id, stream)`, not a mutable stream — so a draw
//! never depends on what order organisms are processed in. That's the property a GPU batch
//! dispatch (undefined thread order) needs from any randomness it touches; the existing
//! `cell80-life` binary's `Rng` (a single mutable xorshift64* stream consumed in `Vec`
//! iteration order) does not have it, which is exactly why EX-0 needs a different one.

/// The one stream EX-0 draws from today (a per-organism-per-tick mutation roll, computed
/// and recorded but not yet branching anything — see `experiments/deterministic-ecology.md`
/// EX-0's scope). Distinct experiments/roles get distinct `stream` values so their draws
/// never collide.
pub const MUTATION_STREAM: u8 = 0;

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
        let ascending: Vec<u32> = (0..64u32).map(|id| draw(seed, tick, id, MUTATION_STREAM)).collect();

        let mut shuffled_ids: Vec<u32> = (0..64u32).collect();
        shuffled_ids.sort_by_key(|&id| (id.wrapping_mul(37).wrapping_add(11)) % 64);
        assert_ne!(shuffled_ids, (0..64u32).collect::<Vec<_>>(), "the permutation used for this test is a no-op — pick a different one");

        for &id in &shuffled_ids {
            let v = draw(seed, tick, id, MUTATION_STREAM);
            assert_eq!(v, ascending[id as usize], "draw for id={id} depends on call order");
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
}
