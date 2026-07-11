//! Deterministic hash mixing five values into one u16, extending hash4's own multiply-xor-multiply chain by one more term and prime -- for hashing five-field records without pre-combining pairs by hand.
//! tags: hash, mix, quintet, five, fingerprint, key, combine
//! entry: Hash5::run
struct Hash5 { a: u16, b: u16, c: u16, d: u16, e: u16, out: u16 }
impl Hash5 {
    fn run(&mut self) -> u16 {
        let mut h = self.a.wrapping_mul(0x9E37u16);
        h = (h ^ self.b).wrapping_mul(0x85EBu16);
        h = (h ^ self.c).wrapping_mul(0xEB2Fu16);
        h = (h ^ self.d).wrapping_mul(0xC2B2u16);
        h = (h ^ self.e).wrapping_mul(0x27D4u16);
        h = h ^ (h >> 7u16);
        self.out = h;
        h
    }
}
