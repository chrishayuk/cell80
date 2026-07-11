//! Isolates the value of the lowest set bit of x via x & (0 - x); 0 when x == 0 (the classic two's-complement isolation trick, distinct from every existing op that names a bit index or only returns a count).
//! tags: bits, lowest-set-bit, isolate, two's-complement, lsb, blsi
fn run(x: u16) -> u16 { x & (0u16.wrapping_sub(x)) }
