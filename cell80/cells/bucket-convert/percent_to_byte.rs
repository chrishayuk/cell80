//! Convert a 0..100 percent to a 0..255 byte scale: p*255/100.
//! tags: convert, percent, byte, scale, 255, conversion
fn run(p: u16) -> u16 { p * 255u16 / 100u16 }
