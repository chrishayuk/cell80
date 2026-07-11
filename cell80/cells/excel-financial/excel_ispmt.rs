//! ISPMT(rate, per, nper, pv) -- all four arguments required, no omittable ones: interest owed for period `per` under Excel's straight-line, non-amortized assumption, interest = pv*rate*(per-nper)/nper shrinking linearly from its peak at per=1 to zero at per=nper; pv's outflow-negative Excel sign convention (positive pv/loan-received flips to negative interest-cost, negative pv/amount-invested flips to positive interest-income) is carried as explicit pv_mag/pv_negative and result_mag/result_negative sign-magnitude pairs since i16 fields aren't dialect-legal -- distinct from IPMT, which prices the interest portion of a payment against the true amortized remaining balance instead of this simple straight-line split.
//! tags: excel, finance, ispmt, interest, straight-line, non-amortized, period, loan, bps, sign-magnitude, wide, u32
//! entry: ExcelIspmt::run
//! limits: escalates (halt 0xFF06, out_of_domain) if nper == 0, per == 0, or per > nper (Excel's documented per range is 1..=nper); escalates (halt 0xFF05, needs_wider_math) if pv_mag * rate_bps, or that per-period interest times (nper - per), overflows u32
struct ExcelIspmt {
    pv_mag: u32,
    pv_negative: u16,
    rate_bps: u32,
    per: u16,
    nper: u16,
    result_mag: u32,
    result_negative: u16,
}
impl ExcelIspmt {
    fn run(&mut self) -> u16 {
        if self.nper == 0u16 { halt(0xFF06u16); }
        if self.per == 0u16 { halt(0xFF06u16); }
        if self.per > self.nper { halt(0xFF06u16); }

        let nper32 = self.nper as u32;
        let per32 = self.per as u32;
        let diff = nper32 - per32;

        let per_period_interest = mul_checked_u32(self.pv_mag, self.rate_bps) / 10000u32;
        let mag = mul_checked_u32(per_period_interest, diff) / nper32;

        self.result_mag = mag;
        let flipped = 1u16 - self.pv_negative;
        let sign = if mag == 0u32 { 0u16 } else { flipped };
        self.result_negative = sign;
        1u16
    }
}
