//! ODDLPRICE(settlement, maturity, last_interest, rate, yld, redemption, frequency, [basis]): price per $100 face of a security whose LAST coupon period is odd (short or long), in closed form because settlement falls inside that single remaining period -- pr = [redemption + C*(a_over_e + dsc_over_e)] / [1 + (yld/frequency)*dsc_over_e] - C*a_over_e with C = 100*rate/frequency, the exact pricing relationship excel_oddlyield inverts for yield (the two cells are algebraic mirrors: that one solves the rate out of this formula, this one prices with the rate given). Consumes the same caller-computed a_over_e/dsc_over_e day-count fractions (raw dates and basis resolve upstream) and carries the same single-remaining-period scope (a_over_e + dsc_over_e <= 1). Distinct from ODDFPRICE (odd FIRST period -- a full quasi-coupon discounting walk) and plain PRICE (regular schedule, no odd stub).
//! tags: excel, oddlprice, price, odd-coupon, last-coupon, bond, security, day-count, fraction, basis, closed-form, redemption, yield, finance, f32
//! kernel_bank: on
//! entry: ExcelOddlprice::run
//! limits: escalates (halt 0xFF06, out_of_domain) if yld < 0, rate < 0, redemption <= 0, frequency isn't 1/2/4, a_over_e < 0, dsc_over_e <= 0, or a_over_e + dsc_over_e > 1.0 (the multi-quasi-period long-odd case is out of scope, excel_oddlyield's own precedent); escalates (halt 0xFF07, float_overflow) if the result is infinite, (halt 0xFF08, float_domain) if it's NaN
struct ExcelOddlprice {
    rate: f32,
    yld: f32,
    redemption: f32,
    frequency: u16,
    a_over_e: f32,
    dsc_over_e: f32,
    price: f32,
}
impl ExcelOddlprice {
    fn run(&mut self) -> u16 {
        if self.yld < 0.0f32 { halt(0xFF06u16); }
        if self.rate < 0.0f32 { halt(0xFF06u16); }
        if self.redemption <= 0.0f32 { halt(0xFF06u16); }
        let freq_ok = self.frequency == 1u16 || self.frequency == 2u16 || self.frequency == 4u16;
        if !freq_ok { halt(0xFF06u16); }
        if self.a_over_e < 0.0f32 { halt(0xFF06u16); }
        if self.dsc_over_e <= 0.0f32 { halt(0xFF06u16); }
        let dci_over_e = self.a_over_e + self.dsc_over_e;
        if dci_over_e > 1.0f32 { halt(0xFF06u16); }

        let freq_f = int_to_f32(self.frequency);
        let c = (100.0f32 * self.rate) / freq_f;

        let numer = self.redemption + c * dci_over_e;
        let denom = 1.0f32 + (self.yld / freq_f) * self.dsc_over_e;
        let p = numer / denom - c * self.a_over_e;

        if p.is_nan() { halt(0xFF08u16); }
        let fin = p.is_finite();
        if !fin { halt(0xFF07u16); }
        self.price = p;
        1u16
    }
}
