//! Recovers the constant per-period bps rate from a value observed before and exactly 2 compounding periods later (before, after, after>=before): r = sqrt(after/before)-1, computed exactly in integer bps via a wide isqrt on (after*10000/before)*10000 -- distinct from bps_increase_between (recovers a rate over exactly 1 period) and compound_increase_by_bps (applies a KNOWN rate for N periods); recovering the rate from a pair spanning MORE than one period needs an Nth root, unbuildable before isqrt_u32 landed.
//! tags: money, bps, basis-points, compound, compounding, rate, periods, sqrt, isqrt, root, wide, u32, checked, escalate
//! entry: BpsRateOver2Periods::run
//! limits: escalates (halt 0xFF06, out_of_domain) if before == 0 or after < before; escalates (halt 0xFF05, needs_wider_math) if either scaling multiply overflows u32
struct BpsRateOver2Periods { before: u32, after: u32, bps: u32 }
impl BpsRateOver2Periods {
    fn run(&mut self) -> u16 {
        if self.before == 0u32 || self.after < self.before { halt(0xFF06u16); }
        let scaled_once = mul_checked_u32(self.after, 10000u32);
        let temp = scaled_once / self.before;
        let scaled = mul_checked_u32(temp, 10000u32);

        // Branch-free bitwise integer square root of scaled (the same loop q_sqrt/isqrt_u32 run).
        let mut val = scaled;
        let mut res = 0u32;
        let mut bit = 1u32 << 30u32;
        while bit > val { bit = bit >> 2u32; }
        while bit != 0u32 {
            if val >= res + bit {
                val = val - (res + bit);
                res = (res >> 1u32) + bit;
            } else {
                res = res >> 1u32;
            }
            bit = bit >> 2u32;
        }

        self.bps = res - 10000u32;
        1u16
    }
}
