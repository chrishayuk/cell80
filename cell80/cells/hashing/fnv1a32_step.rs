//! One true FNV-1a hash step over a byte on a full 32-bit accumulator using the real FNV-1a constants -- hash = (hash ^ byte) * 16777619 (offset basis conventionally 2166136261) -- the u32-domain sibling of fnv1a_step (which only ever ran at u16 width), needing a state cell since the calling convention has no u32 free-fn parameters.
//! tags: hash, fnv, fnv1a, step, rolling, checksum, wide, u32
//! entry: Fnv1a32Step::run
struct Fnv1a32Step { hash: u32, byte: u16, out: u32 }
impl Fnv1a32Step {
    fn run(&mut self) -> u16 {
        let h = self.hash ^ ((self.byte & 0xFFu16) as u32);
        self.out = h.wrapping_mul(16777619u32);
        1u16
    }
}
