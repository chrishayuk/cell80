//! One CRC-32 (CRC-32/ISO-HDLC, poly 0xEDB88320 reflected) step over a byte on a 32-bit accumulator -- the crc8_step/crc16_step shift-xor loop widened one more rung to a full u32 crc field, needing a state cell since the calling convention has no u32 free-fn parameters.
//! tags: hash, crc, crc32, checksum, step, rolling, wide, u32
//! entry: Crc32Step::run
struct Crc32Step { crc: u32, byte: u16, out: u32 }
impl Crc32Step {
    fn run(&mut self) -> u16 {
        let mut c = self.crc ^ ((self.byte & 0xFFu16) as u32);
        let mut i = 0u16;
        while i < 8u16 {
            c = if (c & 1u32) != 0u32 { (c >> 1u32) ^ 0xEDB88320u32 } else { c >> 1u32 };
            i = i + 1u16;
        }
        self.out = c;
        1u16
    }
}
