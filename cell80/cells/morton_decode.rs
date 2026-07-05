//! Morton (Z-order curve) decode: the inverse of morton_encode — split a u32 spatial index back into its two interleaved u16 coordinates via the same branch-free bit-compaction trick (constant shift amounts, no dynamic-shift loop).
//! tags: morton, z-order, spatial, index, deinterleave, bits, decode, grid, wide, u32
//! entry: MortonDecode::run
struct MortonDecode { code: u32, x: u16, y: u16 }
impl MortonDecode {
    fn run(&mut self) -> u16 {
        let mut vx = self.code & 0x55555555u32;
        vx = (vx | (vx >> 1u32)) & 0x33333333u32;
        vx = (vx | (vx >> 2u32)) & 0x0F0F0F0Fu32;
        vx = (vx | (vx >> 4u32)) & 0x00FF00FFu32;
        vx = (vx | (vx >> 8u32)) & 0x0000FFFFu32;

        let mut vy = (self.code >> 1u32) & 0x55555555u32;
        vy = (vy | (vy >> 1u32)) & 0x33333333u32;
        vy = (vy | (vy >> 2u32)) & 0x0F0F0F0Fu32;
        vy = (vy | (vy >> 4u32)) & 0x00FF00FFu32;
        vy = (vy | (vy >> 8u32)) & 0x0000FFFFu32;

        self.x = vx as u16;
        self.y = vy as u16;
        1u16
    }
}
