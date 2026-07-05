//! Resulting unit-dimension code when dividing a numerator quantity by a denominator quantity (e.g. money/count=rate_money_per_count, money/time=rate_money_per_time, same/same=count) — same codes as unit_mul (docs/library-growth.md). Escalates on any unmodeled pair.
//! tags: unit, units, dimension, divide, rate, money, distance, area, volume, time, wage, checked
//! limits: escalates (halt 0xFF06, out_of_domain) for an unmodeled or invalid unit pair
fn run(a: u16, b: u16) -> u16 {
    if a > 8u16 || b > 8u16 { halt(0xFF06u16); }
    if a == b { return 0u16; }
    if a == 1u16 && b == 0u16 { return 6u16; }
    if a == 3u16 && b == 2u16 { return 7u16; }
    if a == 4u16 && b == 3u16 { return 3u16; }
    if a == 5u16 && b == 3u16 { return 4u16; }
    if a == 5u16 && b == 4u16 { return 3u16; }
    if a == 1u16 && b == 2u16 { return 8u16; }
    halt(0xFF06u16);
    0u16
}
