//! Excel STDEV.S(number1, [number2], ...): sample standard deviation of up to 16 values -- the Bessel-corrected estimate of a population's spread from a SAMPLE, dividing the sum of squared deviations from the mean by n-1 (not n) since the sample's own mean already spends one degree of freedom estimating the true population mean before the spread around it is measured -- a two-pass computation over the same u32[16] bit-pattern array envelope excel_npv.rs established (first pass sums the values to get the mean, second pass sums squared deviations from that mean), then a single .sqrt() call over the Bessel-corrected variance routes through the shipped fsqrt kernel exactly as excel_sqrt.rs does. Distinct from STDEV.P (divides the same squared-deviation sum by n, the population convention, always yielding a smaller number than this cell's n-1 divisor) and from a bare variance (this cell's own denom = sq_sum/(n-1) is one sqrt short of the standard deviation STDEV.S actually returns).
//! tags: excel, stdev.s, stdev-s, sample-standard-deviation, standard-deviation, sample, bessel-correction, degrees-of-freedom, variance, dispersion, array, statistics, f32, math-trig
//! kernel_bank: on
//! entry: ExcelStdevS::run
//! limits: fixed 16-slot value envelope, not caller-configurable (the array-state envelope wall, same as excel_npv's cash-flow array -- Excel's own STDEV.S is uncapped up to 255 arguments); escalates (halt 0xFF06, out_of_domain) if count < 2 (Bessel's n-1 divisor is undefined at n=1 and meaningless at n=0, exactly Excel's own #DIV/0! for STDEV.S with fewer than 2 numbers) or count > 16; escalates (halt 0xFF08, float_domain) on a NaN result, (halt 0xFF07, float_overflow) on a non-finite one
struct ExcelStdevS {
    values: [u32; 16],
    count: u16,
    result: f32,
}
impl ExcelStdevS {
    fn run(&mut self) -> u16 {
        if self.count < 2u16 { halt(0xFF06u16); }
        if self.count > 16u16 { halt(0xFF06u16); }

        // Pass 1: mean over the live slots.
        let mut sum = 0.0f32;
        let mut i = 0u16;
        while i < self.count {
            let v = f32_from_bits(self.values[i as usize]);
            sum = sum + v;
            i = i + 1u16;
        }
        let n_f = int_to_f32(self.count as u32);
        let mean = sum / n_f;

        // Pass 2: sum of squared deviations from that mean.
        let mut sq_sum = 0.0f32;
        let mut j = 0u16;
        while j < self.count {
            let v = f32_from_bits(self.values[j as usize]);
            let d = v - mean;
            sq_sum = sq_sum + d * d;
            j = j + 1u16;
        }

        // Bessel's correction: divide by n-1, not n (count is already >= 2 here,
        // so denom is always >= 1.0 and never zero).
        let denom = n_f - 1.0f32;
        let variance = sq_sum / denom;
        let sd = variance.sqrt();

        if sd.is_nan() { halt(0xFF08u16); }
        let fin = sd.is_finite();
        if !fin { halt(0xFF07u16); }
        self.result = sd;
        1u16
    }
}
