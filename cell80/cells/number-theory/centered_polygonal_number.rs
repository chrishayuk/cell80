//! The nth centered s-gonal number: C(s, n) = 1 + s*n*(n+1)/2 — the center point plus n rings of s points each (s >= 3, n >= 0; n=0 is the bare center point, 1, for every s). Star numbers are this family's s=12 case one ring later than its own usual 1-indexed convention (star_number(k) = centered_polygonal_number(12, k-1)) — not shipped as a separate cell for exactly that reason.
//! tags: number, centered, polygon, polygonal, hexagonal, star, figurate, sequence, math
//! limits: escalates (halt 0xFF06, out_of_domain) if s < 3; escalates (halt 0xFF05, needs_wider_math) if C(s, n) would exceed 65535
fn run(s: u16, n: u16) -> u16 {
    if s < 3u16 { halt(0xFF06u16); }
    let nw = n as u32;
    let tri = if n % 2u16 == 0u16 { (nw / 2u32) * (nw + 1u32) } else { nw * ((nw + 1u32) / 2u32) };
    let scaled = mul_checked_u32(s as u32, tri);
    let total = add_checked_u32(1u32, scaled);
    if total > 65535u32 { halt(0xFF05u16); }
    total as u16
}
