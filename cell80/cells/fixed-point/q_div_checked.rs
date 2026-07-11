//! Checked Q8.8 fixed-point divide: (a << 8) / b at wide u32 width like q_div, but escalates instead of silently truncating when the scaled quotient overflows u16 (returns 0 when b == 0, matching q_div's own zero-divisor convention).
//! tags: fixed-point, q8.8, divide, checked, wide, u32, escalate, overflow, safe
//! limits: escalates (halt 0xFF05, needs_wider_math) if the scaled quotient ((a<<8)/b) exceeds 0xFFFF; returns 0 if b == 0, matching q_div's own zero-divisor convention
fn run(a: u16, b: u16) -> u16 {
    if b != 0u16 {
        let scaled = ((a as u32) << 8u32) / b as u32;
        if scaled > 0xFFFFu32 {
            halt(0xFF05u16);
        }
        scaled as u16
    } else {
        0u16
    }
}
