//! 16-bit xorshift generator step (x ^= x<<7; x ^= x>>9; x ^= x<<8) — a distinct pseudo-random recurrence from lcg_next (no multiply, pure shift/xor). The caller threads `x` through — re-supply the field each call. A seed of 0 is a fixed point (0 forever); always seed nonzero.
//! tags: rng, random, pseudo-random, xorshift, seed, deterministic, generator, state
//! entry: Xorshift16::run
//! limits: a zero seed never escapes 0 (xorshift's well-known fixed point) — seed nonzero
struct Xorshift16 { x: u16 }
impl Xorshift16 {
    fn run(&mut self) -> u16 {
        let mut v = self.x;
        v = v ^ (v << 7u16);
        v = v ^ (v >> 9u16);
        v = v ^ (v << 8u16);
        self.x = v;
        v
    }
}
