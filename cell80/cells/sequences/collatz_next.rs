//! Applies exactly one step of the Collatz (3n+1 / n/2) map to n and returns that single next value -- collatz_stopping_time/collatz_max_value only ever return a trajectory summary (step count or peak), never one raw step, so this exposes the bare transform for external step-by-step tracing.
//! tags: number, collatz, sequence, hailstone, step, iterate, next, single-step, wide, escalate
//! limits: escalates (halt 0xFF06, out_of_domain) if n == 0; escalates (halt 0xFF05, needs_wider_math) if n is odd and 3n+1 would overflow u16
fn run(n: u16) -> u16 {
    if n == 0u16 { halt(0xFF06u16); }
    if n % 2u16 == 0u16 {
        n / 2u16
    } else {
        let wide = (n as u32) * 3u32 + 1u32;
        if wide > 65535u32 { halt(0xFF05u16); }
        wide as u16
    }
}
