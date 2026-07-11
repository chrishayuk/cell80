//! The winning margin among three raw candidate scores a,b,c — top minus second-highest (the median of the three) — the raw value clear_winner3 computes internally but only exposes as a threshold boolean.
//! tags: winner, margin, score, ranking, median, three, candidate, gap, plan
fn run(a: u16, b: u16, c: u16) -> u16 {
    let top = imax(imax(a, b), c);
    let lo = imin(imin(a, b), c);
    let second = a.wrapping_add(b).wrapping_add(c).wrapping_sub(lo).wrapping_sub(top);
    top - second
}
