//! Morton (Z-order curve) 3D decode: the inverse of morton_encode_3d — split a 30-bit spatial index back into its three interleaved 10-bit coordinates (x,y,z) via the same branch-free bit-compaction trick as morton_decode, 3-way instead of 2-way.
//! tags: morton, z-order, spatial, index, deinterleave, bits, decode, grid, wide, u32, 3d
//! entry: MortonDecode3d::run
struct MortonDecode3d { code: u32, x: u16, y: u16, z: u16 }
impl MortonDecode3d {
    fn run(&mut self) -> u16 {
        let mut vx = self.code & 0x09249249u32;
        vx = (vx | (vx >> 2u32)) & 0x030C30C3u32;
        vx = (vx | (vx >> 4u32)) & 0x0300F00Fu32;
        vx = (vx | (vx >> 8u32)) & 0xFF0000FFu32;
        vx = (vx | (vx >> 16u32)) & 0x000003FFu32;

        let mut vy = (self.code >> 1u32) & 0x09249249u32;
        vy = (vy | (vy >> 2u32)) & 0x030C30C3u32;
        vy = (vy | (vy >> 4u32)) & 0x0300F00Fu32;
        vy = (vy | (vy >> 8u32)) & 0xFF0000FFu32;
        vy = (vy | (vy >> 16u32)) & 0x000003FFu32;

        let mut vz = (self.code >> 2u32) & 0x09249249u32;
        vz = (vz | (vz >> 2u32)) & 0x030C30C3u32;
        vz = (vz | (vz >> 4u32)) & 0x0300F00Fu32;
        vz = (vz | (vz >> 8u32)) & 0xFF0000FFu32;
        vz = (vz | (vz >> 16u32)) & 0x000003FFu32;

        self.x = vx as u16;
        self.y = vy as u16;
        self.z = vz as u16;
        1u16
    }
}
