//! Recover the whole value from a known bps-portion of it: given part (the output of bps_of, i.e. value*bps/10000) and the bps rate, value = part * 10000 / bps. The inverse of bps_of.
//! tags: money, bps, basis-points, whole, reverse-percent, original, checked, wide, u32
//! entry: WholeFromBpsOf::run
//! limits: escalates (halt 0xFF06, out_of_domain) if bps == 0; escalates (halt 0xFF05, needs_wider_math) if part * 10000 overflows u32
struct WholeFromBpsOf { part: u32, bps: u32, value: u32 }
impl WholeFromBpsOf {
    fn run(&mut self) -> u16 {
        if self.bps == 0u32 { halt(0xFF06u16); }
        let product = self.part.wrapping_mul(10000u32);
        if self.part != 0u32 && product / 10000u32 != self.part { halt(0xFF05u16); }
        self.value = product / self.bps;
        1u16
    }
}
