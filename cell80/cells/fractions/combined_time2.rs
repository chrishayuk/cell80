//! Combined completion time of two agents working simultaneously, each able to finish a task alone in t1 and t2 time units: t1*t2/(t1+t2), returned as an exact fraction reduced via gcd_u32 -- the classic 'two pipes fill a tank together' parallel-rate word problem, distinct from ratio_split2's additive split and frac_avg2's averaging.
//! tags: fraction, frac, rate, combined, parallel, harmonic, work, time, wide, u32, checked, escalate
//! entry: CombinedTime2::run
//! limits: escalates (halt 0xFF06, out_of_domain) if t1 == 0 or t2 == 0; escalates (halt 0xFF05, needs_wider_math) if t1*t2 overflows u32 (mul_checked_u32) or t1+t2 overflows u32 (add_checked_u32)
struct CombinedTime2 { t1: u32, t2: u32, num: u32, den: u32 }
impl CombinedTime2 {
    fn run(&mut self) -> u16 {
        if self.t1 == 0u32 || self.t2 == 0u32 { halt(0xFF06u16); }
        let num_raw = mul_checked_u32(self.t1, self.t2);
        let den_raw = add_checked_u32(self.t1, self.t2);
        let g = gcd_u32(num_raw, den_raw);
        self.num = num_raw / g;
        self.den = den_raw / g;
        1u16
    }
}
