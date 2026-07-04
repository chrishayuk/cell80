//! Quantize x to a bucket index by step size: x / step (0 if step == 0).
//! tags: quantize, bucket, step, index, discretize, bin
fn run(x: u16, step: u16) -> u16 { if step != 0u16 { x / step } else { 0u16 } }
