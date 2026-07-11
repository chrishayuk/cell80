//! Verifies a claimed wide clamp: recomputes v = if x > hi { hi } else if x < lo { lo } else { x } (clamp_u32's own logic) and returns 1 if v == claimed, else 0 — the reverse-equation counterpart of clamp_u32 (never halts, always a verdict).
//! tags: verify, verifier, equation, clamp, bound, bounds, limit, restrict, constrain, range, wide, u32, check, plan, reverse-equation
//! entry: ClampEqualsWide::run
struct ClampEqualsWide { x: u32, lo: u32, hi: u32, claimed: u32 }
impl ClampEqualsWide {
    fn run(&mut self) -> u16 {
        let v = if self.x > self.hi { self.hi } else if self.x < self.lo { self.lo } else { self.x };
        (v == self.claimed) as u16
    }
}
