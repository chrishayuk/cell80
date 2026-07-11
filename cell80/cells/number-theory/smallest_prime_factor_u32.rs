//! Smallest prime factor of n (n >= 2) at wide u32 width — the wide sibling of smallest_prime_factor (u16 domain, cap 256), running the same trial-division loop with the ceiling raised to 65536 (matching is_prime_u32's bound, sufficient since 65536*65536 exceeds u32::MAX). Returns n itself if n is prime.
//! tags: number, prime, factor, smallest, divisor, factorization, wide, u32, large, number-theory
//! entry: SmallestPrimeFactorWide::run
//! limits: escalates (halt 0xFF06, out_of_domain) if n < 2; correct for the full u32 domain, but cost scales with sqrt(n) — n near u32::MAX needs a cycle budget far above the 2,000,000 default
struct SmallestPrimeFactorWide { n: u32, result: u32 }
impl SmallestPrimeFactorWide {
    fn run(&mut self) -> u16 {
        if self.n < 2u32 { halt(0xFF06u16); }
        let mut d = 2u32;
        let mut found = 0u32;
        while d < 65536u32 && d * d <= self.n && found == 0u32 {
            if self.n % d == 0u32 { found = d; }
            d = d + 1u32;
        }
        let r = if found != 0u32 { found } else { self.n };
        self.result = r;
        1u16
    }
}
