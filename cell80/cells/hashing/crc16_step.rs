//! One CRC-16 (CRC-16/ARC, poly 0xA001 reflected) step over a byte, the crc8_step shift-xor loop widened to the full 16-bit register instead of masking down to 8 bits.
//! tags: hash, crc, crc16, checksum, step, rolling
fn run(crc: u16, byte: u16) -> u16 {
    let mut c = crc ^ (byte & 0xFFu16);
    let mut i = 0u16;
    while i < 8u16 {
        c = if (c & 1u16) != 0u16 { (c >> 1u16) ^ 0xA001u16 } else { c >> 1u16 };
        i = i + 1u16;
    }
    c
}
