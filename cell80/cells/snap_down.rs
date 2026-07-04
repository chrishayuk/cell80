//! Round x DOWN to the nearest multiple of step (x if step == 0). Floor to grid.
//! tags: snap, round-down, floor, multiple, grid, quantize
fn run(x: u16, step: u16) -> u16 { if step != 0u16 { (x / step) * step } else { x } }
