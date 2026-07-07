//! The nth Bell number B_n (the number of ways to partition an n-element set): 1, 1, 2, 5, 15, 52, 203, 877, 4140, ... Computed via the Bell triangle, kept in one array updated in place (each new row's first entry is the previous row's last entry; each subsequent entry is the running sum plus the entry above it) -- checked, escalates instead of silently wrapping once an intermediate row sum would exceed u32::MAX.
//! tags: number, bell, partition, combinatorics, set, sequence, checked, wide, u32, escalate
//! entry: BellNumber::run
//! limits: n must be < 20 (the array bound); escalates (halt 0xFF05, needs_wider_math) if an intermediate Bell-triangle entry would exceed u32::MAX (n = 15 is the first to trigger this, even though B_15 itself would still fit)
struct BellNumber { n: u32, result: u32 }
impl BellNumber {
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
            let last_val = arr[(i - 1u32) as usize];
            let mut old_prev = arr[0];
            arr[0] = last_val;
            let mut j = 1u32;
            while j <= i {
                let old_at_j = arr[j as usize];
                let sum = add_checked_u32(arr[(j - 1u32) as usize], old_prev);
                arr[j as usize] = sum;
                old_prev = old_at_j;
                j = j + 1u32;
            }
            i = i + 1u32;
        }
        self.result = arr[0];
        1u16
    }
}
