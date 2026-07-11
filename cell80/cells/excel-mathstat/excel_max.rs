//! Excel MAX(number1, [number2], ...): the largest value among up to 16 supplied numbers, found by a single left-to-right pass that seeds a running maximum from the first live slot and raises it whenever a later value compares larger (v > m) -- shares excel_npv's (excel-financial pack) array-state envelope convention (`.cell` v11): the arguments arrive in a u32[16] state field carrying f32 bit patterns (the dialect has no [f32; N] fields; the host writes f32::to_bits per element, the cell reinterprets each with f32_from_bits), with `count` naming how many of the 16 envelope slots are live. Real Excel MAX accepts up to 255 arguments; this dialect's envelope is fixed at compile time -- 16 slots is the established precedent (the array-state envelope wall) and is documented, not hidden. Distinct from MIN (the identical single-pass shape with the comparison flipped, lowering a running minimum instead of raising a running maximum), from LARGE (the k-th largest of the list, driven by a caller-supplied rank -- MAX is LARGE's k=1 special case but takes no rank argument and cannot return any other position), and from AVERAGE/SUM (both fold every live value into one running total, never discarding any of them, whereas MAX's running-maximum update discards every value that isn't currently the largest).
//! tags: excel, max, maximum, largest, greatest, highest, extremum, array, list-of-numbers, reduction, statistical, f32
//! kernel_bank: on
//! entry: ExcelMax::run
//! limits: fixed 16-slot argument envelope, not caller-configurable (the array-state envelope wall, same as excel_npv/excel_min); escalates (halt 0xFF06, out_of_domain) if count is 0 or exceeds 16; escalates (halt 0xFF08, float_domain) if the running maximum is NaN (only reachable when the first live value, values[0], itself decodes to NaN -- a strict `>` comparison against NaN is always false, so a NaN encountered at any later position is silently skipped over rather than winning or propagating), (halt 0xFF07, float_overflow) if the final maximum is non-finite (infinite)
struct ExcelMax {
    values: [u32; 16],
    count: u16,
    max: f32,
}
impl ExcelMax {
    fn run(&mut self) -> u16 {
        if self.count == 0u16 {
            halt(0xFF06u16);
        }
        if self.count > 16u16 {
            halt(0xFF06u16);
        }

        let mut m = 0.0f32;
        let mut i = 0u16;
        while i < self.count {
            let v = f32_from_bits(self.values[i as usize]);
            if i == 0u16 {
                m = v;
            } else {
                if v > m {
                    m = v;
                }
            }
            i = i + 1u16;
        }
        if m.is_nan() {
            halt(0xFF08u16);
        }
        let fin = m.is_finite();
        if !fin {
            halt(0xFF07u16);
        }

        self.max = m;
        1u16
    }
}
