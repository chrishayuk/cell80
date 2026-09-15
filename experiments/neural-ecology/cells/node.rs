// NE-0 v1.0; the host validates the input domain before cell execution.
// Cell80 ABI uses four two-byte slots (eight bytes); the experiment's canonical
// checkpoint encoding uses six bytes, with explicit packing at this boundary.
struct Node { v: u16, r: u8, s: u8, q: u16 }
impl Node {
    fn run(&mut self, current: i16) -> u16 {
        if self.r == 1u8 {
            self.v = 0u16;
            self.s = 0u8;
            self.r = 0u8;
        } else {
            let mut u: i16 = ((7u16 * self.v) / 8u16) as i16 + current;
            if u < 0i16 { u = 0i16; }
            if u > 255i16 { u = 255i16; }
            if u >= 64i16 {
                self.s = 1u8;
                self.r = 1u8;
                self.v = 0u16;
            } else {
                self.s = 0u8;
                self.r = 0u8;
                self.v = u as u16;
            }
        }
        let mut q = (15u16 * self.q) / 16u16 + 64u16 * (self.s as u16);
        if q > 255u16 { q = 255u16; }
        self.q = q;
        self.s as u16
    }
}
