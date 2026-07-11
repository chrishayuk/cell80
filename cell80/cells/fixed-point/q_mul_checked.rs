//! Checked Q8.8 fixed-point multiply: (a * b) >> 8, escalating instead of silently truncating when the scaled product exceeds u16's range -- q_mul's checked counterpart, since q_mul's own doc comment documents the unguarded wide shift with no escalation path at all.
//! tags: fixed-point, q8.8, multiply, checked, escalate, wide, u32, overflow
//! limits: escalates (halt 0xFF05, needs_wider_math) if (a*b)>>8 exceeds 65535 (u16::MAX)
fn run(a: u16, b: u16) -> u16 {
    let scaled = (a as u32 * b as u32) >> 8u32;
    if scaled > 65535u32 { halt(0xFF05u16); }
    scaled as u16
}
