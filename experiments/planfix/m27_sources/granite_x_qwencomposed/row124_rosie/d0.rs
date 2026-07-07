fn run() -> u16 {
    let distance_first_period = 10 * 3;
    let distance_second_period = (7 - 3) * 5;
    if distance_first_period + distance_second_period > 0 {
        distance_first_period + distance_second_period
    } else {
        0
    }
}