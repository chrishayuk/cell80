//! Deterministic hash mixing three values into one u16, extending hash_pair's own multiply-xor-multiply chain by one more term and prime.
//! tags: hash, mix, triple, fingerprint, key, combine
fn run(a: u16, b: u16, c: u16) -> u16 {
    let mut h = a.wrapping_mul(0x9E37u16);
    h = (h ^ b).wrapping_mul(0x85EBu16);
    h = (h ^ c).wrapping_mul(0xEB2Fu16);
    h ^ (h >> 7u16)
}
