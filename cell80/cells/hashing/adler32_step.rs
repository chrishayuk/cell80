//! One Adler-32 checksum step over a byte: s1=(s1+byte) mod 65521, s2=(s2+s1) mod 65521, packed as (s2<<16)|s1 -- two running sums mod a prime, a different checksum family from the crc*_step shift-xor reflected-polynomial line.
//! tags: hash, adler, adler32, checksum, step, rolling, wide, u32
//! entry: Adler32Step::run
struct Adler32Step { checksum: u32, byte: u16, out: u32 }
impl Adler32Step {
    fn run(&mut self) -> u16 {
        let s1 = self.checksum & 0xFFFFu32;
        let s2 = (self.checksum >> 16u32) & 0xFFFFu32;
        let s1n = (s1 + ((self.byte & 0xFFu16) as u32)) % 65521u32;
        let s2n = (s2 + s1n) % 65521u32;
        self.out = (s2n << 16u32) | s1n;
        1u16
    }
}
