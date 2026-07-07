//! Verifies a claimed bps increase: returns 1 if after == before + before*bps/10000, else 0 — the verifier counterpart of increase_by_bps (money-bps's checked-arithmetic sibling had no reverse-equation check yet, unlike every other checked-arithmetic shape). Never escalates: a verifier always returns a verdict, computed in a wider internal width so a genuine overflow can't false-positive as a match.
//! tags: verify, verifier, equation, percent, bps, basis-points, money, tax, tip, markup, wide, u32, check, plan, reverse-equation
//! entry: PercentEqualsBps::run
struct PercentEqualsBps { before: u32, after: u32, bps: u32 }
impl PercentEqualsBps {
    fn run(&mut self) -> u16 {
        let product = self.before.wrapping_mul(self.bps);
        if self.before != 0u32 && product / self.before != self.bps {
            0u16
        } else {
            let delta = product / 10000u32;
            let expected = self.before.wrapping_add(delta);
            if expected < self.before { 0u16 } else { (expected == self.after) as u16 }
        }
    }
}
