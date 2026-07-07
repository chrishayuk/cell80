//! The Carmichael function lambda(n): the exponent of the multiplicative group mod n -- the smallest m such that a^m == 1 (mod n) for every a coprime to n. Computed as the lcm of lambda(p^e) over each prime-power factor of n: lambda(2)=1, lambda(4)=2, lambda(2^e)=2^(e-2) for e>=3; lambda(p^e)=(p-1)*p^(e-1) for odd p (the same formula euler_totient uses at odd prime powers). Every intermediate lcm combination is itself a divisor of the final lambda(n), which is always <= n -- so despite computing at u32 width for safety, nothing in the u16 input domain can actually overflow it (proven, not just unobserved).
//! tags: number, carmichael, lambda, totient, modular, exponent, group, reduced-totient, number-theory
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0
fn run(n: u16) -> u16 {
    if n == 0u16 { halt(0xFF06u16); }
    let mut result = 1u32;
    let mut m = n;
    let mut p = 2u16;
    while p < 256u16 && p * p <= m {
        if m % p == 0u16 {
            let mut e = 0u16;
            while m % p == 0u16 { m = m / p; e = e + 1u16; }
            let mut component = 0u32;
            if p == 2u16 {
                if e == 1u16 {
                    component = 1u32;
                } else if e == 2u16 {
                    component = 2u32;
                } else {
                    let mut c = 1u32;
                    let mut t = 0u16;
                    while t < e - 2u16 { c = mul_checked_u32(c, 2u32); t = t + 1u16; }
                    component = c;
                }
            } else {
                let mut pw = 1u32;
                let mut t = 0u16;
                while t < e - 1u16 { pw = mul_checked_u32(pw, p as u32); t = t + 1u16; }
                component = mul_checked_u32((p - 1u16) as u32, pw);
            }
            let g = gcd_u32(result, component);
            result = mul_checked_u32(result / g, component);
        }
        p = p + 1u16;
    }
    if m > 1u16 {
        let component = (m - 1u16) as u32;
        let g = gcd_u32(result, component);
        result = mul_checked_u32(result / g, component);
    }
    result as u16
}
