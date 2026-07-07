//! The nth square pyramidal number: 1^2 + 2^2 + ... + n^2 = n*(n+1)*(2n+1)/6, checked — escalates instead of silently wrapping once the running sum would exceed u32::MAX. Computed by iterative summation rather than the closed form, so cost scales with n (like is_prime_u32, budget a larger --cycles for n much beyond a few thousand).
//! tags: number, pyramidal, square, sum, squares, sequence, figurate, checked, wide, u32, escalate
//! entry: SquarePyramidal::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the running sum of squares would exceed u32::MAX
struct SquarePyramidal { n: u32, result: u32 }
impl SquarePyramidal {
    fn run(&mut self) -> u16 {
        let mut sum = 0u32;
        let mut i = 0u32;
        while i < self.n {
            i = i + 1u32;
            let sq = mul_checked_u32(i, i);
            sum = add_checked_u32(sum, sq);
        }
        self.result = sum;
        1u16
    }
}
