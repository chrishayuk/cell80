//! The integer partition function p(n): the number of ways to write n as a sum of positive integers, order not counting (3 = 3 = 2+1 = 1+1+1, so p(3) = 3) -- distinct from every existing 'partition' cell in this pack (bell_number, fubini_number, stirling_second), which all count SET partitions, not integer partitions. Computed via the classic subset-sum DP into a small fixed-size local array: for each part size k from 1 to n, add dp[i-k] into dp[i] for every i from k to n (each part size is folded in exactly once, so a part may be reused any number of times within a sum -- the standard "coin change count" recurrence specialized to coins 1..n). Cost is O(n^2) with a checked-arithmetic call per inner step (the square_pyramidal_number shape), so n past a few dozen needs a larger --cycles budget than the interpreter default.
//! tags: number, partition, integer-partition, combinatorics, dp, subset-sum, checked, wide, u32, escalate
//! entry: PartitionNumber::run
//! limits: n must be < 150 (the array bound), escalating (halt 0xFF06, out_of_domain) if exceeded; escalates (halt 0xFF05, needs_wider_math) if an intermediate dp entry would exceed u32::MAX (n = 128 is the first to trigger this, since p(128) = 4,351,078,600 already exceeds u32::MAX even though p(127) still fits)
struct PartitionNumber { n: u32, result: u32 }
impl PartitionNumber {
    fn run(&mut self) -> u16 {
        if self.n >= 150u32 { halt(0xFF06u16); }
        let mut dp: [u32; 150] = [0u32; 150];
        dp[0] = 1u32;
        let mut k = 1u32;
        while k <= self.n {
            let mut i = k;
            while i <= self.n {
                let sum = add_checked_u32(dp[i as usize], dp[(i - k) as usize]);
                dp[i as usize] = sum;
                i = i + 1u32;
            }
            k = k + 1u32;
        }
        self.result = dp[self.n as usize];
        1u16
    }
}
