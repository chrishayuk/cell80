//! Nominal annual rate implied by an effective annual rate and a compounding-periods-per-year count -- NOMINAL(effect_rate, npery) = npery*((1+effect_rate)^(1/npery) - 1), solved via Newton-Raphson Nth-root extraction on x^npery = 1+effect_rate since no fractional-exponent pow exists; both arguments are required with no Excel-side default, and neither an outflow-negative sign convention nor an annuity `type` flag applies since NOMINAL is a pure rate-conversion, not a cash-flow schedule -- distinct from EFFECT (compound_increase_by_bps composing the SAME forward integer power), this is the pack's only cell that inverts one.
//! tags: excel, finance, nominal, effect, effective-rate, nominal-rate, annual-rate, compounding, npery, compounding-periods, newton-raphson, nth-root, inverse, rate-conversion, f32, float, softfloat
//! kernel_bank: on
//! entry: ExcelNominal::run
//! limits: escalates (halt 0xFF06, out_of_domain) if npery == 0 or effect_rate <= 0.0 (Excel's #NUM! domain: effect_rate must be > 0, npery must be >= 1); escalates (halt 0xFF08, float_domain) on a NaN result, (halt 0xFF07, float_overflow) on a non-finite one; Newton-Raphson runs a fixed 10 iterations from x0 = 1.0 -- quadratic convergence reaches f32's fixed point in 3-4 steps for every case tried, npery from 1 to 10000. x^npery (and its derivative npery*x^(npery-1)) are evaluated via binary exponentiation (O(log2 npery) multiplies), not a flat O(npery) loop -- the original draft's linear loop measured at ~2.16M cycles for npery=4 alone (over the 2,000,000 default budget for every npery >= 4), the same class of fix excel_db/excel_rri's own doc comments already apply to this exact Newton-nth-root shape.
struct ExcelNominal {
    effect_rate: f32,
    npery: u16,
    nominal_rate: f32,
}
impl ExcelNominal {
    fn run(&mut self) -> u16 {
        if self.npery == 0u16 {
            halt(0xFF06u16);
        }
        if self.effect_rate <= 0.0f32 {
            halt(0xFF06u16);
        }
        let c = 1.0f32 + self.effect_rate;
        let npery_f = int_to_f32(self.npery);
        let exponent = self.npery - 1u16;
        let mut x = 1.0f32;
        let mut iter = 0u16;
        while iter < 10u16 {
            // xn = x^npery via binary exponentiation: bit-test-and-square, folding x
            // into xn whenever the matching bit of `exponent` is set, then one more
            // multiply by x to go from x^(npery-1) to x^npery -- avoids the O(npery)
            // repeated-multiply loop the original draft used.
            let mut xnm1 = 1.0f32;
            let mut base = x;
            let mut e = exponent;
            while e > 0u16 {
                let bit_set = e & 1u16;
                if bit_set == 1u16 {
                    xnm1 = xnm1 * base;
                }
                base = base * base;
                e = e >> 1u16;
            }
            let xn = xnm1 * x;
            // d = npery * x^(npery-1), the exact real-number value the old O(npery)
            // repeated-addition loop summed one term at a time.
            let d = npery_f * xnm1;
            let fx = xn - c;
            let delta = fx / d;
            x = x - delta;
            iter = iter + 1u16;
        }
        let dx = x - 1.0f32;
        // nominal = npery * dx, the exact real-number value the old O(npery)
        // repeated-addition loop summed one term at a time.
        let nominal = npery_f * dx;
        if nominal.is_nan() {
            halt(0xFF08u16);
        }
        let fin = nominal.is_finite();
        if !fin {
            halt(0xFF07u16);
        }
        self.nominal_rate = nominal;
        1u16
    }
}
