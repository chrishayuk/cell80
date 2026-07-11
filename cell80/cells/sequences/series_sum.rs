//! Sum of an arithmetic series given its two endpoints and term count instead of (a, d): count*(first + last)/2, multiplying before dividing so odd first+last stays exact — composing via avg2 then multiplying is unsound because avg2 floors the endpoint average before the count ever multiplies it.
//! tags: number, arithmetic, series, sequence, sum, endpoints, first, last, math, checked, wide, u32, escalate
//! entry: SeriesSum::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the endpoint sum or its product with count overflows u32; escalates (halt 0xFF06, out_of_domain) if count*(first+last) is not evenly divisible by 2
struct SeriesSum { first: u32, last: u32, count: u32, result: u32 }
impl SeriesSum {
    fn run(&mut self) -> u16 {
        if self.count == 0u32 {
            self.result = 0u32;
            return 1u16;
        }
        let endpoint_sum = add_checked_u32(self.first, self.last);
        let prod = mul_checked_u32(self.count, endpoint_sum);
        if prod % 2u32 != 0u32 { halt(0xFF06u16); }
        self.result = prod / 2u32;
        1u16
    }
}
