//! Avalanche-mix a full 32-bit value into a well-scrambled u16 finalizer hash, using a full-width xor-shift/multiply chain over all 32 input bits (never truncated to u16 first) before folding the result down -- the u32-domain sibling of mix16, for finalizing wide keys like a morton_encode index or a packed pair without discarding half their entropy.
//! tags: hash, mix, avalanche, scramble, finalize, fingerprint, wide, u32
//! entry: Mix32::run
struct Mix32 { x: u32, out: u16 }
impl Mix32 {
    fn run(&mut self) -> u16 {
        let mut h = self.x;
        h = (h ^ (h >> 16u32)).wrapping_mul(0x85EBCA6Bu32);
        h = (h ^ (h >> 13u32)).wrapping_mul(0xC2B2AE35u32);
        h = h ^ (h >> 16u32);
        // Fold the fully avalanched 32-bit state's high and low halves together so
        // every one of the original 32 input bits still influences the u16 output.
        let lo = h as u16;
        let hi = (h >> 16u32) as u16;
        let r = lo ^ hi;
        self.out = r;
        r
    }
}
