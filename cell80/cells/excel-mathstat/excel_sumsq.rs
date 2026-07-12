//! Excel SUMSQ(number1, [number2], ...): sum of the squares of up to 16 values, acc = sum(values[i]^2) -- this pack's first SUM-family array reduction, built on the exact array-state envelope excel_npv.rs established (`.cell` v11): the arguments arrive in a u32[16] state field carrying f32 bit patterns (the dialect has no [f32; N] fields; the host writes each element via f32::to_bits, the cell reinterprets it with f32_from_bits), with `count` naming how many of the 16 envelope slots are actually live. Excel's real SUMSQ takes up to 255 arguments; this dialect's envelope is fixed at compile time and caps it at 16 (the array-state envelope wall), documented in limits below rather than hidden. Distinct from SUM (accumulates values[i] directly, never squared) and from AVERAGE (divides the accumulated total by count instead of returning the raw sum) -- SUMSQ never divides and never uses the raw signed value, only its square, so a mix of positive and negative inputs of the same magnitude contribute identically.
//! tags: excel, sumsq, sum-of-squares, squares, list-of-numbers, array, mathstat, f32
//! kernel_bank: on
//! entry: ExcelSumsq::run
//! limits: fixed 16-slot value envelope, not caller-configurable (the array-state envelope wall); escalates (halt 0xFF06, out_of_domain) if count is 0 or exceeds 16; escalates (halt 0xFF08, float_domain) on a NaN result, (halt 0xFF07, float_overflow) on a non-finite one
struct ExcelSumsq {
    values: [u32; 16],
    count: u16,
    sumsq: f32,
}
impl ExcelSumsq {
    fn run(&mut self) -> u16 {
        if self.count == 0u16 { halt(0xFF06u16); }
        if self.count > 16u16 { halt(0xFF06u16); }
        let mut acc = 0.0f32;
        let mut i = 0u16;
        while i < self.count {
            let v = f32_from_bits(self.values[i as usize]);
            acc = acc + v * v;
            i = i + 1u16;
        }
        if acc.is_nan() { halt(0xFF08u16); }
        let fin = acc.is_finite();
        if !fin { halt(0xFF07u16); }
        self.sumsq = acc;
        1u16
    }
}
