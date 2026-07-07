//! Linear congruential generator step: seed = seed * 1664525 + 1013904223 (mod 2^32, Numerical Recipes constants), returning the top 16 bits (the higher bits of an LCG are far less patterned than the low bits). The caller threads `seed` through — re-supply the field each call, since state cells don't persist memory across separate runs.
//! tags: rng, random, pseudo-random, lcg, seed, deterministic, generator, state
//! entry: Lcg::run
struct Lcg { seed: u32 }
impl Lcg {
    fn run(&mut self) -> u16 {
        let n = self.seed.wrapping_mul(1664525u32).wrapping_add(1013904223u32);
        self.seed = n;
        (n >> 16u32) as u16
    }
}
