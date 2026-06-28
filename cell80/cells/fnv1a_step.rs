//! One FNV-1a-style hash step over a byte: (hash ^ byte) * prime (16-bit).
//! tags: hash, fnv, fnv1a, step, rolling, checksum
fn run(hash: u16, byte: u16) -> u16 { (hash ^ (byte & 0xFFu16)).wrapping_mul(0x0193u16) }
