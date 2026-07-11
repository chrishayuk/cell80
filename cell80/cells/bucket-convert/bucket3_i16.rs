//! Bucket a signed x into 0, 1, or 2 by two ascending signed thresholds via plain i16 comparison: x<t1 → 0, x<t2 → 1, else 2 -- the signed sibling of bucket3 (which only works over u16 and misreads negative deltas as huge unsigned values).
//! tags: bucket, bin, classify, threshold, tier, quantize, signed, i16, delta, temperature
fn run(x: i16, t1: i16, t2: i16) -> u16 {
    if x >= t2 { 2u16 } else if x >= t1 { 1u16 } else { 0u16 }
}
