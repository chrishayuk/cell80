//! Cumulative interest paid on a loan between two inclusive periods -- CUMIPMT(rate,nper,pv,start_period,end_period,type): all six arguments required (none omittable), pv positive (amount borrowed) yields a negative result (outflow-negative convention), and type (0=ordinary annuity, pay at period end; 1=annuity-due, pay at period start, first period's interest is 0) selects which per-period balance recurrence runs -- distinct from money-bps's compound_increase_by_bps (single fixed-rate compounding loop, no fixed-payment/declining-balance amortization) and frac_pow (bare integer-power idiom, no balance tracking or period-range accumulation).
//! tags: excel, finance, loan, amortization, cumulative-interest, interest-between-periods, annuity, pmt, balance, period-range, f32, float, softfloat
//! kernel_bank: on
//! entry: ExcelCumipmt::run
//! limits: escalates (halt 0xFF06, out_of_domain) if nper==0, start_period<1, end_period<start_period, end_period>nper, pay_type not in {0,1}, or rate==0 (division by zero in the payment formula); escalates (halt 0xFF08, float_domain) on a NaN result, (halt 0xFF07, float_overflow) on a non-finite result
struct ExcelCumipmt {
    rate: f32,
    nper: u16,
    pv: f32,
    start_period: u16,
    end_period: u16,
    pay_type: u16,
    cum_interest: f32,
}
impl ExcelCumipmt {
    fn run(&mut self) -> u16 {
        if self.nper == 0u16 { halt(0xFF06u16); }
        if self.start_period < 1u16 { halt(0xFF06u16); }
        if self.end_period < self.start_period { halt(0xFF06u16); }
        if self.end_period > self.nper { halt(0xFF06u16); }
        if self.pay_type != 0u16 && self.pay_type != 1u16 { halt(0xFF06u16); }

        let one_plus_rate = 1.0f32 + self.rate;

        // (1+rate)^nper via a bounded repeated-multiply loop (frac_pow's idiom),
        // needed to derive the fixed per-period payment.
        let mut pv_factor = 1.0f32;
        let mut i = 0u16;
        while i < self.nper {
            pv_factor = pv_factor * one_plus_rate;
            i = i + 1u16;
        }

        let denom = pv_factor - 1.0f32;
        if denom == 0.0f32 { halt(0xFF06u16); }

        let mut pmt = -(self.pv * self.rate * pv_factor) / denom;
        if self.pay_type == 1u16 {
            pmt = pmt / one_plus_rate;
        }
        let payment_mag = -pmt;

        // Bounded per-period balance-tracking loop (period 1..nper, 1-indexed):
        // interest_i = balance-before-payment * rate (0 for period 1 under
        // annuity-due, since that payment lands before any interest accrues),
        // principal_i = payment_mag - interest_i, balance -= principal_i.
        // Sum interest_i over [start_period, end_period], then negate to match
        // Excel's outflow-negative sign convention.
        let mut balance = self.pv;
        let mut sum_interest = 0.0f32;
        let mut period = 1u16;
        while period <= self.nper {
            let mut interest = balance * self.rate;
            if self.pay_type == 1u16 && period == 1u16 {
                interest = 0.0f32;
            }
            let principal = payment_mag - interest;
            balance = balance - principal;
            if period >= self.start_period && period <= self.end_period {
                sum_interest = sum_interest + interest;
            }
            period = period + 1u16;
        }

        let result = -sum_interest;
        if result.is_nan() {
            halt(0xFF08u16);
        }
        let fin = result.is_finite();
        if !fin {
            halt(0xFF07u16);
        }
        self.cum_interest = result;
        1u16
    }
}
