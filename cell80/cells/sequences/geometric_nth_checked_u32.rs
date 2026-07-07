//! The nth term of a geometric sequence starting at start with ratio ratio: start * ratio^(n-1), 1-indexed (n=1 is the first term) — the missing nth-term sibling of geometric_series_sum (which only sums the sequence, not a single term). Computed by direct iterative multiplication rather than exponentiation, so it escalates exactly when the true term doesn't fit u32, no earlier.
//! tags: number, geometric, sequence, nth, term, ratio, math, checked, wide, u32, escalate
//! entry: GeometricNthChecked::run
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0; escalates (halt 0xFF05, needs_wider_math) if a term overflows u32
struct GeometricNthChecked { start: u32, ratio: u32, n: u32, result: u32 }
impl GeometricNthChecked {
    fn run(&mut self) -> u16 {
        if self.n == 0u32 { halt(0xFF06u16); }
        let mut term = self.start;
        let mut i = 1u32;
        while i < self.n {
            term = mul_checked_u32(term, self.ratio);
            i = i + 1u32;
        }
        self.result = term;
        1u16
    }
}
