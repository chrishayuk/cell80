//! Check whether x is an s-gonal (polygonal) number for a given side count s (s >= 3) -- is there some n >= 0 with polygonal_number(s, n) == x. The membership-test counterpart of polygonal_number, one general predicate instead of a separate fixed-s check for every side count.
//! tags: number, polygon, polygonal, gonal, figurate, predicate, membership, check, math, sequence
//! limits: escalates (halt 0xFF06, out_of_domain) if s < 3
fn run(s: u16, x: u16) -> u16 {
    if s < 3u16 { halt(0xFF06u16); }
    let sw = s as u32;
    let xw = x as u32;
    let mut n = 0u32;
    let mut p = 0u32;
    while p < xw {
        n = n + 1u32;
        p = p + 1u32 + (sw - 2u32) * (n - 1u32);
    }
    if p == xw { 1u16 } else { 0u16 }
}
