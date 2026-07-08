fn run() -> u16 {
    let lee_time = 38;
    let lee_speed_factor = 1 / (1 - 0.2); // Lee was 1/0.8 faster than Gerald before diet change
    let gerald_new_time = lee_time * lee_speed_factor;
    gerald_new_time
}