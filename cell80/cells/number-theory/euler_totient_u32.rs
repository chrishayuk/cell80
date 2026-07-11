//! Euler's totient (phi) at wide u32 width, via the same prime-factor-strip-and-reduce loop euler_totient uses — the wide sibling of euler_totient (which is bounded to the u16 domain, n <= 65535); trial-division cap raised from 256 to 65536 so every prime factor up to sqrt(u32::MAX) is still found, mirroring is_prime_u32/isqrt_u32/is_square_u32's cap-raise pattern. Trial division scales with sqrt(n): a large prime near u32::MAX needs on the order of tens of thousands of iterations, so pass a larger --cycles budget explicitly for n much beyond a few million.
//! tags: number, totient, euler, phi, coprime, count, wide, u32, large, number-theory
//! entry: EulerTotientWide::run
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0; correct for the full u32 domain, but cost scales with sqrt(n) — n near u32::MAX needs a cycle budget far above the 2,000,000 default
struct EulerTotientWide { n: u32, result: u32 }
impl EulerTotientWide {
    fn run(&mut self) -> u16 {
        if self.n == 0u32 { halt(0xFF06u16); }
        let mut result = self.n;
        let mut m = self.n;
        let mut p = 2u32;
        while p < 65536u32 && p * p <= m {
            if m % p == 0u32 {
                result = result - result / p;
                while m % p == 0u32 { m = m / p; }
            }
            p = p + 1u32;
        }
        if m > 1u32 { result = result - result / m; }
        self.result = result;
        1u16
    }
}
