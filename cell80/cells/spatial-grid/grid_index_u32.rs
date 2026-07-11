//! Wide/checked sibling of grid_index: flat array index = y*width + x for a u32-domain y and u16 x/width, escalating instead of silently wrapping (unlike grid_index, whose own doc comment admits it only holds if y*width+x fits u16) -- the encode-side counterpart to grid_coords_u32's decode.
//! tags: grid, index, flatten, 2d, spatial, array, wide, u32, checked, overflow, escalate
//! entry: GridIndexWide::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if y*width or (y*width)+x would exceed u32::MAX
struct GridIndexWide { x: u16, y: u32, width: u16, index: u32 }
impl GridIndexWide {
    fn run(&mut self) -> u16 {
        let row = mul_checked_u32(self.y, self.width as u32);
        let idx = add_checked_u32(row, self.x as u32);
        self.index = idx;
        1u16
    }
}
