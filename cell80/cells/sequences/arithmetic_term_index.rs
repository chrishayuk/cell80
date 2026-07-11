//! Given start, step, and a term value from an arithmetic sequence, recover which term number n produced it: n = (term - start)/step + 1 — the missing inverse of arithmetic_nth_u32 (which only goes forward from n to term, never back from a term to its index).
//! tags: number, arithmetic, sequence, inverse, term, index, nth, math, checked, wide, u32, escalate
//! entry: ArithmeticTermIndex::run
//! limits: escalates (halt 0xFF06, out_of_domain) if step == 0, if term < start, or if (term - start) isn't an exact multiple of step; escalates (halt 0xFF05, needs_wider_math) if the final index+1 add overflows
struct ArithmeticTermIndex { start: u32, step: u32, term: u32, n: u32 }
impl ArithmeticTermIndex {
    fn run(&mut self) -> u16 {
        if self.step == 0u32 { halt(0xFF06u16); }
        if self.term < self.start { halt(0xFF06u16); }
        let gap = self.term - self.start;
        if gap % self.step != 0u32 { halt(0xFF06u16); }
        let idx = gap / self.step;
        self.n = add_checked_u32(idx, 1u32);
        1u16
    }
}
