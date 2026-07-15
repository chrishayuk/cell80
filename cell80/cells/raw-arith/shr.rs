//! Raw logical shift right by a runtime amount: a >> b, zero-filled (a shift of 16 or more saturates to 0) -- the general-purpose two-argument sibling the bit-mask pack's bit_is_set only uses internally at a fixed bit position.
//! tags: bits, shift, right, shr, divide, power-of-two, raw, logical
fn run(a: u16, b: u16) -> u16 { a >> b }
