//! Convert a decimal price to Excel DOLLARFR's fractional-dollar quoted form (bond/security tick notation, e.g. a price of 1.125 quoted in sixteenths becomes 1.02) -- currency-agnostic despite the Excel name, any decimal/fractional-tick pair works: whole=TRUNC(decimal), remainder=decimal-whole, numerator=ROUND(remainder*fraction) rounded half-away-from-zero, digits=num_digits(fraction) (reuses num_digits' divide-until-zero loop, inlined since cells can't call each other) turned into a power-of-ten divisor via a repeated-multiply loop, result=whole+numerator/10^digits -- both arguments are required in real Excel (no optional defaults, no outflow-negative sign, no annuity-type flag apply to a price conversion), and fraction==0 mirrors Excel's own #DIV/0! -- the inverse of DOLLARDE (fraction-to-decimal, an exact num/den-fraction cell, not this f32 one).
//! tags: excel, dollarfr, dollar, price, fraction, fractional, decimal, tick, ticks, treasury, bond, security, convert, f32, digits, round, truncate
//! kernel_bank: on
//! entry: DollarFr::run
//! limits: escalates (halt 0xFF06, out_of_domain) if fraction == 0 (Excel's #DIV/0!); escalates (halt 0xFF08, float_domain) if the computed result is NaN; escalates (halt 0xFF07, float_overflow) if the computed result is non-finite (e.g. from a non-finite decimal input)
struct DollarFr {
    decimal: f32,
    fraction: u16,
    result: f32,
}
impl DollarFr {
    fn run(&mut self) -> u16 {
        if self.fraction == 0u16 { halt(0xFF06u16); }
        let mut v = self.fraction;
        let mut digits = 1u16;
        while v >= 10u16 {
            digits = digits + 1u16;
            v = v / 10u16;
        }
        let mut pow10 = 1u32;
        let mut i = 0u16;
        while i < digits {
            pow10 = pow10 * 10u32;
            i = i + 1u16;
        }
        let decimal = self.decimal;
        let whole = decimal.trunc();
        let remainder = decimal - whole;
        let fraction_f = int_to_f32(self.fraction);
        let scaled = remainder * fraction_f;
        let numerator = scaled.round();
        let pow10_f = int_to_f32(pow10);
        let frac_part = numerator / pow10_f;
        let result = whole + frac_part;
        if result.is_nan() { halt(0xFF08u16); }
        let fin = result.is_finite();
        if !fin { halt(0xFF07u16); }
        self.result = result;
        1u16
    }
}
