//! ODDFPRICE(settlement, issue, first_coupon, maturity, rate, yld, redemption, frequency, [basis]): price per $100 face of a security whose FIRST coupon period is odd (short) -- the odd stub pays a day-count-scaled first coupon C*dfc_over_e, and every cash flow discounts at the TRUE fractional exponent (k - 1 + dsc/e) through one owned F2 fpow with the integer powers chained off it (excel_oddfyield, authored before F2 landed, had to flatten the odd stub into a whole discounting step because no fractional pow existed -- its own doc comment says so; this cell prices with the real exponent), minus the pre-issue accrued rebate C*a_over_e. Consumes caller-computed fractions dfc_over_e (odd first coupon's day-count scale), dsc_over_e (settlement to first coupon over E), a_over_e (issue to settlement over E) plus a COUPNUM-style num_periods -- raw dates and basis resolve upstream, this pack's universal feed-in convention. Distinct from ODDLPRICE (odd LAST period, single-period closed form, no walk) and plain PRICE (regular schedule, full first coupon).
//! tags: excel, oddfprice, price, odd-coupon, first-coupon, stub, bond, security, day-count, fraction, basis, transcendental, pow, redemption, yield, finance, f32
//! kernel_bank: on
//! entry: ExcelOddfprice::run
//! accuracy: <= ~45 ulp worst case (one fpow at <= 41 ulp over its declared domain, the rest correctly-rounded walks; rustz80's F2 harness pins the kernel)
//! limits: escalates (halt 0xFF06, out_of_domain) if rate < 0, yld < 0, redemption <= 0, frequency isn't 1/2/4, dfc_over_e <= 0 or > 1 (short odd first periods only -- the multi-quasi-period long-odd case is out of scope, excel_oddfyield's precedent), dsc_over_e <= 0 or > 1, a_over_e < 0 or > 1, num_periods == 0, or num_periods > 60 (the discount walk against the cycle budget once the fpow is paid); escalates (halt 0xFF07, float_overflow) if the result is infinite, (halt 0xFF08, float_domain) if it's NaN
struct ExcelOddfprice {
    rate: f32,
    yld: f32,
    redemption: f32,
    frequency: u16,
    num_periods: u16,
    dfc_over_e: f32,
    dsc_over_e: f32,
    a_over_e: f32,
    price: f32,
}
impl ExcelOddfprice {
    fn run(&mut self) -> u16 {
        if self.rate < 0.0f32 { halt(0xFF06u16); }
        if self.yld < 0.0f32 { halt(0xFF06u16); }
        if self.redemption <= 0.0f32 { halt(0xFF06u16); }
        let freq_ok = self.frequency == 1u16 || self.frequency == 2u16 || self.frequency == 4u16;
        if !freq_ok { halt(0xFF06u16); }
        if self.dfc_over_e <= 0.0f32 { halt(0xFF06u16); }
        if self.dfc_over_e > 1.0f32 { halt(0xFF06u16); }
        if self.dsc_over_e <= 0.0f32 { halt(0xFF06u16); }
        if self.dsc_over_e > 1.0f32 { halt(0xFF06u16); }
        if self.a_over_e < 0.0f32 { halt(0xFF06u16); }
        if self.a_over_e > 1.0f32 { halt(0xFF06u16); }
        if self.num_periods == 0u16 { halt(0xFF06u16); }
        if self.num_periods > 60u16 { halt(0xFF06u16); }

        let freq_f = int_to_f32(self.frequency);
        let c = (100.0f32 * self.rate) / freq_f;
        let base = 1.0f32 + self.yld / freq_f;

        // One true fractional exponent (1+y/f)^(dsc/e); integer powers chain off it.
        let frac = base.powf(self.dsc_over_e);
        let inv = 1.0f32 / base;
        let mut df = 1.0f32 / frac;

        let mut acc = 0.0f32;
        let mut k = 1u16;
        while k <= self.num_periods {
            // The odd short first coupon is scaled by its day-count fraction; the
            // rest are full coupons; redemption rides the final period.
            let coupon = if k == 1u16 { c * self.dfc_over_e } else { c };
            let is_last = k == self.num_periods;
            let redemption_cf = if is_last { self.redemption } else { 0.0f32 };
            acc = acc + (coupon + redemption_cf) * df;
            df = df * inv;
            k = k + 1u16;
        }
        let p = acc - c * self.a_over_e;

        if p.is_nan() { halt(0xFF08u16); }
        let fin = p.is_finite();
        if !fin { halt(0xFF07u16); }
        self.price = p;
        1u16
    }
}
