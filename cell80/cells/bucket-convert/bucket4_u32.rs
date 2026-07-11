//! Bucket x into 0, 1, 2, or 3 by three ascending wide u32 thresholds: x<t1 -> 0, x<t2 -> 1, x<t3 -> 2, else 3 -- the wide sibling of bucket4 (which works over u16 and can't classify values beyond 65535).
//! tags: bucket, bin, classify, threshold, tier, quantize, four, wide, u32, large
//! entry: Bucket4Wide::run
struct Bucket4Wide { x: u32, t1: u32, t2: u32, t3: u32, out: u16 }
impl Bucket4Wide {
    fn run(&mut self) -> u16 {
        let b = if self.x >= self.t3 { 3u16 } else if self.x >= self.t2 { 2u16 } else if self.x >= self.t1 { 1u16 } else { 0u16 };
        self.out = b;
        b
    }
}
