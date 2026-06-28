//! Round x UP to the nearest multiple of step (x if step == 0). Ceil to grid.
//! tags: snap, round-up, ceiling, multiple, grid, quantize
fn run(x: u16, step: u16) -> u16 { let mut r = x; if step != 0u16 && x != 0u16 { r = ((x - 1u16) / step + 1u16) * step; } r }
