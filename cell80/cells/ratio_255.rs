//! Ratio scaled to a 0..255 byte fraction: part*255/whole (0 if whole == 0).
//! tags: ratio, byte, fraction, scale, proportion, normalize
fn run(part: u16, whole: u16) -> u16 {
    let mut r = 0u16;
    if whole != 0u16 {
        let q = part as u32 * 255u32 / whole as u32;
        r = q as u16;
        if (q >> 16u32) as u16 != 0u16 {
            r = 65535u16;
        }
    }
    r
}
