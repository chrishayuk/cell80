//! The nth Fubini (ordered Bell) number: the count of ways to partition an n-element set into an ordered sequence of nonempty blocks (a(0)=1, a(n) = sum_{k=1}^{n} C(n,k)*a(n-k)) -- the ordered counterpart choose_u32/permute_u32 already established one shape over, since bell_number counts unordered partitions and has no ordered sibling. Computed bottom-up into a small fixed-size local array holding a(0..n) (the same in-place-array technique bell_number/stirling_first use), with each step's C(n,k) via the same multiplicative running-division formula choose_u32/stirling_second use. Checked, escalates instead of silently wrapping on overflow.
//! tags: number, fubini, ordered-bell, partition, ordered, combinatorics, sequence, checked, wide, u32, escalate
//! entry: FubiniNumber::run
//! limits: n must be < 20 (the array bound); escalates (halt 0xFF05, needs_wider_math) if an intermediate C(n,k) product or the running sum would exceed u32::MAX (this triggers at n = 12, since a(12) = 28,091,567,595 already exceeds u32::MAX even though a(11) still fits)
struct FubiniNumber { n: u32, result: u32 }
impl FubiniNumber {
    fn run(&mut self) -> u16 {
        if self.n >= 20u32 { halt(0xFF06u16); }
        let mut arr: [u32; 20] = [0u32; 20];
        arr[0] = 1u32;
        if self.n == 0u32 {
            self.result = 1u32;
            return 1u16;
        }
        let mut i = 1u32;
        while i <= self.n {
            let mut sum = 0u32;
            let mut k = 1u32;
            while k <= i {
                let mut kk = k;
                if i - k < k { kk = i - k; }
                let mut c = 1u32;
                let mut m = 1u32;
                while m <= kk {
                    let term = i - kk + m;
                    let num = mul_checked_u32(c, term);
                    c = num / m;
                    m = m + 1u32;
                }
                let contrib = mul_checked_u32(c, arr[(i - k) as usize]);
                sum = add_checked_u32(sum, contrib);
                k = k + 1u32;
            }
            arr[i as usize] = sum;
            i = i + 1u32;
        }
        self.result = arr[self.n as usize];
        1u16
    }
}
