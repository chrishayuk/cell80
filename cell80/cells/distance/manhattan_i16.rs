//! Manhattan distance for signed (i16) grid coordinates: each difference goes through an excess-32768 shift into the shared iabs_diff kernel, dx + dy accumulated wide (u32 dist field) — origin-centered points welcome.
//! tags: grid, distance, spatial, score, navigation, signed, i16, wide, u32
//! entry: PtsSigned::run
struct PtsSigned { x1: i16, y1: i16, x2: i16, y2: i16, dist: u32 }
impl PtsSigned {
    fn run(&mut self) -> u16 {
        let sx1 = (self.x1 as u16).wrapping_add(32768u16);
        let sx2 = (self.x2 as u16).wrapping_add(32768u16);
        let sy1 = (self.y1 as u16).wrapping_add(32768u16);
        let sy2 = (self.y2 as u16).wrapping_add(32768u16);
        let dx = iabs_diff(sx1, sx2);
        let dy = iabs_diff(sy1, sy2);
        self.dist = dx as u32 + dy as u32;
        if (self.dist >> 16u32) as u16 != 0u16 { 65535u16 } else { self.dist as u16 }
    }
}
