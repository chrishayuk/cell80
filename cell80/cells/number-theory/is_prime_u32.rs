//! Returns 1 if n is prime at wide u32 width, else 0 — the wide sibling of is_prime (which works over u16, up to 65535). Trial division scales with sqrt(n): a large prime near u32::MAX needs on the order of tens of millions of cycles, far past the 2,000,000 default — pass a larger --cycles budget explicitly for n much beyond a few million.
//! tags: number, prime, primality, predicate, factor, divisor, wide, u32, large
//! entry: IsPrimeWide::run
//! limits: correct for the full u32 domain, but cost scales with sqrt(n) — n near u32::MAX needs a cycle budget far above the 2,000,000 default
struct IsPrimeWide { n: u32, ok: u16 }
impl IsPrimeWide {
    fn run(&mut self) -> u16 {
        let mut r = 1u16;
        if self.n < 2u32 { r = 0u16; }
        let mut d = 2u32;
        while d < 65536u32 && d * d <= self.n {
            if self.n % d == 0u32 { r = 0u16; }
            d = d + 1u32;
        }
        self.ok = r;
        r
    }
}
