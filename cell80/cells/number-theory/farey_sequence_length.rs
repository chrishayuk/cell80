//! The length of the Farey sequence of order n: |F_n| = 1 + sum_{k=1}^{n} phi(k), each phi(k) computed inline via euler_totient's own prime-factor-strip loop nested inside an outer k=1..n loop -- the checked O(n*sqrt(n)) running-sum shape square_pyramidal_number established for this pack, applied to the totient instead of the squares.
//! tags: number, farey, sequence, fraction, totient, euler, phi, sum, aggregate, checked, wide, u32, escalate, number-theory
//! entry: FareySequenceLength::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the running sum of totients would exceed u32::MAX
struct FareySequenceLength { n: u16, result: u32 }
impl FareySequenceLength {
    fn run(&mut self) -> u16 {
        let mut sum = 1u32;
        let bound = self.n as u32;
        let mut k = 1u32;
        while k <= bound {
            let kk = k as u16;
            let mut phi = kk;
            let mut m = kk;
            let mut p = 2u16;
            while p < 256u16 && p * p <= m {
                if m % p == 0u16 {
                    phi = phi - phi / p;
                    while m % p == 0u16 { m = m / p; }
                }
                p = p + 1u16;
            }
            if m > 1u16 { phi = phi - phi / m; }
            sum = add_checked_u32(sum, phi as u32);
            k = k + 1u32;
        }
        self.result = sum;
        1u16
    }
}
