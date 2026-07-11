//! Clears the lowest set bit of x via x & (x - 1u16), Kernighan's classic bit-clearing trick (the mask-value counterpart to lowest_set_bit's isolation, distinct from mask_clear/clear_bit which need an explicit bit index or mask).
//! tags: bits, clear, lowest-set-bit, kernighan, blsr, isolate
fn run(x: u16) -> u16 { x & (x - 1u16) }
