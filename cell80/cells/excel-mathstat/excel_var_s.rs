//! Excel VAR.S(number1, [number2], ...): sample variance (Bessel-corrected, divides by n-1) of a list of numbers -- the array-state envelope excel_npv established (`.cell` v11). A two-pass reduction over the u32[16] envelope: pass one sums the values as f32 (each element decoded via f32_from_bits from the host-written f32::to_bits pattern) to get the mean; pass two walks the array again accumulating (value - mean)^2, then divides by count-1 for the Bessel-corrected sample variance -- the same n-1 correction `sample_variance_from_sums` (statistics pack) already proves correct, but that cell only works from precomputed sums with raw-dataset aggregation left upstream; this cell does the aggregation itself over a raw array, the same new work excel_npv/excel_stdev_p already established the shape of. `count` names how many of the 16 slots are live; Excel's real VAR.S is uncapped up to 255 arguments, but this dialect's envelope is fixed at compile time (the array-state envelope wall, same limitation excel_npv/excel_stdev_p document). Distinct from STDEV.S (this cell's own square root) and from VAR.P (divides by count, not count-1 -- a population variance, undefined distinction below 2 samples that VAR.S alone carries).
//! tags: excel, var, vars, var-s, variance, sample-variance, bessel-correction, dispersion, statistics, array, f32, math-trig
//! kernel_bank: on
//! entry: ExcelVarS::run
//! limits: fixed 16-slot envelope, not caller-configurable (the array-state envelope wall); escalates (halt 0xFF06, out_of_domain) if count is less than 2 (sample variance is undefined below 2 points, Excel's own #DIV/0!) or exceeds 16; escalates (halt 0xFF08, float_domain) on a NaN result, (halt 0xFF07, float_overflow) on a non-finite one
struct ExcelVarS {
    values: [u32; 16],
    count: u16,
    var: f32,
}
impl ExcelVarS {
    fn run(&mut self) -> u16 {
        if self.count < 2u16 { halt(0xFF06u16); }
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
        let denom = n - 1.0f32;
        let r = ssd / denom;
        if r.is_nan() { halt(0xFF08u16); }
        let fin = r.is_finite();
        if !fin { halt(0xFF07u16); }
        self.var = r;
        1u16
    }
}
