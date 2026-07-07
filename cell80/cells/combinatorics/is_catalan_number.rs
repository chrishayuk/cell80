//! Check whether x is a Catalan number (1, 1, 2, 5, 14, 42, 132, 429, ...) -- the inverse-membership test, distinct from catalan_number (which computes the nth one directly). Walks the same recurrence catalan_number uses (C(0)=1, C(n+1)=C(n)*2*(2n+1)/(n+2)) upward until it reaches or passes x, bounded by x itself. Never escalates: x is u16-bounded, and C(12) = 208012 already exceeds u16::MAX, so the search always terminates within the u16 domain long before any u32 intermediate could overflow.
//! tags: number, catalan, combinatorics, membership, predicate, sequence
fn run(x: u16) -> u16 {
    let xw = x as u32;
    let mut c = 1u32;
    if c == xw {
        return 1u16;
    }
    let mut k = 0u32;
    while c < xw {
        let term = 2u32 * (2u32 * k + 1u32);
        let num = mul_checked_u32(c, term);
        c = num / (k + 2u32);
        k = k + 1u32;
        if c == xw {
            return 1u16;
        }
    }
    0u16
}
