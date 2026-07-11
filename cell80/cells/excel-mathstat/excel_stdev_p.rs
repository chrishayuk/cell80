//! Excel STDEV.P(number1, [number2], ...): population standard deviation of a list of numbers -- squared deviations from the mean divided by n (NOT n-1, unlike the sample form STDEV.S), then square-rooted. A two-pass reduction over the u32[16] array-state envelope excel_npv established (`.cell` v11): pass one sums the values as f32 (each element decoded via f32_from_bits from the host-written f32::to_bits pattern) to get the mean; pass two walks the array again accumulating (value - mean)^2 into a running sum, divides that sum by count for the population variance, then calls .sqrt() -- routed straight through the native fsqrt kernel exactly like excel_sqrt -- for the population standard deviation. `count` names how many of the 16 slots are live; Excel's real STDEV.P is uncapped up to 255 arguments, but this dialect's array-state envelope is fixed at compile time (16 slots, the established precedent) -- the array-state envelope wall, same limitation excel_npv documents. Distinct from STDEV.S (Bessel-corrected, divides the squared-deviation sum by n-1, a separate cell) and from VAR.P (this cell's un-square-rooted intermediate, its own Excel function, not exposed as a second output here).
//! tags: excel, stdev, stdevp, stdev-p, standard-deviation, population-standard-deviation, population, variance, dispersion, array, f32, math-trig, statistics
//! kernel_bank: on
//! entry: ExcelStdevP::run
//! limits: fixed 16-slot envelope, not caller-configurable (the array-state envelope wall); escalates (halt 0xFF06, out_of_domain) if count is 0 or exceeds 16 (a single value, count==1, is valid and returns 0.0, matching Excel); escalates (halt 0xFF08, float_domain) on a NaN result, (halt 0xFF07, float_overflow) on a non-finite one
struct ExcelStdevP {
    values: [u32; 16],
    count: u16,
    result: f32,
}
impl ExcelStdevP {
    fn run(&mut self) -> u16 {
        if self.count == 0u16 { halt(0xFF06u16); }
        if self.count > 16u16 { halt(0xFF06u16); }
        let n = int_to_f32(self.count as u32);

        // Pass one: sum the values to get the mean.
        let mut sum = 0.0f32;
        let mut i = 0u16;
        while i < self.count {
            sum = sum + f32_from_bits(self.values[i as usize]);
            i = i + 1u16;
        }
        let mean = sum / n;

        // Pass two: sum of squared deviations from that mean.
        let mut ssd = 0.0f32;
        let mut j = 0u16;
        while j < self.count {
            let v = f32_from_bits(self.values[j as usize]);
            let dev = v - mean;
            ssd = ssd + dev * dev;
            j = j + 1u16;
        }
        let variance = ssd / n;
        let r = variance.sqrt();
        if r.is_nan() { halt(0xFF08u16); }
        let fin = r.is_finite();
        if !fin { halt(0xFF07u16); }
        self.result = r;
        1u16
    }
}
