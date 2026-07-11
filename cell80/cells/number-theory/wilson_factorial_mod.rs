//! k! mod m: a general bounded factorial-mod utility, computed as a running product mod m at u32 width per step (0 if m == 0, matching pow_mod's own m == 0 convention) — distinct from pow_mod (which exponentiates a fixed base, not an accumulating product of 1..k) and from wilson_theorem_check (which fixes k = n-1 and adds the final -1 comparison); this is the plain two-arg (k, m) building block underneath both. Answers 0 immediately when k >= m (m itself is then a factor of k!), and otherwise exits as soon as the running product hits 0, so cost only scales with min(k, m).
//! tags: number, modular, factorial, mod, running-product, wilson, bounded, number-theory
fn run(k: u16, m: u16) -> u16 {
    let mut r = 0u16;
    if m != 0u16 {
        r = 1u16 % m;
        if k >= m {
            // m divides k! outright — the old loop reached i == m, multiplied
            // by 0, and exited with 0; the answer needs no walk at all.
            r = 0u16;
        } else {
            let mut i = 1u16;
            while i <= k && r != 0u16 {
                let im = i % m;
                let prod = (r as u32) * (im as u32);
                r = (prod % (m as u32)) as u16;
                i = i + 1u16;
            }
        }
    }
    r
}
