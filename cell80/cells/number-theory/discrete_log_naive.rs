//! Discrete logarithm by brute-force search: the smallest k in [0, max_exp) with base^k == target (mod m). Genuinely bounded by the caller-supplied max_exp (unlike a general discrete-log solve, which is believed hard) -- a plan verifier's "does this exponent exist within a reasonable search window" check.
//! tags: number, discrete, logarithm, modular, modulo, exponent, search, bounded
//! entry: DiscreteLogNaive::run
//! limits: escalates (halt 0xFF06, out_of_domain) if m < 2 or no k in [0, max_exp) satisfies base^k == target (mod m)
struct DiscreteLogNaive { base: u16, target: u16, m: u16, max_exp: u16, k: u16 }
impl DiscreteLogNaive {
    fn run(&mut self) -> u16 {
        if self.m < 2u16 { halt(0xFF06u16); }
        let b = self.base % self.m;
        let t = self.target % self.m;
        let mut cur = 1u16 % self.m;
        let mut k = 0u16;
        while k < self.max_exp {
            if cur == t {
                self.k = k;
                return 1u16;
            }
            let prod = cur as u32 * b as u32;
            cur = (prod % self.m as u32) as u16;
            k = k + 1u16;
        }
        halt(0xFF06u16);
        0u16
    }
}
