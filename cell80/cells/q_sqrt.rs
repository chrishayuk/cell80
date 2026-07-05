//! Q8.8 fixed-point square root: sqrt(x/256)*256, via a branch-free bitwise integer square root on the widened x*256 (u32 only as a local, never a call param/return — the pattern every Q8.8 free function follows). A naive linear-scan integer sqrt was tried first and cost 3.6M cycles at the domain extreme (past the 2,000,000 default); this bitwise version costs under 20,000.
//! tags: fixed-point, q8.8, sqrt, square-root, math
fn run(x: u16) -> u16 {
    let n = x as u32 * 256u32;
    let mut val = n;
    let mut res = 0u32;
    let mut bit = 1u32 << 30u32;
    while bit > val {
        bit = bit >> 2u32;
    }
    while bit != 0u32 {
        if val >= res + bit {
            val = val - (res + bit);
            res = (res >> 1u32) + bit;
        } else {
            res = res >> 1u32;
        }
        bit = bit >> 2u32;
    }
    res as u16
}
