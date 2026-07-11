//! Excel MEDIAN(number1,[number2],...): the median (middle value) of a list of up to 16 numbers -- the middle element after sorting when count is odd, or the average of the two middle elements when count is even. Shares excel_npv/excel_average's array-state envelope convention (`.cell` v11): arguments arrive in a u32[16] state field carrying f32 bit patterns (the host writes f32::to_bits per element, the cell reinterprets each with f32_from_bits), with `count` naming how many of the 16 envelope slots are live -- Excel's real MEDIAN is uncapped up to 255 arguments, but this dialect's array-state envelope is fixed at compile time (16 slots, the established precedent, the array-state envelope wall). The live slots are copied into a local u32[16] scratch array first (leaving the `values` state field untouched, matching AVERAGE/STDEV.P's read-only-input convention), bubble-sorted in place there -- the same bounded swap-in-place technique digit_sort_asc/digit_sort_desc already establish for this dialect, decoding each comparison through f32_from_bits and swapping the raw bit-pattern u32 slots rather than sorting decoded floats directly (insertion sort would work equally well at n<=16, but this reuses an already-proven in-dialect swap pattern instead of introducing a new one) -- then the middle is picked by integer index arithmetic on `count` (mid = count/2; an odd count reads arr[mid] directly, an even count averages arr[mid-1] and arr[mid]). Distinct from AVERAGE (sum/count, a sum-based mean with no ordering step at all) and from the ranking-stats pack's own fixed-arity median3/median4 (exactly 3 or 4 raw u16 arguments, no array-state, a comparator network instead of a sort loop) and from the not-yet-built LARGE/SMALL (kth order statistic at an arbitrary caller-supplied k, not always the fixed middle).
//! tags: excel, median, middle-value, order-statistic, sort, bubble-sort, central-tendency, list-of-numbers, array, f32, math-trig, statistics
//! kernel_bank: on
//! entry: ExcelMedian::run
//! limits: fixed 16-slot argument envelope, not caller-configurable (the array-state envelope wall, same as excel_npv/excel_average); escalates (halt 0xFF06, out_of_domain) if count is 0 or exceeds 16 (a single value, count==1, is valid and returns that value, matching Excel); escalates (halt 0xFF08, float_domain) on a NaN median, (halt 0xFF07, float_overflow) on a non-finite one
struct ExcelMedian {
    values: [u32; 16],
    count: u16,
    median: f32,
}
impl ExcelMedian {
    fn run(&mut self) -> u16 {
        if self.count == 0u16 { halt(0xFF06u16); }
        if self.count > 16u16 { halt(0xFF06u16); }

        let mut arr: [u32; 16] = [0u32; 16];
        let mut i = 0u16;
        while i < self.count {
            arr[i as usize] = self.values[i as usize];
            i = i + 1u16;
        }

        // Bounded bubble sort over the live slots -- digit_sort_asc/digit_sort_desc's
        // established in-place swap technique, decoding each comparison through
        // f32_from_bits and swapping the raw bit-pattern u32 slots.
        let mut j = 0u16;
        while j < self.count - 1u16 {
            let mut k = 0u16;
            while k < self.count - 1u16 - j {
                let lo = f32_from_bits(arr[k as usize]);
                let hi = f32_from_bits(arr[(k + 1u16) as usize]);
                if lo > hi {
                    let tmp = arr[k as usize];
                    arr[k as usize] = arr[(k + 1u16) as usize];
                    arr[(k + 1u16) as usize] = tmp;
                }
                k = k + 1u16;
            }
            j = j + 1u16;
        }

        let mid = self.count / 2u16;
        let is_odd = self.count % 2u16;
        let median = if is_odd == 1u16 {
            f32_from_bits(arr[mid as usize])
        } else {
            let lo = f32_from_bits(arr[(mid - 1u16) as usize]);
            let hi = f32_from_bits(arr[mid as usize]);
            (lo + hi) / 2.0f32
        };

        if median.is_nan() { halt(0xFF08u16); }
        let fin = median.is_finite();
        if !fin { halt(0xFF07u16); }

        self.median = median;
        1u16
    }
}
