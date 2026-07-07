//! Basis points of a wide value: value * bps / 10000 (e.g. 500 bps of 1000 is 50 — 5%). Escalates (needs_wider_math) on multiply overflow.
//! tags: money, bps, basis-points, percent, fraction, checked, wide, u32
//! entry: BpsOf::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if value * bps overflows u32
struct BpsOf { value: u32, bps: u32, result: u32 }
impl BpsOf {
    fn run(&mut self) -> u16 {
        let product = mul_checked_u32(self.value, self.bps);
        self.result = product / 10000u32;
        1u16
    }
}
