//! Excel SMALL(array, k): the k-th smallest value among up to 16 numbers (k=1 is the smallest, k=2 the second-smallest, and so on up to k=count, the largest) -- the mirror image of LARGE (excel_large.rs), built on the exact array-state envelope excel_npv.rs established (`.cell` v11): the numbers arrive in a u32[16] state field carrying f32 bit patterns (the dialect has no [f32; N] fields; the host writes each element via f32::to_bits, the cell reinterprets it with f32_from_bits), with `count` naming how many of the 16 envelope slots are actually live and `k` a separate live u16 scalar field (Excel's own second argument, 1-indexed, not a compile-time constant). Computed by the identical selection-with-removal technique LARGE uses over a local `used: [u16; 16]` flag array (never mutating the input envelope itself): k passes of "scan every not-yet-claimed slot, remember the SMALLEST seen and its index, then mark that index claimed" -- the only change from LARGE's inner loop is the comparison direction (`v < min_val` here instead of `v > max_val`) -- the value found on the k-th pass is the answer. This is O(k*n), not a real partial sort, but correct and cheap at n<=16. Duplicate values are counted as distinct entries exactly as Excel does (SMALL({1,2,2,3},2) = 2 and SMALL({1,2,2,3},3) = 2, drawn from two separate slots on two separate passes). Excel's real SMALL takes an uncapped range and any k up to that range's size; this dialect's envelope is fixed at compile time and caps it at 16 (the array-state envelope wall), documented in limits below rather than hidden. Distinct from MIN/min_u32 (hardwired to the single smallest, no ranking and no k argument at all), from LARGE (this cell's own mirror, k-th LARGEST instead of k-th smallest), and from MEDIAN (a single fixed middle rank baked into the shape of the reduction, never a caller-supplied k).
//! tags: excel, small, kth-smallest, k-th-smallest, nth-smallest, rank, ranking, order-statistic, selection, bottom-k, array, f32, mathstat, statistics
//! kernel_bank: on
//! entry: ExcelSmall::run
//! limits: fixed 16-slot value envelope, not caller-configurable (the array-state envelope wall); escalates (halt 0xFF06, out_of_domain) if count is 0 or exceeds 16, or if k is 0 or exceeds count (Excel's own #NUM! for an out-of-range k); escalates (halt 0xFF08, float_domain) on a NaN result, (halt 0xFF07, float_overflow) on a non-finite one
struct ExcelSmall {
    values: [u32; 16],
    count: u16,
    k: u16,
    result: f32,
}
impl ExcelSmall {
    fn run(&mut self) -> u16 {
        if self.count == 0u16 { halt(0xFF06u16); }
        if self.count > 16u16 { halt(0xFF06u16); }
        if self.k == 0u16 { halt(0xFF06u16); }
        if self.k > self.count { halt(0xFF06u16); }

        let mut used: [u16; 16] = [0u16; 16];
        let mut found = 0.0f32;
        let mut pass = 0u16;
        while pass < self.k {
            let mut min_val = 0.0f32;
            let mut min_idx = 0u16;
            let mut have_min = 0u16;
            let mut j = 0u16;
            while j < self.count {
                if used[j as usize] == 0u16 {
                    let v = f32_from_bits(self.values[j as usize]);
                    if have_min == 0u16 {
                        min_val = v;
                        min_idx = j;
                        have_min = 1u16;
                    } else if v < min_val {
                        min_val = v;
                        min_idx = j;
                    }
                }
                j = j + 1u16;
            }
            used[min_idx as usize] = 1u16;
            found = min_val;
            pass = pass + 1u16;
        }

        if found.is_nan() { halt(0xFF08u16); }
        let fin = found.is_finite();
        if !fin { halt(0xFF07u16); }
        self.result = found;
        1u16
    }
}
