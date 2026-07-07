//! Unsigned Stirling number of the first kind c(n, k): the number of permutations of n elements with exactly k cycles. (The signed convention s(n,k) = (-1)^(n-k) * c(n,k) is not used here -- c(n,k) is always non-negative, avoiding a sign-magnitude return for a cell whose whole job is counting.) Computed via the standard recurrence c(n,k) = (n-1)*c(n-1,k) + c(n-1,k-1), kept in one array and updated in place row by row (the same in-place carry technique bell_number uses, since this recurrence also needs both the just-written and the about-to-be-overwritten value at once).
//! tags: number, stirling, first-kind, cycle, permutation, combinatorics, checked, wide, u32, escalate
//! entry: StirlingFirst::run
//! limits: returns 0 if k > n; k must be < 24 (the array bound); escalates (halt 0xFF05, needs_wider_math) if an intermediate row entry would exceed u32::MAX
struct StirlingFirst { n: u32, k: u32, result: u32 }
impl StirlingFirst {
    fn run(&mut self) -> u16 {
        if self.k > self.n {
            self.result = 0u32;
            return 1u16;
        }
        if self.k >= 24u32 { halt(0xFF06u16); }
        let mut dp: [u32; 24] = [0u32; 24];
        dp[0] = 1u32;
        let mut i = 1u32;
        while i <= self.n {
            let mut top = i;
            if top > 23u32 { top = 23u32; }
            let mut j = top;
            while j >= 1u32 {
                let mut term1 = 0u32;
                if dp[j as usize] != 0u32 {
                    term1 = mul_checked_u32(i - 1u32, dp[j as usize]);
                }
                dp[j as usize] = add_checked_u32(term1, dp[(j - 1u32) as usize]);
                j = j - 1u32;
            }
            dp[0] = 0u32;
            i = i + 1u32;
        }
        self.result = dp[self.k as usize];
        1u16
    }
}
