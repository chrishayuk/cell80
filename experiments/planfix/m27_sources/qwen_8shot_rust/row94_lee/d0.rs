fn run() -> u16 {
    let lee_time = 38;
    let lee_speed_factor = 1 / (1 - 10 / 100);
    let lees_time_in_factor = 1 / lee_speed_factor;
    let gerald_time_in_factor = lee_time * lee_speed_factor;
    (lee_time * lees_time_in_factor) as u16
}