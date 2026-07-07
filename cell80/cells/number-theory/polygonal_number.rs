//! The nth s-gonal (polygonal) number: P(s, n) = n + (s-2)*n*(n-1)/2, for a polygon with s sides (s >= 3). s=3 reproduces triangular's own values (kept as a separate cell for its own retrieval identity, not folded away), s=4 is the perfect squares, s=5 is pentagonal, s=6 is hexagonal, and so on — one general cell instead of a differently-named cell for every side count.
//! tags: number, polygon, polygonal, gonal, pentagonal, hexagonal, heptagonal, octagonal, figurate, sequence, math
//! limits: escalates (halt 0xFF06, out_of_domain) if s < 3; escalates (halt 0xFF05, needs_wider_math) if P(s, n) would exceed 65535
fn run(s: u16, n: u16) -> u16 {
    if s < 3u16 { halt(0xFF06u16); }
    let sw = s as u32;
    let nw = n as u32;
    let tri = if n % 2u16 == 0u16 { (nw / 2u32) * (nw - 1u32) } else { nw * ((nw - 1u32) / 2u32) };
    let scaled = mul_checked_u32(sw - 2u32, tri);
    let total = add_checked_u32(nw, scaled);
    if total > 65535u32 { halt(0xFF05u16); }
    total as u16
}
