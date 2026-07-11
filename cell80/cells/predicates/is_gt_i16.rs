//! Returns 1 if a > b under true signed ordering (-1 > -32768), else 0 -- the direct complement of is_lt_i16 and the signed sibling of is_gt (u16) and is_gt_u32, neither of which orders negative quantities correctly since a negative i16 bit-reinterpreted as unsigned looks like a large positive number.
//! tags: predicate, compare, greater, greater-than, larger, boolean, signed, i16, ordering, negative
fn run(a: i16, b: i16) -> u16 {
    (a > b) as u16
}
