//! k! mod m: a general bounded factorial-mod utility, computed as a running product mod m at u32 width per step (0 if m == 0, matching pow_mod's own m == 0 convention) — distinct from pow_mod (which exponentiates a fixed base, not an accumulating product of 1..k) and from wilson_theorem_check (which fixes k = n-1 and adds the final -1 comparison); this is the plain two-arg (k, m) building block underneath both. Exits as soon as the running product hits 0 (once i reaches any factor sharing a multiple of m), so cost only scales with min(k, the point where a zero factor appears), not always the full k.
//! tags: number, modular, factorial, mod, running-product, wilson, bounded, number-theory
fn run(k: u16, m: u16) -> u16 {
    let mut r = 0u16;
    if m != 0u16 {
        r = 1u16 % m;
        let mut i = 1u16;
        while i <= k && r != 0u16 {
            let im = i % m;
            let prod = (r as u32) * (im as u32);
            r = (prod % (m as u32)) as u16;
            i = i + 1u16;
        }
    }
    r
}
