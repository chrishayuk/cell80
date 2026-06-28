//! Mean (average) of three values, computed without overflow.
//! tags: mean, average, avg, three, stat, central
fn run(a: u16, b: u16, c: u16) -> u16 { a / 3u16 + b / 3u16 + c / 3u16 + (a % 3u16 + b % 3u16 + c % 3u16) / 3u16 }
