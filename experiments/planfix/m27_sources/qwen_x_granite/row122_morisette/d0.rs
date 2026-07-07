fn run() -> u16 {
19u16
if {
let morisette_apples = 5u16;
let morisette_oranges = 8u16;
let kael_apples = morisette_apples.checked_mul(2).ok_or(0u16)?;
let kael_oranges = morisette_oranges.checked_div(2).ok_or(0u16)?;
let total_fruits = morisette_apples + morisette_oranges + kael_apples + kael_oranges;
total_fruits
}
}