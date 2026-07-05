//! Resulting unit-dimension code when multiplying two typed quantities (e.g. count*money=money, distance*distance=area) — codes: 0=count,1=money,2=time,3=distance,4=area,5=volume,6=rate_money_per_count,7=rate_distance_per_time,8=rate_money_per_time (docs/library-growth.md). Escalates on any unmodeled pair.
//! tags: unit, units, dimension, multiply, money, distance, area, volume, rate, wage, checked
//! limits: escalates (halt 0xFF06, out_of_domain) for an unmodeled or invalid unit pair
fn run(a: u16, b: u16) -> u16 {
    if a > 8u16 || b > 8u16 { halt(0xFF06u16); }
    if a == 0u16 && b == 0u16 { return 0u16; }
    if (a == 0u16 && b == 1u16) || (a == 1u16 && b == 0u16) { return 1u16; }
    if a == 3u16 && b == 3u16 { return 4u16; }
    if (a == 4u16 && b == 3u16) || (a == 3u16 && b == 4u16) { return 5u16; }
    if (a == 6u16 && b == 0u16) || (a == 0u16 && b == 6u16) { return 1u16; }
    if (a == 7u16 && b == 2u16) || (a == 2u16 && b == 7u16) { return 3u16; }
    if (a == 8u16 && b == 2u16) || (a == 2u16 && b == 8u16) { return 1u16; }
    halt(0xFF06u16);
    0u16
}
