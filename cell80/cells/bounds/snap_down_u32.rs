//! Round a wide u32 value x DOWN to the nearest multiple of step (x if step == 0) — the wide sibling of snap_down. Floor to grid at u32 width.
//! tags: snap, round-down, floor, multiple, grid, quantize, wide, u32, large
//! entry: SnapDownWide::run
struct SnapDownWide { x: u32, step: u32, result: u32 }
impl SnapDownWide {
    fn run(&mut self) -> u16 {
        let v = if self.step != 0u32 { (self.x / self.step) * self.step } else { self.x };
        self.result = v;
        1u16
    }
}
