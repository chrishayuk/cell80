//! Minimum of three signed 16-bit values under true signed ordering (-1 < 0) -- the three-operand sibling of min_i16, chained the same way min3 (u16) chains imin, using plain i16 comparison since ordering never needs to combine magnitudes the way sign-magnitude add/subtract does.
//! tags: min, minimum, smallest, signed, i16, compare, ordering, three, extremum
fn run(a: i16, b: i16, c: i16) -> i16 {
    let ab = if a < b { a } else { b };
    if ab < c {
        ab
    } else {
        c
    }
}
