//! Inverse of grid_index: recovers the (x, y) grid coordinates from a flat array index and the grid's row width via x = index % width, y = index / width; guards width == 0 explicitly, returning (0, 0) instead of letting the divide/mod halt on DivByZero.
//! tags: grid, index, coords, inverse, unflatten, 2d, spatial, divmod, guard
//! entry: GridCoords::run
struct GridCoords { index: u16, width: u16, x: u16, y: u16 }
impl GridCoords {
    fn run(&mut self) -> u16 {
        if self.width == 0u16 {
            self.x = 0u16;
            self.y = 0u16;
        } else {
            self.x = self.index % self.width;
            self.y = self.index / self.width;
        }
        1u16
    }
}
