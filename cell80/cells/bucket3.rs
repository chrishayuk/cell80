//! Bucket x into 0, 1, or 2 by two ascending thresholds: x<t1 → 0, x<t2 → 1, else 2.
//! tags: bucket, bin, classify, threshold, tier, quantize
fn run(x: u16, t1: u16, t2: u16) -> u16 { if x >= t2 { 2u16 } else if x >= t1 { 1u16 } else { 0u16 } }
