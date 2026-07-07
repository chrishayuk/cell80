//! Jordan's totient J_k(n): generalizes euler_totient with an exponent k (J_1(n) = phi(n)) -- the product over each prime-power factor p^e of n of p^((e-1)*k) * (p^k - 1). The (e-1)*k exponent is never computed as a scalar product (e up to ~15 times k up to 65535 would overflow u16 before any p^_ term is even reached) -- instead p^((e-1)*k) is built by repeatedly squaring the already-computed p^k value e-1 times, which stays small since e-1 is itself bounded (<= 15 in the u16 domain).
//! tags: number, jordan, totient, phi, generalized, exponent, euler, number-theory
//! entry: JordanTotient::run
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0; escalates (halt 0xFF05, needs_wider_math) if any p^k term, p^((e-1)*k) term, or the running product overflows u32
struct JordanTotient { n: u16, k: u16, result: u32 }
impl JordanTotient {
    fn run(&mut self) -> u16 {
        if self.n == 0u16 { halt(0xFF06u16); }
        let mut result = 1u32;
        let mut m = self.n;
        let mut p = 2u16;
        while p < 256u16 && p * p <= m {
            if m % p == 0u16 {
                let mut e = 0u16;
                while m % p == 0u16 { m = m / p; e = e + 1u16; }
                let mut pk = 1u32;
                let mut i = 0u16;
                while i < self.k { pk = mul_checked_u32(pk, p as u32); i = i + 1u16; }
                let em1 = e - 1u16;
                let mut ppow = 1u32;
                let mut j = 0u16;
                while j < em1 { ppow = mul_checked_u32(ppow, pk); j = j + 1u16; }
                let term = mul_checked_u32(ppow, pk - 1u32);
                result = mul_checked_u32(result, term);
            }
            p = p + 1u16;
        }
        if m > 1u16 {
            let mut pk = 1u32;
            let mut i = 0u16;
            while i < self.k { pk = mul_checked_u32(pk, m as u32); i = i + 1u16; }
            let term = pk - 1u32;
            result = mul_checked_u32(result, term);
        }
        self.result = result;
        1u16
    }
}
