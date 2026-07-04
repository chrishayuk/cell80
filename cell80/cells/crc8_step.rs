//! One CRC-8 (Dallas/Maxim, poly 0x8C reflected) step over a byte.
//! tags: hash, crc, crc8, checksum, step, rolling
fn run(crc: u16, byte: u16) -> u16 {
    let mut c = (crc ^ byte) & 0xFFu16;
    let mut i = 0u16;
    while i < 8u16 {
        c = if (c & 1u16) != 0u16 { (c >> 1u16) ^ 0x8Cu16 } else { c >> 1u16 };
        i = i + 1u16;
    }
    c
}
