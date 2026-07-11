//! Primality check via Wilson's theorem: returns 1 if (n-1)! mod n == n-1 (equivalently (n-1)! == -1 mod n), else 0, for n >= 2 (n < 2 returns 0) — computed as a running product mod n at u32 width per multiply step, an alternative primality witness distinct from is_prime's trial division, in the same cross-check spirit as order_modulo and discrete_log_naive.
//! tags: number, prime, primality, predicate, wilson, factorial, modular, modulo, witness, math
fn run(n: u16) -> u16 {
    let mut ok = 0u16;
    if n >= 2u16 {
        let mut prod = 1u32;
        let mut k = 2u16;
        while k < n {
            prod = prod * k as u32 % n as u32;
            k = k + 1u16;
        }
        if prod == (n - 1u16) as u32 { ok = 1u16; }
    }
    ok
}
