//! Eulerian number A(n, k): the count of permutations of n elements with exactly k descents (positions i where perm[i] > perm[i+1]). Computed via the standard recurrence A(n,k) = (k+1)*A(n-1,k) + (n-k)*A(n-1,k-1), kept in one array updated in place top-down by k (the same in-place carry technique stirling_first.rs uses, since this recurrence also needs both the just-written and the about-to-be-overwritten value at once).
//! tags: number, eulerian, descent, permutation, combinatorics, checked, wide, u32, escalate
//! entry: EulerianNumber::run
//! limits: returns 0 if k >= n (with A(0,0) = 1 as the base case); k must be < 24 (the array bound, halt 0xFF06 out_of_domain); escalates (halt 0xFF05, needs_wider_math) if an intermediate row entry would exceed u32::MAX
struct EulerianNumber { n: u32, k: u32, result: u32 }
impl EulerianNumber {
    fn run(&mut self) -> u16 {
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
                    term1 = mul_checked_u32(j + 1u32, dp[j as usize]);
                }
                let mut term2 = 0u32;
                if dp[(j - 1u32) as usize] != 0u32 {
                    term2 = mul_checked_u32(i - j, dp[(j - 1u32) as usize]);
                }
                dp[j as usize] = add_checked_u32(term1, term2);
                j = j - 1u32;
            }
            i = i + 1u32;
        }
        self.result = dp[self.k as usize];
        1u16
    }
}
