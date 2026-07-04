//! Returns 1 if x is a power of two, else 0.
//! tags: number, power, predicate, pow2, bits, single-bit
fn run(x: u16) -> u16 { (x != 0u16 && (x & (x - 1u16)) == 0u16) as u16 }
