//! Given a term value from a geometric sequence starting at start with ratio ratio, recover which 1-indexed term number n produced it -- the exact inverse of geometric_nth_checked_u32, found the same way that cell walks forward (iteratively multiplying, checked at every step) rather than any logarithm, so it escalates the moment growth either overflows or stalls at a fixed point without ever matching.
//! tags: number, geometric, sequence, inverse, term, index, ratio, math, checked, wide, u32, escalate
//! entry: GeometricTermIndex::run
//! limits: escalates (halt 0xFF06, out_of_domain) if the sequence reaches a fixed point (ratio == 1, ratio == 0, or start == 0 with a nonzero term) without ever equaling term; escalates (halt 0xFF05, needs_wider_math) if a term overflows u32 before matching
struct GeometricTermIndex { start: u32, ratio: u32, term: u32, n: u32 }
impl GeometricTermIndex {
    fn run(&mut self) -> u16 {
        let mut cur = self.start;
        let mut idx = 1u32;
        while cur != self.term {
            let next = mul_checked_u32(cur, self.ratio);
            if next == cur { halt(0xFF06u16); }
            cur = next;
            idx = idx + 1u32;
        }
        self.n = idx;
        1u16
    }
}
