//! Round x to the NEAREST multiple of step (ties up; x if step == 0).
//! tags: round, nearest, multiple, snap, quantize, grid
fn run(x: u16, step: u16) -> u16 { if step != 0u16 { ((x + step / 2u16) / step) * step } else { x } }
