//! Cumulative principal repaid on an amortizing loan between start_period and end_period inclusive (all six Excel arguments required, none omittable): rate/nper/pv/start_period/end_period/type match Excel's own CUMPRINC exactly -- pv is entered positive (amount borrowed) and the result comes back negative (cash paid out, the same outflow convention PMT/IPMT/PPMT use), type=0 is an ordinary annuity (payment at period end) and type=1 is an annuity-due (payment at period start, so period 1 carries no interest) -- shares CUMIPMT's identical running balance/interest loop but accumulates ppmt = pmt-ipmt (the principal remainder) each step instead of ipmt (interest), the one deliberate line-swap Excel's own function pair hinges on.
//! tags: excel, finance, loan, amortization, amortizing, cumulative, principal, cumprinc, annuity, payment, ppmt, pmt, running-balance, outflow, f32
//! kernel_bank: on
//! entry: ExcelCumprinc::run
//! limits: escalates (halt 0xFF06, out_of_domain) if rate<=0, pv<=0, nper==0, start_period==0, start_period>end_period, end_period>nper, or type not in {0,1}; escalates (halt 0xFF08, float_domain) on a NaN result and (halt 0xFF07, float_overflow) on a non-finite one -- e.g. a rate/nper combination whose (1+rate)^nper compounding loop overflows f32
struct ExcelCumprinc {
    rate: f32,
    nper: u16,
    pv: f32,
    start_period: u16,
    end_period: u16,
    pmt_type: u16,
    cum_principal: f32,
}
impl ExcelCumprinc {
    fn run(&mut self) -> u16 {
        if self.rate <= 0.0f32 {
            halt(0xFF06u16);
        }
        if self.pv <= 0.0f32 {
            halt(0xFF06u16);
        }
        if self.nper == 0u16 {
            halt(0xFF06u16);
        }
        if self.start_period == 0u16 {
            halt(0xFF06u16);
        }
        if self.start_period > self.end_period {
            halt(0xFF06u16);
        }
        if self.end_period > self.nper {
            halt(0xFF06u16);
        }
        if self.pmt_type != 0u16 && self.pmt_type != 1u16 {
            halt(0xFF06u16);
        }

        // growth = (1+rate)^nper via a bounded repeated-multiply loop -- frac_pow's
        // idiom (cell80/cells/fractions/frac_pow.rs), carried out in f32 here since
        // rate/pv are real-valued rather than checked integers.
        let base = 1.0f32 + self.rate;
        let mut growth = 1.0f32;
        let mut i = 0u16;
        while i < self.nper {
            growth = growth * base;
            i = i + 1u16;
        }

        // Fixed payment for the whole loan (Excel's PMT, fv=0): pmt = -pv*rate*growth /
        // ((growth-1)*(1+rate*type)) -- type=1 (annuity-due) folds in the extra `base`
        // factor since the payment lands one period earlier.
        let denom_factor = if self.pmt_type == 1u16 { base } else { 1.0f32 };
        let pmt = -(self.pv * self.rate * growth) / ((growth - 1.0f32) * denom_factor);

        // Same running-balance loop CUMIPMT walks: for each period, interest = balance
        // before the payment (zero for period 1 under annuity-due, since that payment
        // lands before any interest accrues), principal = pmt - interest, balance moves
        // by the principal each step. Only periods >= start_period are accumulated.
        let mut balance = self.pv;
        let mut cum = 0.0f32;
        let mut period = 1u16;
        while period <= self.end_period {
            let due_first = self.pmt_type == 1u16 && period == 1u16;
            let ipmt = if due_first { 0.0f32 } else { -(balance * self.rate) };
            let ppmt = pmt - ipmt;
            balance = balance + ppmt;
            if period >= self.start_period {
                cum = cum + ppmt;
            }
            period = period + 1u16;
        }

        if cum.is_nan() {
            halt(0xFF08u16);
        }
        let fin = cum.is_finite();
        if !fin {
            halt(0xFF07u16);
        }
        self.cum_principal = cum;
        1u16
    }
}
