//! Interest PORTION of a given payment period `per` in a fixed-rate, fixed-payment loan/annuity (Excel's IPMT: rate/per/nper/pv required, [fv] defaults to 0, [type] defaults to 0 for an ordinary annuity paid at period-end vs 1 for annuity-due paid at period-start) -- sizes the whole-loan payment via (1+rate)^nper annuity compounding (PMT's own formula), then re-runs that identical compounding to the shorter bound per-1 to get the balance outstanding just before period `per`'s payment, interest = balance*rate (annuity-due divides by (1+rate) and forces period 1 to exactly 0, since no interest has accrued before that first, immediate payment); pv positive (amount borrowed) yields a negative interest_payment (cash paid out), Excel's outflow convention -- distinct from PPMT's principal PORTION of this same payment, from CUMIPMT's SUM of this value across a period range, and from ISPMT's simpler non-amortizing straight-line interest model.
//! tags: excel, financial, ipmt, interest, portion, payment, amortization, amortizing, annuity, loan, rate, period, balance, compounding, tvm, ordinary, due, outflow, f32
//! kernel_bank: on
//! entry: ExcelIpmt::run
//! limits: escalates (halt 0xFF06, out_of_domain) if nper == 0, per == 0, per > nper, or pmt_type not in {0,1}; escalates (halt 0xFF07, float_overflow) / (halt 0xFF08, float_domain) if the compounded result is non-finite or NaN; rate == 0 returns interest_payment = 0 directly without dividing (a zero-rate loan/annuity accrues no interest in any period)
struct ExcelIpmt {
    rate: f32,
    per: u16,
    nper: u16,
    pv: f32,
    fv: f32,
    pmt_type: u16,
    interest_payment: f32,
}
impl ExcelIpmt {
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
        if self.pmt_type != 0u16 && self.pmt_type != 1u16 {
            halt(0xFF06u16);
        }

        if self.rate == 0.0f32 {
            self.interest_payment = 0.0f32;
            return 1u16;
        }

        let due = self.pmt_type == 1u16;
        let base = 1.0f32 + self.rate;

        // growth = (1+rate)^nper via a bounded repeated-multiply loop -- frac_pow's
        // idiom (cell80/cells/fractions/frac_pow.rs), carried out in f32 (the same
        // technique the pack's excel_cumprinc.rs already uses for its own
        // whole-loan payment).
        let mut growth = 1.0f32;
        let mut i = 0u16;
        while i < self.nper {
            growth = growth * base;
            i = i + 1u16;
        }

        // Whole-loan payment (Excel's PMT): pmt = -(fv + pv*growth) * rate /
        // ((growth-1) * denom_factor); denom_factor folds in the extra `base`
        // factor for annuity-due (type=1), since the payment then lands one
        // period earlier than an ordinary annuity's.
        let denom_factor = if due { base } else { 1.0f32 };
        let pmt_num = (self.fv + self.pv * growth) * self.rate;
        let pmt_den = (growth - 1.0f32) * denom_factor;
        let pmt = -(pmt_num / pmt_den);

        // Re-run the SAME compounding loop to the shorter bound (per-1) to get
        // the balance outstanding just before period `per`'s payment.
        let steps = self.per - 1u16;
        let mut growth_p = 1.0f32;
        let mut k = 0u16;
        while k < steps {
            growth_p = growth_p * base;
            k = k + 1u16;
        }

        // Balance just before period `per` (the FV formula evaluated at n=per-1),
        // sign-flipped to Excel's outflow convention (rbl = -balance); the
        // interest portion is that balance times the rate.
        let rbl_inner = self.pv * growth_p + pmt * denom_factor * (growth_p - 1.0f32) / self.rate;
        let rbl = -rbl_inner;

        let mut result = rbl * self.rate;
        if due {
            let adjusted = result / base;
            result = adjusted;
            if self.per == 1u16 {
                result = 0.0f32;
            }
        }

        if result.is_nan() {
            halt(0xFF08u16);
        }
        let fin = result.is_finite();
        if !fin {
            halt(0xFF07u16);
        }
        self.interest_payment = result;
        1u16
    }
}
