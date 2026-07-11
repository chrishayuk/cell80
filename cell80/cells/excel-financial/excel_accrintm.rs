//! ACCRINTM(issue,settlement,rate,par,[basis]) = par*rate*YEARFRAC(issue,settlement,basis) -- par is optional (Excel default 1000 if omitted) and basis is optional (Excel default 0 = US 30/360); the caller resolves basis by running the matching day_count_* cell over (issue,settlement) first and feeds the resulting fraction in as year_frac since this cell only performs the final multiply, distinct from ACCRINT's per-coupon accrual loop because ACCRINTM is one lump sum paid at maturity with no first_interest date or frequency argument at all.
//! tags: excel, bond, accrued-interest, accrintm, maturity, single-payment, lump-sum, par-value, coupon-rate, year-fraction, day-count-dispatch
//! entry: ExcelAccrintm::run
//! limits: escalates (halt 0xFF06, out_of_domain) if rate <= 0, par <= 0, or year_frac < 0 (mirrors Excel's #NUM! for rate<=0/par<=0, plus a defensive guard on a negative accrual fraction); escalates (halt 0xFF08, float_domain) if the product is NaN; escalates (halt 0xFF07, float_overflow) if the product is infinite
struct ExcelAccrintm {
    par: f32,
    rate: f32,
    year_frac: f32,
    accrued_interest: f32,
}
impl ExcelAccrintm {
    fn run(&mut self) -> u16 {
        if self.rate <= 0.0f32 || self.par <= 0.0f32 || self.year_frac < 0.0f32 {
            halt(0xFF06u16);
        }
        let result = self.par * self.rate * self.year_frac;
        if result.is_nan() {
            halt(0xFF08u16);
        }
        let fin = result.is_finite();
        if !fin {
            halt(0xFF07u16);
        }
        self.accrued_interest = result;
        1u16
    }
}
