//! Take pct percent of a value: value*pct/100.
//! tags: percent, scale, of, fraction, proportion, multiply
fn run(value: u16, pct: u16) -> u16 {
    let q = value as u32 * pct as u32 / 100u32;
    let mut r = q as u16;
    if (q >> 16u32) as u16 != 0u16 {
        r = 65535u16;
    }
    r
}
