//! Returns 1 if x (u32) is a power of two, else 0 -- the wide sibling of is_pow2 (which works over u16, up to 65535), via the same x != 0 && (x & (x-1)) == 0 bit trick at u32 width.
//! tags: number, power, predicate, pow2, bits, single-bit, wide, u32, large
//! entry: IsPow2Wide::run
struct IsPow2Wide { x: u32, result: u16 }
impl IsPow2Wide {
    fn run(&mut self) -> u16 {
        let v = (self.x != 0u32 && (self.x & (self.x - 1u32)) == 0u32) as u16;
        self.result = v;
        v
    }
}
