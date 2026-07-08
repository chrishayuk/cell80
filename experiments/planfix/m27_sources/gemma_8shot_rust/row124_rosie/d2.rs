fn run() -> u16 {
    let total_time = 7;
    let time_1 = 3;
    let rate_1 = 10;
    let remaining_time = total_time - time_1;
    let rate_2 = 5;
    (rate_1 * time_1) + (rate_2 * remaining_time)
}