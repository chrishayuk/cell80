//! Excel-compatible RATE: solves pv*(1+r)^nper + pmt*(1+r*type)*((1+r)^nper-1)/r + fv = 0 for the periodic interest rate r via a bounded secant iteration (bootstrapped from guess and guess+0.0001, up to 20 steps, converged once successive rate estimates differ by less than 0.000001) -- unlike frac_pow's fixed known exponent or bps_rate_over_2_periods's closed-form sqrt (only valid at exactly 2 periods), no closed form exists for an arbitrary whole nper, so this is cell80's first genuine root-finding iteration over the IEEE binary32 tier rather than a checked-integer compounding loop.
//! tags: excel, rate, interest-rate, interest, annuity, tvm, time-value-of-money, nper, pmt, pv, fv, secant, newton, newton-raphson, iteration, root-finding, converge, convergence, f32, float, softfloat, ordinary-annuity, annuity-due, financial, loan
//! kernel_bank: on
//! entry: ExcelRate::run
//! limits: nper is a whole number of periods (u16 loop bound -- cell80 has no fractional-exponent pow, so unlike Excel's real-valued nper this only covers whole-period annuities); Excel's omittable args map here as fv=0, type=0 (ordinary annuity), guess=0.1 when the caller omits them -- pv/pmt/fv keep Excel's cash-flow sign convention (money received positive, money paid out negative) and type=1 (annuity-due, payment at period start) multiplies the annuity factor by (1+r) where type=0 (ordinary annuity, payment at period end) uses a bare 1; escalates (halt 0xFF06, out_of_domain) if nper == 0, if the secant step's denominator is exactly 0.0, or if 20 iterations pass without converging; escalates (halt 0xFF08/0xFF07, float_domain/float_overflow) if any candidate rate or residual goes NaN or non-finite before convergence.
struct ExcelRate {
    nper: u16,
    pmt: f32,
    pv: f32,
    fv: f32,
    typ: u16,
    guess: f32,
    rate: f32,
}
impl ExcelRate {
    fn run(&mut self) -> u16 {
        if self.nper == 0u16 {
            halt(0xFF06u16);
        }

        let mut rp = self.guess;
        let mut rc = self.guess + 0.0001f32;

        let mut pw1 = 1.0f32;
        let mut j1 = 0u16;
        while j1 < self.nper {
            pw1 = pw1 * (1.0f32 + rp);
            j1 = j1 + 1u16;
        }
        let tm1 = if self.typ == 0u16 { 1.0f32 } else { 1.0f32 + rp };
        let mut fp = self.pv * pw1 + self.pmt * tm1 * ((pw1 - 1.0f32) / rp) + self.fv;

        let mut pw2 = 1.0f32;
        let mut j2 = 0u16;
        while j2 < self.nper {
            pw2 = pw2 * (1.0f32 + rc);
            j2 = j2 + 1u16;
        }
        let tm2 = if self.typ == 0u16 { 1.0f32 } else { 1.0f32 + rc };
        let mut fc = self.pv * pw2 + self.pmt * tm2 * ((pw2 - 1.0f32) / rc) + self.fv;

        let mut i = 0u16;
        while i < 20u16 {
            let denom = fc - fp;
            if denom == 0.0f32 {
                halt(0xFF06u16);
            }
            let rn = rc - fc * (rc - rp) / denom;

            let mut pw3 = 1.0f32;
            let mut j3 = 0u16;
            while j3 < self.nper {
                pw3 = pw3 * (1.0f32 + rn);
                j3 = j3 + 1u16;
            }
            let tm3 = if self.typ == 0u16 { 1.0f32 } else { 1.0f32 + rn };
            let fnv = self.pv * pw3 + self.pmt * tm3 * ((pw3 - 1.0f32) / rn) + self.fv;

            if fnv.is_nan() || rn.is_nan() {
                halt(0xFF08u16);
            }
            let fin1 = fnv.is_finite();
            let fin2 = rn.is_finite();
            if !fin1 || !fin2 {
                halt(0xFF07u16);
            }

            let diff = rn - rc;
            let adiff = diff.abs();
            if adiff < 0.000001f32 {
                self.rate = rn;
                return 1u16;
            }

            rp = rc;
            fp = fc;
            rc = rn;
            fc = fnv;
            i = i + 1u16;
        }

        halt(0xFF06u16);
        0u16
    }
}
