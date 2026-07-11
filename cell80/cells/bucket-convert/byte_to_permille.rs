//! Convert a 0..255 byte scale to a 0..1000 per-mille scale via the reduced fraction b*200/51 (b*1000/255 in lowest terms, avoiding a b*1000 overflow).
//! tags: convert, byte, permille, per-mille, 1000, scale, conversion
fn run(b: u16) -> u16 { b * 200u16 / 51u16 }
