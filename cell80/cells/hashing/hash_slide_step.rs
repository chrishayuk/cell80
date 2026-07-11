//! Slide a multiplicative rolling hash window by one byte position: hash' = (hash - old_byte*high_pow) * base + new_byte (base fixed at 257, wrapping u16 throughout) -- the classic Rabin-Karp incremental update, distinct from every other append-only step in this pack (crc*_step/fnv1a_step/hash_pair/hash3/hash4 only ever fold a new value in, none can drop the oldest byte a fixed-width window has slid past).
//! tags: hash, rolling, slide, window, rabin-karp, step, multiplicative
//! entry: HashSlideStep::run
struct HashSlideStep { hash: u16, old_byte: u16, new_byte: u16, high_pow: u16, out: u16 }
impl HashSlideStep {
    fn run(&mut self) -> u16 {
        let removed = self.old_byte.wrapping_mul(self.high_pow);
        let h = self.hash.wrapping_sub(removed).wrapping_mul(257u16).wrapping_add(self.new_byte);
        self.out = h;
        h
    }
}
