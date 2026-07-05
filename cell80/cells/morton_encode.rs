//! Morton (Z-order curve) encode: interleave the bits of two u16 coordinates into one u32 spatial index (x's bits at even positions, y's at odd), so a single integer sorts nearby 2D points near each other — a common spatial-indexing key. The classic branch-free "magic numbers" bit-spread (constant shift amounts, no dynamic-shift loop): needs a u32 state field since interleaving two full u16s produces 32 bits, more than either input's own width.
//! tags: morton, z-order, spatial, index, interleave, bits, encode, grid, wide, u32
//! entry: MortonEncode::run
struct MortonEncode { x: u16, y: u16, code: u32 }
impl MortonEncode {
    fn run(&mut self) -> u16 {
        let mut vx = self.x as u32;
        vx = (vx | (vx << 8u32)) & 0x00FF00FFu32;
        vx = (vx | (vx << 4u32)) & 0x0F0F0F0Fu32;
        vx = (vx | (vx << 2u32)) & 0x33333333u32;
        vx = (vx | (vx << 1u32)) & 0x55555555u32;

        let mut vy = self.y as u32;
        vy = (vy | (vy << 8u32)) & 0x00FF00FFu32;
        vy = (vy | (vy << 4u32)) & 0x0F0F0F0Fu32;
        vy = (vy | (vy << 2u32)) & 0x33333333u32;
        vy = (vy | (vy << 1u32)) & 0x55555555u32;

        self.code = vx | (vy << 1u32);
        1u16
    }
}
