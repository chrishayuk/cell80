//! Q(n): the number of partitions of n into distinct parts (no part repeated, e.g. 6 = 6 = 5+1 = 4+2 = 3+2+1, so Q(6) = 4) -- the same subset-sum DP array partition_number uses, but a 0/1 knapsack pass instead of unbounded coin-change: for each part size k the inner loop runs i from n down to k (not up to n), so dp[i-k] is always last round's value and a part already folded into a lower i can never be folded in twice, unlike partition_number where the ascending inner loop lets each part size be reused any number of times within a sum.
//! tags: number, partition, integer-partition, distinct, combinatorics, dp, knapsack, subset-sum, checked, wide, u32, escalate
//! entry: DistinctPartitions::run
//! limits: n must be < 256 (the array bound), escalating (halt 0xFF06, out_of_domain) if exceeded; escalates (halt 0xFF05, needs_wider_math) if an intermediate dp entry would exceed u32::MAX (n = 238 is the first to trigger this, since Q(238) = 4,402,567,324 already exceeds u32::MAX even though Q(237) = 4,163,989,458 still fits)
struct DistinctPartitions { n: u32, result: u32 }
impl DistinctPartitions {
    fn run(&mut self) -> u16 {
        if self.n >= 256u32 { halt(0xFF06u16); }
        let mut dp: [u32; 256] = [0u32; 256];
        dp[0] = 1u32;
        let mut k = 1u32;
        while k <= self.n {
            let mut i = self.n;
            while i >= k {
                let sum = add_checked_u32(dp[i as usize], dp[(i - k) as usize]);
                dp[i as usize] = sum;
                i = i - 1u32;
            }
            k = k + 1u32;
        }
        self.result = dp[self.n as usize];
        1u16
    }
}
