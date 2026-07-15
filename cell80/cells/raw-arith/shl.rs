//! Raw shift left by a runtime amount: a << b (a shift of 16 or more saturates to 0, matching Rust u16 wrapping-shift semantics) -- the general-purpose two-argument sibling the bit-mask pack's set_bit/clear_bit/toggle_bit only use internally at a fixed bit position.
//! tags: bits, shift, left, shl, multiply, power-of-two, raw, wrapping
fn run(a: u16, b: u16) -> u16 { a << b }
