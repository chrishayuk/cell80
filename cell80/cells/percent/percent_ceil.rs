//! Ceiling percentage of a whole: the smallest integer percent p such that p*whole/100 >= part, via the q+1-if-remainder technique div_ceil_u32 uses (0 if whole == 0, saturating at 65535) -- the ceiling sibling of percent (which floors part*100/whole).
//! tags: percent, percentage, ceiling, ceil, round-up, ratio, proportion, fraction, rate
fn run(part: u16, whole: u16) -> u16 {
    let mut r = 0u16;
    if whole != 0u16 {
        let p = part as u32 * 100u32;
        let q = p / whole as u32;
        let rem = p % whole as u32;
        let rounded = if rem != 0u32 { q + 1u32 } else { q };
        r = if (rounded >> 16u32) as u16 != 0u16 { 65535u16 } else { rounded as u16 };
    }
    r
}
