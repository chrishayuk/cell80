//! Excel DURATION(settlement, maturity, coupon, yld, frequency, [basis]): the Macaulay duration itself -- the present-value-weighted average time (in years) to a bond's cash flows -- MDURATION's undivided sibling (MDURATION = DURATION / (1+yld/frequency)), but computed in CLOSED FORM rather than mduration's per-period walk: with x = 1/(1+y/f), the geometric identities sum_{k=1..N} x^k = x(1-x^N)/(1-x) and sum k*x^k = x(1-(N+1)x^N+N*x^(N+1))/(1-x)^2 collapse the whole coupon strip to one x^N (square-and-multiply, the pack's excel_oddfyield/excel_nominal idiom) plus a dozen f32 ops -- O(log N), which is why this cell prices ANY realistic schedule inside the default cycle budget while a per-period walk tops out near 60. Consumes num_periods (a COUPNUM-style whole-period count) the way mduration does -- settlement/maturity/basis resolve upstream -- and carries the same whole-period scope (settlement on a coupon date, no fractional first period). Zero-coupon (coupon == 0) degenerates exactly to duration = N/frequency years.
//! tags: excel, duration, macaulay-duration, bond, coupon, yield, frequency, weighted-average-life, present-value, discounted-cash-flow, closed-form, geometric-series, finance, f32
//! kernel_bank: on
//! entry: ExcelDuration::run
//! limits: escalates (halt 0xFF06, out_of_domain) if frequency isn't 1, 2, or 4; if num_periods is 0 or exceeds 2400 (mduration's own domain cap -- affordable here because the closed form is O(log N)); if coupon < 0; or if yld <= 0 (yld == 0 makes the geometric denominators vanish; Excel's DURATION likewise #NUM!s at non-positive yield); escalates (halt 0xFF07, float_overflow) if the result is infinite, (halt 0xFF08, float_domain) if it's NaN
struct ExcelDuration {
    num_periods: u16,
    coupon: f32,
    yld: f32,
    frequency: u16,
    duration: f32,
}
impl ExcelDuration {
    fn run(&mut self) -> u16 {
        let freq_ok = self.frequency == 1u16 || self.frequency == 2u16 || self.frequency == 4u16;
        if !freq_ok { halt(0xFF06u16); }
        if self.num_periods == 0u16 { halt(0xFF06u16); }
        if self.num_periods > 2400u16 { halt(0xFF06u16); }
        if self.coupon < 0.0f32 { halt(0xFF06u16); }
        if self.yld <= 0.0f32 { halt(0xFF06u16); }

        let freq_f = int_to_f32(self.frequency);
        let c = (100.0f32 * self.coupon) / freq_f;
        let x = 1.0f32 / (1.0f32 + self.yld / freq_f);

        // x^N by square-and-multiply over the bits of N (<= 12 squarings).
        let mut xn = 1.0f32;
        let mut sq = x;
        let mut n = self.num_periods;
        while n != 0u16 {
            if (n & 1u16) == 1u16 {
                xn = xn * sq;
            }
            sq = sq * sq;
            n = n >> 1u16;
        }

        let nf = int_to_f32(self.num_periods);
        let one_minus_x = 1.0f32 - x;
        // sum_{k=1..N} x^k and sum_{k=1..N} k*x^k, closed form.
        let sum_x = x * (1.0f32 - xn) / one_minus_x;
        let sum_kx = x * (1.0f32 - (nf + 1.0f32) * xn + nf * (xn * x)) / (one_minus_x * one_minus_x);

        let price = c * sum_x + 100.0f32 * xn;
        let weighted_periods = c * sum_kx + nf * (100.0f32 * xn);
        let mac = weighted_periods / (price * freq_f);

        if mac.is_nan() { halt(0xFF08u16); }
        let fin = mac.is_finite();
        if !fin { halt(0xFF07u16); }

        self.duration = mac;
        1u16
    }
}
