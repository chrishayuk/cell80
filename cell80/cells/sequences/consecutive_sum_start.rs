//! Given n consecutive integers step apart summing to sum, find the first one: first = (sum - step*n*(n-1)/2) / n. Generalizes the "n consecutive integers" and "n consecutive odd/even integers" shapes into one cell via the step parameter (step=1 for consecutive integers, step=2 for consecutive odd/even). Escalates if the split isn't exact or would go negative — a wrong-plan signal.
//! tags: number, consecutive, sequence, sum, start, first, odd, even, math, checked, wide, u32, escalate
//! entry: ConsecutiveSumStart::run
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0, if the offset exceeds sum, or if the split isn't exact; escalates (halt 0xFF05, needs_wider_math) if a multiply overflows
struct ConsecutiveSumStart { n: u32, sum: u32, step: u32, first: u32 }
impl ConsecutiveSumStart {
    fn run(&mut self) -> u16 {
        if self.n == 0u32 { halt(0xFF06u16); }
        let nm1 = self.n - 1u32;
        let half_pair = if self.n % 2u32 == 0u32 {
            mul_checked_u32(self.n / 2u32, nm1)
        } else {
            mul_checked_u32(self.n, nm1 / 2u32)
        };
        let offset = mul_checked_u32(self.step, half_pair);
        if offset > self.sum { halt(0xFF06u16); }
        let rem = self.sum - offset;
        if rem % self.n != 0u32 { halt(0xFF06u16); }
        self.first = rem / self.n;
        1u16
    }
}
