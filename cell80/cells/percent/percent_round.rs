//! Round-to-nearest percentage of a whole, ties rounding up: round(part*100/whole) via round_div's overflow-safe tie test in u32 (0 if whole == 0, saturating at 65535) -- the rounding-mode sibling of percent, which floors instead.
//! tags: percent, percentage, ratio, proportion, round, round-nearest, nearest, ties-up, rounding, division
fn run(part: u16, whole: u16) -> u16 {
    let mut result = 0u16;
    if whole != 0u16 {
        let w = whole as u32;
        let num = part as u32 * 100u32;
        let q = num / w;
        let r = num % w;
        let q2 = if r >= w - r { q + 1u32 } else { q };
        result = if (q2 >> 16u32) as u16 != 0u16 { 65535u16 } else { q2 as u16 };
    }
    result
}
