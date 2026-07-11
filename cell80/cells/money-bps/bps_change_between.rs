//! Direction-agnostic basis-points change between two wide values (before and after, either order): bps_mag = |after - before| * 10000 / before, bps_neg = 1 if after < before else 0 — unifies bps_increase_between and bps_decrease_between (each halts if called against its required direction) into one sign-magnitude call for a caller who doesn't already know whether the value rose or fell.
//! tags: money, bps, basis-points, percent, change, rate, delta, direction, wide, u32, checked, sign-magnitude, escalate
//! entry: BpsChangeBetween::run
//! limits: escalates (halt 0xFF06, out_of_domain) if before == 0; escalates (halt 0xFF05, needs_wider_math) if the multiply overflows u32
struct BpsChangeBetween { before: u32, after: u32, bps_mag: u32, bps_neg: u16 }
impl BpsChangeBetween {
    fn run(&mut self) -> u16 {
        if self.before == 0u32 { halt(0xFF06u16); }
        let neg = (self.after < self.before) as u16;
        let diff = if self.after >= self.before { self.after - self.before } else { self.before - self.after };
        let scaled = mul_checked_u32(diff, 10000u32);
        self.bps_mag = scaled / self.before;
        self.bps_neg = neg;
        1u16
    }
}
