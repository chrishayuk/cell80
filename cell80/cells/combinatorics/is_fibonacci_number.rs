//! Check whether x is a Fibonacci number (0, 1, 1, 2, 3, 5, 8, 13, 21, ...) -- the inverse-membership test, distinct from fibonacci_checked_u32 (which computes the nth one directly, indexed by n), mirroring the catalan_number / is_catalan_number sibling pair already in this pack. Walks the same recurrence fibonacci_checked_u32 uses (F(0)=0, F(1)=1, F(n)=F(n-1)+F(n-2)) upward until it reaches or passes x, bounded by x itself. Never escalates: x is u16-bounded, and F(25) = 75025 already exceeds u16::MAX, so the search always terminates within ~24 steps long before any u32 intermediate could overflow.
//! tags: number, fibonacci, combinatorics, membership, predicate, sequence
fn run(x: u16) -> u16 {
    let xw = x as u32;
    let mut a = 0u32;
    if a == xw {
        return 1u16;
    }
    let mut b = 1u32;
    if b == xw {
        return 1u16;
    }
    while b < xw {
        let next = a + b;
        a = b;
        b = next;
        if b == xw {
            return 1u16;
        }
    }
    0u16
}
