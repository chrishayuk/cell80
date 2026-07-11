//! Hamming distance between two 32-bit values: the count of bit positions where a and b differ, popcount(a ^ b) computed directly in u32 -- the wide sibling of hamming_distance16, needing u32 state fields since a and b cannot pass as fn run params under the 16-bit calling convention.
//! tags: bits, hamming, distance, xor, popcount, similarity, error-detection, wide, u32
//! entry: HammingDistance32::run
struct HammingDistance32 { a: u32, b: u32, out: u16 }
impl HammingDistance32 {
    fn run(&mut self) -> u16 {
        let mut v = self.a ^ self.b;
        let mut c = 0u16;
        while v != 0u32 {
            c = c + ((v & 1u32) as u16);
            v = v >> 1u32;
        }
        self.out = c;
        c
    }
}
