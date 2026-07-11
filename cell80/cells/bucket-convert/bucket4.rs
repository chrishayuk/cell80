//! Bucket x into 0, 1, 2, or 3 by three ascending thresholds: x<t1 -> 0, x<t2 -> 1, x<t3 -> 2, else 3 -- the one-more-threshold arity sibling of bucket3.
//! tags: bucket, bin, classify, threshold, tier, quantize, four
//! entry: Bucket4::run
struct Bucket4 { x: u16, t1: u16, t2: u16, t3: u16, out: u16 }
impl Bucket4 {
    fn run(&mut self) -> u16 {
        let b = if self.x >= self.t3 { 3u16 } else if self.x >= self.t2 { 2u16 } else if self.x >= self.t1 { 1u16 } else { 0u16 };
        self.out = b;
        b
    }
}
