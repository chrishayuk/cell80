//! Given a geometric sequence's ratio, 1-indexed term number n, and that term's value, recover the starting value: start = term / ratio^(n-1) — the geometric analogue of arithmetic_first_term, complementing geometric_nth_checked_u32 (solves for term) and geometric_term_index (solves for n), since neither of those recovers start.
//! tags: number, geometric, sequence, inverse, first, start, term, ratio, math, checked, wide, u32, escalate
//! entry: GeometricFirstTerm::run
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0, if the divisor ratio^(n-1) is 0, or if term isn't an exact multiple of it; escalates (halt 0xFF05, needs_wider_math) if building the divisor overflows u32
struct GeometricFirstTerm { ratio: u32, n: u32, term: u32, start: u32 }
impl GeometricFirstTerm {
    fn run(&mut self) -> u16 {
        if self.n == 0u32 { halt(0xFF06u16); }
        let mut divisor = 1u32;
        let mut i = 1u32;
        while i < self.n {
            divisor = mul_checked_u32(divisor, self.ratio);
            i = i + 1u32;
        }
        if divisor == 0u32 { halt(0xFF06u16); }
        if self.term % divisor != 0u32 { halt(0xFF06u16); }
        self.start = self.term / divisor;
        1u16
    }
}
