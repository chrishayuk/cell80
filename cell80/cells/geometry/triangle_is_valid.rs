//! Returns 1 if three side lengths (a, b, c) form a valid (non-degenerate) triangle, i.e. each side is strictly less than the sum of the other two, else 0. Sums are widened to u32 internally so a large pair (e.g. two sides near 65535) can't wrap past u16 and silently flip the verdict.
//! tags: geometry, triangle, validate, predicate, inequality, math
fn run(a: u16, b: u16, c: u16) -> u16 {
    let aw = a as u32;
    let bw = b as u32;
    let cw = c as u32;
    let mut r = 0u16;
    if aw + bw > cw && bw + cw > aw && aw + cw > bw {
        r = 1u16;
    }
    r
}
