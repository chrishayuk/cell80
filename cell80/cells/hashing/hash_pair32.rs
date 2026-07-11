//! Deterministic hash mixing two full 32-bit values into one u16, the u32-domain analogue of hash_pair's own multiply-xor-multiply avalanche chain (widened constants, folded down instead of truncated) for combining two wide keys without pre-hashing either to u16 first.
//! tags: hash, mix, pair, fingerprint, key, combine, wide, u32
//! entry: HashPair32::run
struct HashPair32 { a: u32, b: u32, out: u16 }
impl HashPair32 {
    fn run(&mut self) -> u16 {
        let mut h = self.a.wrapping_mul(0x9E3779B9u32);
        h = (h ^ self.b).wrapping_mul(0x85EBCA6Bu32);
        h = h ^ (h >> 7u32);
        // Fold the mixed 32-bit state's high and low halves together (mix32's move)
        // rather than truncating, so every one of the 64 input bits still has a
        // chance to influence the u16 output.
        let lo = h as u16;
        let hi = (h >> 16u32) as u16;
        let r = lo ^ hi;
        self.out = r;
        r
    }
}
