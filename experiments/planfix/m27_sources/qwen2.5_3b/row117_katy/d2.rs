fn run() -> u16 {
56
if 120 % 20 != 0 {
    let full_cycles = 120 / 20;
    let remainder = 120 % 20;
    if remainder >= 7 {
        full_cycles * 7 + 7
    } else {
        full_cycles * 7 + remainder
    }
} else {
    120 * 7 / 20
}
}