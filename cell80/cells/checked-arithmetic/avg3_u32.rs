//! Floor average of three wide u32 values via per-term divide-by-3 plus remainder correction, overflow-free even when a+b+c itself would exceed u32::MAX — the arity-3 extension avg2_u32 lacks (composing avg2_u32 twice gives a differently-weighted, wrong result, unlike add/sub/mul which have honest arity-3 wide siblings).
//! tags: math, average, mean, three, avg3, wide, u32, overflow-free, divide, remainder
//! entry: Avg3Wide::run
struct Avg3Wide { a: u32, b: u32, c: u32, result: u32 }
impl Avg3Wide {
    fn run(&mut self) -> u16 {
        self.result = self.a / 3u32 + self.b / 3u32 + self.c / 3u32 + (self.a % 3u32 + self.b % 3u32 + self.c % 3u32) / 3u32;
        1u16
    }
}
