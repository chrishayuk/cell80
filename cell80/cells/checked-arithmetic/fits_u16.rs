//! Returns 1 if a wide u32 value fits in u16 (<= 65535) without narrowing loss, else 0.
//! tags: math, checked, wide, u32, guard, fits, range
//! entry: FitsU16::run
struct FitsU16 { x: u32, ok: u16 }
impl FitsU16 {
    fn run(&mut self) -> u16 {
        self.ok = (self.x <= 65535u32) as u16;
        self.ok
    }
}
