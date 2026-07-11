//! Returns 1 if a < b under true signed ordering, else 0 -- the signed sibling of is_lt/is_lt_u32, neither of which orders negative quantities correctly since a negative i16 bit-reinterpreted as unsigned looks like a large positive number (min_i16/max_i16 already flag this and prove native i16 comparison codegens correctly).
//! tags: predicate, compare, less, less-than, smaller, boolean, signed, i16, ordering, negative
fn run(a: i16, b: i16) -> u16 { (a < b) as u16 }
