fn run() -> u16 {
140
let seattle_sheep = 20;
let charleston_sheep = seattle_sheep * 4 / 1; // Four times fewer means Seattle's is 1/4 of Charleston's
let toulouse_sheep = charleston_sheep / 2;
let total_sheep = seattle_sheep + charleston_sheep + toulouse_sheep;
total_sheep
}