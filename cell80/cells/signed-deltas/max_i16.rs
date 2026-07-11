//! Maximum of two signed 16-bit values under true signed ordering (-1 > -32768) -- the direct complement of min_i16 and the signed sibling of max (u16) and max_u32, neither of which orders negative quantities correctly since a negative i16 bit-reinterpreted as unsigned looks like a large positive number.
//! tags: max, maximum, signed, i16, compare, ordering, larger, bigger, negative
fn run(a: i16, b: i16) -> i16 {
    if a > b {
        a
    } else {
        b
    }
}
