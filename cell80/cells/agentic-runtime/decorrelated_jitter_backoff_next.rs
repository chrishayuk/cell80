//! Decorrelated-jitter backoff: picks a fresh random value in [base, min(cap, max(current, base) * 3)] each call, so the range walks off the *previous randomized value* itself, unlike jittered_backoff_next/jittered_linear_backoff_next which both scale a fixed deterministic ceiling down to [0, ceiling].
//! tags: backoff, retry, jitter, random, decorrelated, multiplicative, rate-limit, agentic, state
//! entry: DecorrelatedJitterBackoff::run
struct DecorrelatedJitterBackoff { current: u16, base: u16, cap: u16, rand_bps: u16, next: u16 }
impl DecorrelatedJitterBackoff {
    fn run(&mut self) -> u16 {
        let temp = if self.current > self.base { self.current } else { self.base };
        let temp3 = temp as u32 * 3u32;
        let cap32 = self.cap as u32;
        let ceiling32 = if temp3 < cap32 { temp3 } else { cap32 };
        let ceiling16 = ceiling32 as u16;
        let ceiling = if ceiling16 < self.base { self.base } else { ceiling16 };
        let range = ceiling - self.base;
        let scaled = range as u32 * self.rand_bps as u32 / 10000u32;
        let scaled16 = if (scaled >> 16u32) as u16 != 0u16 { 65535u16 } else { scaled as u16 };
        let n = self.base.saturating_add(scaled16);
        self.next = n;
        self.next
    }
}
