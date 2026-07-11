//! Excel MIN(number1, [number2], ...): the smallest value among up to 16 supplied numbers, found by a single left-to-right pass that seeds a running minimum from the first live slot and lowers it whenever a later value compares smaller (v < m) -- shares excel_npv's (excel-financial pack) array-state envelope convention (`.cell` v11): the arguments arrive in a u32[16] state field carrying f32 bit patterns (the dialect has no [f32; N] fields; the host writes f32::to_bits per element, the cell reinterprets each with f32_from_bits), with `count` naming how many of the 16 envelope slots are live. Real Excel MIN accepts up to 255 arguments; this dialect's envelope is fixed at compile time -- 16 slots is the established precedent (the array-state envelope wall) and is documented, not hidden. Distinct from MAX (the identical single-pass shape with the comparison flipped, raising a running maximum instead of lowering a running minimum), from SMALL (the k-th smallest of the list, driven by a caller-supplied rank -- MIN is SMALL's k=1 special case but takes no rank argument and cannot return any other position), and from AVERAGE/SUM (both fold every live value into one running total, never discarding any of them, whereas MIN's running-minimum update discards every value that isn't currently the smallest).
//! tags: excel, min, minimum, smallest, least, lowest, extremum, array, list-of-numbers, reduction, statistical, f32
//! kernel_bank: on
//! entry: ExcelMin::run
//! limits: fixed 16-slot argument envelope, not caller-configurable (the array-state envelope wall, same as excel_npv/excel_average); escalates (halt 0xFF06, out_of_domain) if count is 0 or exceeds 16; escalates (halt 0xFF08, float_domain) on a NaN result, (halt 0xFF07, float_overflow) on a non-finite one
struct ExcelMin {
    values: [u32; 16],
    count: u16,
    min: f32,
}
impl ExcelMin {
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
                if v < m {
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

        self.min = m;
        1u16
    }
}
