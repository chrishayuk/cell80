//! Round a wide u32 value x to the NEAREST multiple of step (ties up; x if step == 0) -- the wide sibling of round_to_multiple (which works over u16 and can't represent totals beyond 65535).
//! tags: round, nearest, multiple, snap, quantize, grid, wide, u32
//! entry: RoundToMultipleWide::run
//! limits: escalates (halt 0xFF05, needs_wider_math) if the ties-up add (x + step/2) or the final scale-back multiply would overflow u32
struct RoundToMultipleWide { x: u32, step: u32, result: u32 }
impl RoundToMultipleWide {
    fn run(&mut self) -> u16 {
        if self.step == 0u32 {
            self.result = self.x;
        } else {
            let half = self.step / 2u32;
            let sum = add_checked_u32(self.x, half);
            let q = sum / self.step;
            let scaled = mul_checked_u32(q, self.step);
            self.result = scaled;
        }
        1u16
    }
}
