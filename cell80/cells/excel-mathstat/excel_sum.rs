//! Excel SUM(number1, [number2], ...): the sum of up to 16 numeric arguments -- the simplest cell in this wave's array-reduction batch, riding the same array-state envelope excel_npv.rs established (`.cell` v11): each argument arrives via a u32[16] state field carrying f32 bit patterns (the dialect has no [f32; N] fields; the host writes each element with f32::to_bits, the cell reinterprets it via f32_from_bits), with `count` naming how many of the 16 envelope slots are actually live. Excel's real SUM takes up to 255 arguments; this dialect's envelope is fixed at compile time, so arity caps at 16 (the array-state envelope wall, the same limitation excel_npv documents) -- one plain accumulation loop, no discount factor, no division by count, and no per-term weighting or comparison, which is what separates this from AVERAGE (this identical envelope, but the accumulated total divided by count) and from every other array-reduction sibling in this batch (MAX/MIN/PRODUCT/etc., which fold terms with a different operator than addition).
//! tags: excel, sum, total, addition, accumulate, array, aggregate, f32, mathstat
//! kernel_bank: on
//! entry: ExcelSum::run
//! limits: fixed 16-slot argument envelope, not caller-configurable (the array-state envelope wall); escalates (halt 0xFF06, out_of_domain) if count is 0 or exceeds 16; escalates (halt 0xFF08, float_domain) on a NaN result, (halt 0xFF07, float_overflow) on a non-finite one
struct ExcelSum {
    values: [u32; 16],
    count: u16,
    total: f32,
}
impl ExcelSum {
    fn run(&mut self) -> u16 {
        if self.count == 0u16 { halt(0xFF06u16); }
        if self.count > 16u16 { halt(0xFF06u16); }
        let mut acc = 0.0f32;
        let mut i = 0u16;
        while i < self.count {
            let v = f32_from_bits(self.values[i as usize]);
            acc = acc + v;
            i = i + 1u16;
        }
        if acc.is_nan() { halt(0xFF08u16); }
        let fin = acc.is_finite();
        if !fin { halt(0xFF07u16); }
        self.total = acc;
        1u16
    }
}
