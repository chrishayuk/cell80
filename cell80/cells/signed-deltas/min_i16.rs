//! Minimum of two signed 16-bit values under true signed ordering (-1 < 0) -- the signed sibling of min (u16) and min_u32, neither of which orders negative quantities correctly since a negative i16 bit-reinterpreted as unsigned looks like a large positive number.
//! tags: min, minimum, signed, i16, compare, ordering, smaller, negative
fn run(a: i16, b: i16) -> i16 {
    if a < b {
        a
    } else {
        b
    }
}
