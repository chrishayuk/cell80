//! Integer harmonic mean of two u16 values, floor(2*a*b/(a+b)), 0 if a+b == 0 -- the third leg of the classical AM-GM-HM triad (avg2, geomean2) that safe-arith was missing, computed as 2*q + floor(2*r/(a+b)) from q = prod/(a+b), r = prod%(a+b) to avoid the 2*a*b overflow the naive doubled-product form risks.
//! tags: math, mean, average, harmonic-mean, hm, reciprocal, safe-arith
fn run(a: u16, b: u16) -> u16 {
    let denom = a as u32 + b as u32;
    if denom != 0u32 {
        let prod = a as u32 * b as u32;
        let q = prod / denom;
        let r = prod % denom;
        (2u32 * q + (2u32 * r) / denom) as u16
    } else {
        0u16
    }
}
