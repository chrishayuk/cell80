//! T(n,k): the count of ways to partition an n-element set into an ORDERED sequence of exactly k nonempty (unordered) blocks, via T(n,k) = k*(T(n-1,k-1) + T(n-1,k)), T(0,0)=1, T(n,0)=0 for n>0 -- the explicit-k slice fubini_number's own summary leaves unstated ('a(n) = sum_{k=1}^{n} C(n,k)*a(n-k)'), the same generalization shape rencontres_number already applied to derangement_count; sums to fubini_number(n) over all k and equals k! * stirling_second(n,k).
//! tags: number, ordered, partition, fubini, ordered-bell, stirling, second-kind, combinatorics, sequence, checked, wide, u32, escalate
//! entry: OrderedPartitionK::run
//! limits: returns 0 if k > n or (k == 0 and n > 0); n must be < 20 (the array bound, matching fubini_number's own cap); escalates (halt 0xFF05, needs_wider_math) if an intermediate product or sum would exceed u32::MAX
struct OrderedPartitionK { n: u32, k: u32, result: u32 }
impl OrderedPartitionK {
    fn run(&mut self) -> u16 {
        if self.n >= 20u32 { halt(0xFF06u16); }
        if self.k > self.n {
            self.result = 0u32;
            return 1u16;
        }
        if self.k == 0u32 {
            let base_case = if self.n == 0u32 { 1u32 } else { 0u32 };
            self.result = base_case;
            return 1u16;
        }
        // Row-by-row DP over a single fixed-size array indexed by block count j (0..k),
        // swept high-to-low within each row so arr[j-1] still holds the previous row's
        // value when arr[j] is computed -- the same in-place-array technique bell_number
        // and fubini_number use, adapted to sweep direction so one array covers two rows.
        let mut arr: [u32; 20] = [0u32; 20];
        arr[0] = 1u32;
        let mut i = 1u32;
        while i <= self.n {
            let mut j = self.k;
            while j >= 1u32 {
                let s = add_checked_u32(arr[(j - 1u32) as usize], arr[j as usize]);
                arr[j as usize] = mul_checked_u32(j, s);
                j = j - 1u32;
            }
            arr[0] = 0u32;
            i = i + 1u32;
        }
        self.result = arr[self.k as usize];
        1u16
    }
}
