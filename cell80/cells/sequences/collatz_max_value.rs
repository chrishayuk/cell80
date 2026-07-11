//! The maximum (peak) value reached in the Collatz (3n+1 / n/2) trajectory from n down to 1, bounded by max_steps -- the same walk collatz_stopping_time runs, tracking a running max instead of a step count.
//! tags: number, collatz, sequence, hailstone, peak, max, iterate, steps, bounded, checked, wide, u32, escalate
//! entry: CollatzMaxValue::run
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0, or if 1 is not reached within max_steps; escalates (halt 0xFF05, needs_wider_math) if 3v+1 or the running max overflows u32
struct CollatzMaxValue { n: u32, max_steps: u32, max_value: u32 }
impl CollatzMaxValue {
    fn run(&mut self) -> u16 {
        if self.n == 0u32 { halt(0xFF06u16); }
        let mut v = self.n;
        let mut peak = self.n;
        let mut count = 0u32;
        while v != 1u32 {
            if count >= self.max_steps { halt(0xFF06u16); }
            let nv = if v % 2u32 == 0u32 {
                v / 2u32
            } else {
                add_checked_u32(mul_checked_u32(v, 3u32), 1u32)
            };
            v = nv;
            let np = if v > peak { v } else { peak };
            peak = np;
            count = count + 1u32;
        }
        self.max_value = peak;
        1u16
    }
}
