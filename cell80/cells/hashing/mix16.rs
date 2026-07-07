//! Avalanche-mix one u16 into a well-scrambled u16 (a finalizer / hash of one value).
//! tags: hash, mix, avalanche, scramble, finalize, fingerprint
fn run(x: u16) -> u16 { let mut h = x; h = (h ^ (h >> 8u16)).wrapping_mul(0x2993u16); h = h ^ (h >> 7u16); h }
