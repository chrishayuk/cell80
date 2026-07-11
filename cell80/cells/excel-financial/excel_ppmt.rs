//! Principal portion of a payment for period `per` of an amortizing loan/annuity, ppmt = pmt - ipmt (rate/per/nper/pv/[fv]/[type] match Excel's own PPMT exactly: fv is omittable and defaults to 0, type is omittable and defaults to 0 = ordinary annuity/payment at period end, 1 = annuity-due/payment at period start with period 1 carrying no interest): pv entered positive (amount borrowed) comes back as a negative pmt/ipmt/ppmt (cash paid out), the same outflow convention IPMT/PMT/CUMPRINC use -- walks the same running-balance/interest loop CUMPRINC/CUMIPMT use but only up to `per`, returning that single period's principal slice instead of an accumulated cumulative sum, and (unlike CUMPRINC/CUMIPMT, which have no such argument) honors PPMT's own optional fv.
//! tags: excel, ppmt, principal, portion, payment, amortization, amortizing, loan, annuity, annuity-due, interest, balance, rate, period, nper, pv, fv, financial, finance, f32
//! kernel_bank: on
//! entry: ExcelPpmt::run
//! limits: escalates (halt 0xFF06, out_of_domain) if nper == 0, per == 0, per > nper, rate <= -1.0, or type not in {0, 1}; escalates (halt 0xFF08, float_domain) on a NaN result and (halt 0xFF07, float_overflow) on a non-finite one
struct ExcelPpmt {
    rate: f32,
    per: u16,
    nper: u16,
    pv: f32,
    fv: f32,
    pmt_type: u16,
    ppmt: f32,
}
impl ExcelPpmt {
    fn run(&mut self) -> u16 {
        if self.nper == 0u16 {
            halt(0xFF06u16);
        }
        if self.per == 0u16 {
            halt(0xFF06u16);
        }
        if self.per > self.nper {
            halt(0xFF06u16);
        }
        if self.rate <= -1.0f32 {
            halt(0xFF06u16);
        }
        if self.pmt_type != 0u16 && self.pmt_type != 1u16 {
            halt(0xFF06u16);
        }

        // growth ends as (1+rate)^nper and sum ends as sum_{k=0}^{nper-1}(1+rate)^k,
        // via one bounded repeated-multiply loop -- frac_pow's idiom, in f32. sum
        // stands in for the usual (growth-1)/rate annuity factor without ever
        // dividing by rate, so rate == 0 needs no special case (growth stays 1.0,
        // sum accumulates to nper exactly).
        let base = 1.0f32 + self.rate;
        let mut growth = 1.0f32;
        let mut sum = 0.0f32;
        let mut i = 0u16;
        while i < self.nper {
            sum = sum + growth;
            growth = growth * base;
            i = i + 1u16;
        }

        // Whole-loan payment (Excel's PMT, fv included): pmt = -(pv*growth+fv)/sum,
        // folded by one more `base` factor for type=1 (annuity-due) -- the same
        // adjustment CUMPRINC's pmt line applies.
        let mut pmt = 0.0f32 - (self.pv * growth + self.fv) / sum;
        if self.pmt_type == 1u16 {
            pmt = pmt / base;
        }

        // Walk the same running-balance loop CUMIPMT/CUMPRINC use, one period at a
        // time up to `per`: interest = balance before the payment (zero for period 1
        // under annuity-due, since that payment lands before any interest accrues),
        // principal = pmt - interest, balance moves by the principal each step.
        let mut balance = self.pv;
        let mut ppmt_at_per = 0.0f32;
        let mut period = 1u16;
        while period <= self.per {
            let due_first = self.pmt_type == 1u16 && period == 1u16;
            let ipmt = if due_first { 0.0f32 } else { 0.0f32 - (balance * self.rate) };
            let ppmt = pmt - ipmt;
            balance = balance + ppmt;
            ppmt_at_per = ppmt;
            period = period + 1u16;
        }

        if ppmt_at_per.is_nan() {
            halt(0xFF08u16);
        }
        let fin = ppmt_at_per.is_finite();
        if !fin {
            halt(0xFF07u16);
        }
        self.ppmt = ppmt_at_per;
        1u16
    }
}
