//! Isolates the value of the highest set bit of x via smear-then-subtract (OR x down into all lower bits, then subtract half the result); 0 when x == 0.
//! tags: bits, highest-bit, msb, isolate, mask, smear, bitmask
fn run(x: u16) -> u16 {
    let mut v = x;
    v = v | (v >> 1u16);
    v = v | (v >> 2u16);
    v = v | (v >> 4u16);
    v = v | (v >> 8u16);
    v - (v >> 1u16)
}
