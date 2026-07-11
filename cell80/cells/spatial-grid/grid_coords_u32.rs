//! Wide sibling of grid_coords: recovers (x, y) from a wide u32 flat array index and a u16 row width via x = index % width, y = index / width; guards width == 0 explicitly, returning (0, 0) instead of letting the divide/mod halt (unlike the generic div_floor_u32/mod_u32, which halt on a zero divisor).
//! tags: grid, index, coords, inverse, unflatten, 2d, spatial, divmod, guard, wide, u32
//! entry: GridCoordsWide::run
struct GridCoordsWide { index: u32, width: u16, x: u16, y: u32 }
impl GridCoordsWide {
    fn run(&mut self) -> u16 {
        if self.width == 0u16 {
            self.x = 0u16;
            self.y = 0u32;
        } else {
            let w = self.width as u32;
            self.x = (self.index % w) as u16;
            self.y = self.index / w;
        }
        1u16
    }
}
