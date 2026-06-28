//! Deterministic hash mixing two values into one u16.
//! tags: hash, mix, pair, fingerprint, key, combine
fn run(a: u16, b: u16) -> u16 { let mut h = a.wrapping_mul(0x9E37u16); h = (h ^ b).wrapping_mul(0x85EBu16); h ^ (h >> 7u16) }
