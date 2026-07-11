//! Convert a 0..1000 per-mille scale to a 0..255 byte scale via the reduced fraction pm*51/200.
//! tags: convert, permille, per-mille, byte, scale, 255, conversion
fn run(pm: u16) -> u16 { pm * 51u16 / 200u16 }
