//! Convert a fractional-dollar price (Excel DOLLARDE's ticks-and-fraction notation, e.g. bond quote "102-16" meaning 102 and 16/32nds) already split into whole/frac_digits_value/fraction into its exact decimal value, returned as a reduced num/den fraction (whole*fraction + frac_digits_value)/fraction via the shared gcd_u32 kernel -- both arguments are required in real Excel (no optional defaults, no outflow-negative sign, no annuity type flag apply here) -- reuses num_digits' divide-until-zero digit-count loop (inlined, since cells can't call each other) only to bound frac_digits_value below fraction's own digit width (a stricter, cell80-specific domain check Excel itself skips), distinct from frac_add_whole (identical n/d+whole recombine, no digit-width bound) and from DOLLARFR (the inverse decimal-to-fraction direction, an f32-tier cell, not this one).
//! tags: excel, dollarde, dollar, price, fraction, decimal, tick, ticks, treasury, bond, stock, convert, digit-width
//! entry: ExcelDollarde::run
//! limits: escalates (halt 0xFF06, out_of_domain) if fraction == 0, or if frac_digits_value has more digits than fraction itself (frac_digits_value >= 10^num_digits(fraction)); escalates (halt 0xFF05, needs_wider_math) if whole*fraction or the final sum would overflow u32 (never reachable with u16 inputs in practice, guarded anyway via mul_checked_u32/add_checked_u32 per the checked-arithmetic convention)
struct ExcelDollarde {
    whole: u16,
    frac_digits_value: u16,
    fraction: u16,
    num: u32,
    den: u32,
}
impl ExcelDollarde {
    fn run(&mut self) -> u16 {
        if self.fraction == 0u16 { halt(0xFF06u16); }
        let mut v = self.fraction;
        let mut digits = 1u16;
        while v >= 10u16 {
            digits = digits + 1u16;
            v = v / 10u16;
        }
        let mut limit = 1u32;
        let mut i = 0u16;
        while i < digits {
            limit = limit * 10u32;
            i = i + 1u16;
        }
        let frac_digits32 = self.frac_digits_value as u32;
        if frac_digits32 >= limit { halt(0xFF06u16); }
        let whole32 = self.whole as u32;
        let fraction32 = self.fraction as u32;
        let wd = mul_checked_u32(whole32, fraction32);
        let num_raw = add_checked_u32(wd, frac_digits32);
        if num_raw == 0u32 {
            self.num = 0u32;
            self.den = 1u32;
            return 1u16;
        }
        let g = gcd_u32(num_raw, fraction32);
        self.num = num_raw / g;
        self.den = fraction32 / g;
        1u16
    }
}
