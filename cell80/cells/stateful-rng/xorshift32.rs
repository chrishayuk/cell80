//! 32-bit xorshift generator step (x ^= x<<13; x ^= x>>17; x ^= x<<5, Marsaglia's classic constants) over a full u32 state word, returning the top 16 bits — distinct from xorshift16 (different shifts, a full 32-bit word giving a 2^32-1 period instead of xorshift16's 2^16-1) and from lcg_next (no multiply, pure shift/xor). The caller threads `x` through — re-supply the field each call. A seed of 0 is a fixed point (0 forever); always seed nonzero.
//! tags: rng, random, pseudo-random, xorshift, seed, deterministic, generator, state, 32-bit
//! entry: Xorshift32::run
//! limits: a zero seed never escapes 0 (xorshift's well-known fixed point) — seed nonzero
struct Xorshift32 { x: u32 }
impl Xorshift32 {
    fn run(&mut self) -> u16 {
        let mut v = self.x;
        v = v ^ (v << 13u32);
        v = v ^ (v >> 17u32);
        v = v ^ (v << 5u32);
        self.x = v;
        (v >> 16u32) as u16
    }
}
