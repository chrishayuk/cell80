//! Unit-compatibility check for adding/subtracting two typed quantities: returns their shared dimension code if a == b, else escalates — codes: 0=count,1=money,2=time,3=distance,4=area,5=volume,6=rate_money_per_count,7=rate_distance_per_time (docs/library-growth.md).
//! tags: unit, units, dimension, money, time, distance, add, subtract, compatible, checked
//! limits: escalates (halt 0xFF06, out_of_domain) if a and b differ, or either code is unrecognized (> 7)
fn run(a: u16, b: u16) -> u16 {
    if a > 7u16 || b > 7u16 { halt(0xFF06u16); }
    if a != b { halt(0xFF06u16); }
    a
}
