//! The nth Motzkin number M(n) -- ways to draw non-crossing chords among n points on a circle (1, 1, 2, 4, 9, 21, 51, ...), via the exact recurrence (k+4)*M(k+2) = (2k+5)*M(k+1) + 3*(k+1)*M(k), M(0)=M(1)=1, checked: escalates instead of silently wrapping -- distinct from catalan_number's one-term recurrence, fibonacci/tribonacci's plain linear ones, and lucas_u_v's two-parameter family.
//! tags: number, motzkin, combinatorics, sequence, counting, chords, checked, wide, u32, escalate
//! entry: MotzkinNumber::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if an intermediate product/sum overflows u32, or if the recurrence's division would not land evenly -- this can trigger before M(n) itself would exceed u32::MAX (n = 25 is the first to trigger this, even though safe n tops out at 24)
struct MotzkinNumber { n: u32, result: u32 }
impl MotzkinNumber {
    fn run(&mut self) -> u16 {
        if self.n == 0u32 || self.n == 1u32 {
            self.result = 1u32;
            return 1u16;
        }
        let mut m0 = 1u32; // M(k)
        let mut m1 = 1u32; // M(k+1)
        let mut k = 0u32;
        while k + 2u32 <= self.n {
            let denom = k + 4u32;
            let coeff_a = 2u32 * k + 5u32;
            let coeff_b = 3u32 * (k + 1u32);
            let part_a = mul_checked_u32(coeff_a, m1);
            let part_b = mul_checked_u32(coeff_b, m0);
            let numer = add_checked_u32(part_a, part_b);
            if numer % denom != 0u32 { halt(0xFF05u16); }
            let m2 = numer / denom;
            m0 = m1;
            m1 = m2;
            k = k + 1u32;
        }
        self.result = m1;
        1u16
    }
}
