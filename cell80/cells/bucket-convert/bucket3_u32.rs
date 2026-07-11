//! Bucket x into 0, 1, or 2 by two ascending wide u32 thresholds: x<t1 → 0, x<t2 → 1, else 2 — the wide sibling of bucket3 (which works over u16 and can't classify values beyond 65535, e.g. large counters or byte offsets).
//! tags: bucket, bin, classify, threshold, tier, quantize, wide, u32, large
//! entry: Bucket3Wide::run
struct Bucket3Wide { x: u32, t1: u32, t2: u32, out: u16 }
impl Bucket3Wide {
    fn run(&mut self) -> u16 {
        let b = if self.x >= self.t2 { 2u16 } else if self.x >= self.t1 { 1u16 } else { 0u16 };
        self.out = b;
        b
    }
}
