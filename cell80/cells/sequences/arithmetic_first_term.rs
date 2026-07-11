//! Given an arithmetic sequence's common difference step, the index n of a known term, and that term's value, recover the starting value: start = term - step*(n-1) -- the actually-final unknown in start + step*(n-1) = term, since arithmetic_nth_u32 solves for term, arithmetic_term_index for n, and arithmetic_common_diff for step, but none of them has ever solved for start itself.
//! tags: number, arithmetic, sequence, inverse, first term, start, index, math, checked, wide, u32, escalate
//! entry: ArithmeticFirstTerm::run
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0, or if step*(n-1) exceeds term; escalates (halt 0xFF05, needs_wider_math) if the step*(n-1) multiply overflows
struct ArithmeticFirstTerm { step: u32, n: u32, term: u32, start: u32 }
impl ArithmeticFirstTerm {
    fn run(&mut self) -> u16 {
        if self.n == 0u32 { halt(0xFF06u16); }
        if self.n == 1u32 {
            self.start = self.term;
            return 1u16;
        }
        let nm1 = self.n - 1u32;
        let offset = mul_checked_u32(self.step, nm1);
        if offset > self.term { halt(0xFF06u16); }
        self.start = self.term - offset;
        1u16
    }
}
