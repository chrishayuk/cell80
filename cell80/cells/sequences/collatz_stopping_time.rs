//! The number of Collatz (3n+1 / n/2) steps needed to reach 1 from n, bounded by max_steps -- the "count of iterations, not the sequence" shape persistent_digital_root established for digit-summing, applied to the 3n+1 recurrence.
//! tags: number, collatz, sequence, hailstone, stopping-time, iterate, steps, count, bounded, checked, wide, u32, escalate
//! entry: CollatzStoppingTime::run
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0, or if 1 is not reached within max_steps; escalates (halt 0xFF05, needs_wider_math) if 3n+1 overflows u32
struct CollatzStoppingTime { n: u32, max_steps: u32, steps: u32 }
impl CollatzStoppingTime {
    fn run(&mut self) -> u16 {
        if self.n == 0u32 { halt(0xFF06u16); }
        let mut v = self.n;
        let mut count = 0u32;
        while v != 1u32 {
            if count >= self.max_steps { halt(0xFF06u16); }
            let nv = if v % 2u32 == 0u32 {
                v / 2u32
            } else {
                add_checked_u32(mul_checked_u32(v, 3u32), 1u32)
            };
            v = nv;
            count = count + 1u32;
        }
        self.steps = count;
        1u16
    }
}
