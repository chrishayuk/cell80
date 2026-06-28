//! Round x to the NEAREST multiple of step (ties up; x if step == 0).
//! tags: round, nearest, multiple, snap, quantize, grid
fn run(x: u16, step: u16) -> u16 { let mut r = x; if step != 0u16 { r = ((x + step / 2u16) / step) * step; } r }
