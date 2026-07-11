//! Excel LARGE(array, k): the k-th largest value among up to 16 numbers (k=1 is the largest, k=2 the second-largest, and so on down to k=count, the smallest) -- built on the exact array-state envelope excel_npv.rs established (`.cell` v11): the numbers arrive in a u32[16] state field carrying f32 bit patterns (the dialect has no [f32; N] fields; the host writes each element via f32::to_bits, the cell reinterprets it with f32_from_bits), with `count` naming how many of the 16 envelope slots are actually live and `k` a separate live u16 scalar field (Excel's own second argument, 1-indexed, not a compile-time constant). Computed by straightforward selection over a local `used: [u16; 16]` flag array (never mutating the input envelope itself): k passes of "scan every not-yet-claimed slot, remember the largest seen and its index, then mark that index claimed" -- the value found on the k-th pass is the answer. This is O(k*n), not a real partial sort, but correct and cheap at n<=16 -- exactly the selection-with-removal approach the array-state harness was built to make trivial. Excel's real LARGE takes an uncapped range and any k up to that range's size; this dialect's envelope is fixed at compile time and caps it at 16 (the array-state envelope wall), documented in limits below rather than hidden. Distinct from MAX/max_u32 (hardwired to the single largest, no ranking and no k argument at all) and from the not-yet-built SMALL (LARGE's mirror image, k-th SMALLEST, the identical selection loop with the comparison flipped) -- unlike either, LARGE's k is itself a live input whose validity (1 <= k <= count) has to be checked before any scan begins, not baked into the shape of the loop.
//! tags: excel, large, kth-largest, k-th-largest, nth-largest, rank, ranking, order-statistic, selection, top-k, array, f32, mathstat, statistics
//! kernel_bank: on
//! entry: ExcelLarge::run
//! limits: fixed 16-slot value envelope, not caller-configurable (the array-state envelope wall); escalates (halt 0xFF06, out_of_domain) if count is 0 or exceeds 16, or if k is 0 or exceeds count (Excel's own #NUM! for an out-of-range k); escalates (halt 0xFF08, float_domain) on a NaN result, (halt 0xFF07, float_overflow) on a non-finite one
struct ExcelLarge {
    values: [u32; 16],
    count: u16,
    k: u16,
    result: f32,
}
impl ExcelLarge {
    fn run(&mut self) -> u16 {
        if self.count == 0u16 { halt(0xFF06u16); }
        if self.count > 16u16 { halt(0xFF06u16); }
        if self.k == 0u16 { halt(0xFF06u16); }
        if self.k > self.count { halt(0xFF06u16); }

        let mut used: [u16; 16] = [0u16; 16];
        let mut found = 0.0f32;
        let mut pass = 0u16;
        while pass < self.k {
            let mut max_val = 0.0f32;
            let mut max_idx = 0u16;
            let mut have_max = 0u16;
            let mut j = 0u16;
            while j < self.count {
                if used[j as usize] == 0u16 {
                    let v = f32_from_bits(self.values[j as usize]);
                    if have_max == 0u16 {
                        max_val = v;
                        max_idx = j;
                        have_max = 1u16;
                    } else if v > max_val {
                        max_val = v;
                        max_idx = j;
                    }
                }
                j = j + 1u16;
            }
            used[max_idx as usize] = 1u16;
            found = max_val;
            pass = pass + 1u16;
        }

        if found.is_nan() { halt(0xFF08u16); }
        let fin = found.is_finite();
        if !fin { halt(0xFF07u16); }
        self.result = found;
        1u16
    }
}
