//! Excel FVSCHEDULE(principal, schedule): future value of a principal compounded through a SERIES of per-period rates -- fv = principal * (1+r_0) * (1+r_1) * ... -- for stepped or variable-rate deposits where FV's single constant rate can't express the schedule at all. The rate schedule arrives in a u32[16] state field carrying f32 bit patterns (the `.cell` v11 array-input envelope: host writes f32::to_bits per element, the cell reinterprets with f32_from_bits), `count` naming the live slots; count == 0 is valid and returns the principal unchanged (Excel's empty-schedule behaviour). Distinct from excel_effect (ONE nominal rate compounded npery times, bps-integer tier) and from compound_increase_by_bps (same single-rate shape in pure bps).
//! tags: excel, fvschedule, future-value, schedule, variable-rate, compound, stepped-rate, array, finance, f32
//! kernel_bank: on
//! entry: ExcelFvschedule::run
//! limits: fixed 16-slot rate-schedule envelope, not caller-configurable; escalates (halt 0xFF06, out_of_domain) if count exceeds 16; escalates (halt 0xFF08, float_domain) on a NaN result, (halt 0xFF07, float_overflow) on a non-finite one
struct ExcelFvschedule {
    principal: f32,
    schedule: [u32; 16],
    count: u16,
    fv: f32,
}
impl ExcelFvschedule {
    fn run(&mut self) -> u16 {
        if self.count > 16u16 { halt(0xFF06u16); }
        let mut acc = self.principal;
        let mut i = 0u16;
        while i < self.count {
            let r = f32_from_bits(self.schedule[i as usize]);
            acc = acc * (1.0f32 + r);
            i = i + 1u16;
        }
        if acc.is_nan() { halt(0xFF08u16); }
        let fin = acc.is_finite();
        if !fin { halt(0xFF07u16); }
        self.fv = acc;
        1u16
    }
}
