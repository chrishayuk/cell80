//! Given an arithmetic sequence's start, the index n of a known term, and that term's value, recover the common difference: step = (term - start) / (n - 1) -- the third and last solvable unknown in start + step*(n-1) = term, completing the missing-sibling triple alongside arithmetic_nth_u32 (solves for term) and arithmetic_term_index (solves for n).
//! tags: number, arithmetic, sequence, inverse, common difference, step, term, index, math, checked, wide, u32, escalate
//! entry: ArithmeticCommonDiff::run
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0, if n == 1 and term != start, if term < start, or if (term - start) isn't an exact multiple of (n - 1)
struct ArithmeticCommonDiff { start: u32, n: u32, term: u32, step: u32 }
impl ArithmeticCommonDiff {
    fn run(&mut self) -> u16 {
        if self.n == 0u32 { halt(0xFF06u16); }
        if self.n == 1u32 {
            // Only the first term is pinned down; any step satisfies it as long as
            // the claimed term actually equals start. Step itself is unconstrained,
            // so report the canonical 0 rather than dividing by (n-1) == 0.
            if self.term != self.start { halt(0xFF06u16); }
            self.step = 0u32;
            return 1u16;
        }
        if self.term < self.start { halt(0xFF06u16); }
        let gap = self.term - self.start;
        let nm1 = self.n - 1u32;
        if gap % nm1 != 0u32 { halt(0xFF06u16); }
        self.step = gap / nm1;
        1u16
    }
}
