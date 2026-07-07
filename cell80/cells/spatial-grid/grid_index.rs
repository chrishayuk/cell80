//! Flat array index of a grid cell (x, y) in a grid of the given row width: y * width + x.
//! tags: grid, index, flatten, 2d, spatial, array
//! limits: assumes y*width+x fits u16 (like percent/scale_percent)
fn run(x: u16, y: u16, width: u16) -> u16 {
    y * width + x
}
