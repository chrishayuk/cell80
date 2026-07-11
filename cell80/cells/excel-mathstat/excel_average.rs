//! Excel AVERAGE(number1,[number2],...): the arithmetic mean of a list of up to 16 numbers, sum(values)/count -- shares excel_npv's (excel-financial pack) array-state envelope convention (`.cell` v11): the arguments arrive in a u32[16] state field carrying f32 bit patterns (the dialect has no [f32; N] fields; the host writes f32::to_bits per element, the cell reinterprets each with f32_from_bits), with `count` naming how many of the 16 envelope slots are live. Real Excel AVERAGE accepts up to 255 arguments; this dialect's envelope is fixed at compile time -- 16 slots is the established precedent (the array-state envelope wall) and is documented, not hidden. Distinct from a plain SUM over the same envelope (returns the accumulated total itself, with no division) and from COUNT/COUNTA (returns the tally, not a mean) -- AVERAGE is the two composed: accumulate the sum across the live slots first, then divide by count exactly once at the end, rather than folding a running mean in per-element (which would need a different, non-associative update rule).
//! tags: excel, average, arithmetic-mean, mean-of-a-list, mean, list-of-numbers, sum-over-count, array, f32, math-trig, statistical
//! entry: ExcelAverage::run
//! limits: fixed 16-slot argument envelope, not caller-configurable (the array-state envelope wall, same as excel_npv); escalates (halt 0xFF06, out_of_domain) if count is 0 (Excel's own #DIV/0! for AVERAGE of zero arguments) or count exceeds 16; escalates (halt 0xFF08, float_domain) on a NaN running sum or final result, (halt 0xFF07, float_overflow) on a non-finite one
struct ExcelAverage {
    values: [u32; 16],
    count: u16,
    average: f32,
}
impl ExcelAverage {
    fn run(&mut self) -> u16 {
        if self.count == 0u16 {
            halt(0xFF06u16);
        }
        if self.count > 16u16 {
            halt(0xFF06u16);
        }

        let mut acc = 0.0f32;
        let mut i = 0u16;
        while i < self.count {
            let v = f32_from_bits(self.values[i as usize]);
            acc = acc + v;
            i = i + 1u16;
        }
        if acc.is_nan() {
            halt(0xFF08u16);
        }
        let acc_fin = acc.is_finite();
        if !acc_fin {
            halt(0xFF07u16);
        }

        let cnt = int_to_f32(self.count as u32);
        let result = acc / cnt;
        if result.is_nan() {
            halt(0xFF08u16);
        }
        let result_fin = result.is_finite();
        if !result_fin {
            halt(0xFF07u16);
        }

        self.average = result;
        1u16
    }
}
