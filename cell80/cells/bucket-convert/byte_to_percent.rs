//! Convert a 0..255 byte scale to a 0..100 percent: b*100/255.
//! tags: convert, byte, percent, scale, 255, conversion
fn run(b: u16) -> u16 { b * 100u16 / 255u16 }
