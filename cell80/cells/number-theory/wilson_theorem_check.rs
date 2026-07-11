//! Primality check via Wilson's theorem: returns 1 if (n-1)! mod n == n-1 (equivalently (n-1)! == -1 mod n), else 0, for n >= 2 (n < 2 returns 0) — computed as a running product mod n at u32 width per multiply step, an alternative primality witness distinct from is_prime's trial division, in the same cross-check spirit as order_modulo and discrete_log_naive.
//! tags: number, prime, primality, predicate, wilson, factorial, modular, modulo, witness, math
//! limits: cost is O(n) multiplies when n is prime (the witness's own nature); most composites zero the running product within a few steps and exit early
fn run(n: u16) -> u16 {
    let mut ok = 0u16;
    // A composite n > 4 always has (n-1)! == 0 (mod n) != n-1, so any cheap
    // compositeness witness answers 0 without the walk — divisibility by
    // 2/3/5 covers 73% of inputs outright.
    let small_factor =
        n > 5u16 && (n % 2u16 == 0u16 || n % 3u16 == 0u16 || n % 5u16 == 0u16);
    if n >= 2u16 && !small_factor {
        let mut prod = 1u32;
        let mut k = 2u16;
        // 0 is absorbing (0 * k mod n stays 0), so a zeroed product exits —
        // remaining composites die at their smallest factor pair; only
        // primes pay the full factorial walk (the witness's own nature).
        while k < n && prod != 0u32 {
            prod = prod * k as u32 % n as u32;
            k = k + 1u16;
        }
        if prod == (n - 1u16) as u32 { ok = 1u16; }
    }
    ok
}
