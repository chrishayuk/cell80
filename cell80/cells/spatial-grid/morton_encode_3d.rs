//! 3-axis Morton (Z-order) encode: interleave the low 10 bits of three u16 coordinates (x,y,z) into one 30-bit u32 code via the same branch-free "magic numbers" bit-spread as morton_encode, generalized from a 2-way to a 3-way interleave (x's bits at positions 0,3,6..., y's at 1,4,7..., z's at 2,5,8...) -- the octree/voxel-key analogue of morton_encode's 2D quadtree key.
//! tags: morton, z-order, spatial, index, interleave, bits, encode, octree, voxel, 3d, grid, wide, u32
//! entry: MortonEncode3d::run
struct MortonEncode3d { x: u16, y: u16, z: u16, code: u32 }
impl MortonEncode3d {
    fn run(&mut self) -> u16 {
        let mut vx = (self.x as u32) & 0x000003FFu32;
        vx = (vx | (vx << 16u32)) & 0x030000FFu32;
        vx = (vx | (vx << 8u32)) & 0x0300F00Fu32;
        vx = (vx | (vx << 4u32)) & 0x030C30C3u32;
        vx = (vx | (vx << 2u32)) & 0x09249249u32;

        let mut vy = (self.y as u32) & 0x000003FFu32;
        vy = (vy | (vy << 16u32)) & 0x030000FFu32;
        vy = (vy | (vy << 8u32)) & 0x0300F00Fu32;
        vy = (vy | (vy << 4u32)) & 0x030C30C3u32;
        vy = (vy | (vy << 2u32)) & 0x09249249u32;

        let mut vz = (self.z as u32) & 0x000003FFu32;
        vz = (vz | (vz << 16u32)) & 0x030000FFu32;
        vz = (vz | (vz << 8u32)) & 0x0300F00Fu32;
        vz = (vz | (vz << 4u32)) & 0x030C30C3u32;
        vz = (vz | (vz << 2u32)) & 0x09249249u32;

        self.code = vx | (vy << 1u32) | (vz << 2u32);
        1u16
    }
}
