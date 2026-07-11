//! Excel PRICE(settlement, maturity, rate, yld, redemption, frequency, [basis]): price per $100 face of a regular-coupon bond settling BETWEEN coupon dates -- redemption and every coupon discount at the TRUE fractional exponents (k - 1 + dsc/e), the fractional first-period offset running through the owned F2 fpow once (this is exactly the wall docs/excel-financial-map.md's own note said kept PRICE host_only; the integer powers chain off it with one running multiply per period, mduration's idiom), minus the accrued-interest rebate 100*(rate/f)*a/e. Consumes the caller-computed day-count fractions dsc_over_e and a_over_e plus a COUPNUM-style num_periods, the same upstream feed-in convention as ACCRINT/MDURATION/ODDLYIELD -- raw dates and basis never enter this cell. Distinct from PRICEDISC (zero-coupon discount security, no coupon walk), PRICEMAT (interest at maturity, single period) and ODDFPRICE/ODDLPRICE (odd first/last stub periods; this cell is the REGULAR-schedule case).
//! tags: excel, price, bond, bond-price, coupon, yield, redemption, frequency, day-count, dsc, fractional-period, transcendental, pow, dirty-price, clean-price, finance, f32
//! kernel_bank: on
//! entry: ExcelPrice::run
//! accuracy: <= ~45 ulp worst case (one fpow at <= 41 ulp over its declared domain, the rest correctly-rounded walks; rustz80's F2 harness pins the kernel)
//! limits: escalates (halt 0xFF06, out_of_domain) if rate < 0, yld < 0, redemption <= 0, frequency isn't 1/2/4, dsc_over_e <= 0 or > 1, a_over_e < 0 or > 1, num_periods == 0, or num_periods > 60 (the per-period discount walk against the cycle budget once the fpow is paid -- 15 years of quarterly coupons; longer schedules escalate rather than run over); escalates (halt 0xFF07, float_overflow) if the result is infinite, (halt 0xFF08, float_domain) if it's NaN
struct ExcelPrice {
    rate: f32,
    yld: f32,
    redemption: f32,
    frequency: u16,
    num_periods: u16,
    dsc_over_e: f32,
    a_over_e: f32,
    price: f32,
}
impl ExcelPrice {
    fn run(&mut self) -> u16 {
        if self.rate < 0.0f32 { halt(0xFF06u16); }
        if self.yld < 0.0f32 { halt(0xFF06u16); }
        if self.redemption <= 0.0f32 { halt(0xFF06u16); }
        let freq_ok = self.frequency == 1u16 || self.frequency == 2u16 || self.frequency == 4u16;
        if !freq_ok { halt(0xFF06u16); }
        if self.dsc_over_e <= 0.0f32 { halt(0xFF06u16); }
        if self.dsc_over_e > 1.0f32 { halt(0xFF06u16); }
        if self.a_over_e < 0.0f32 { halt(0xFF06u16); }
        if self.a_over_e > 1.0f32 { halt(0xFF06u16); }
        if self.num_periods == 0u16 { halt(0xFF06u16); }
        if self.num_periods > 60u16 { halt(0xFF06u16); }

        let freq_f = int_to_f32(self.frequency);
        let c = (100.0f32 * self.rate) / freq_f;
        let base = 1.0f32 + self.yld / freq_f;

        // The one fractional exponent: (1+y/f)^(dsc/e); every later power chains
        // integer steps off it with a running divide-as-multiply.
        let frac = base.powf(self.dsc_over_e);
        let inv = 1.0f32 / base;
        let mut df = 1.0f32 / frac;

        let mut acc = 0.0f32;
        let mut k = 1u16;
        while k <= self.num_periods {
            let is_last = k == self.num_periods;
            let redemption_cf = if is_last { self.redemption } else { 0.0f32 };
            let cf = c + redemption_cf;
            acc = acc + cf * df;
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
