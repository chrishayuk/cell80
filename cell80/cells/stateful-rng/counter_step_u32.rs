//! Wide sibling of counter_step for u32 fields: increments count by 1, wrapping to 0 the moment it would reach `limit` (limit 0 means never wrap, up to the native u32 boundary) — needed once a round-robin/dispatch index must range over a pool larger than counter_step's u16 ceiling of 65535. Returns a 1u16 success flag; caller reads the wrapped `count` back as a field, the same convention as every other wide state cell in the library.
//! tags: counter, round-robin, cycle, wrap, index, dispatch, state, wide, u32, pick, next, worker
//! entry: CounterStepU32::run
struct CounterStepU32 { count: u32, limit: u32 }
impl CounterStepU32 {
    fn run(&mut self) -> u16 {
        let n = self.count + 1u32;
        let wrapped = if self.limit != 0u32 && n >= self.limit { 0u32 } else { n };
        self.count = wrapped;
        1u16
    }
}
