fn run() -> u16 {
    let full_speed_hours = 3;
    let reduced_speed_hours = 7 - full_speed_hours;
    (full_speed_hours * 10 + reduced_speed_hours * 5) as u16
}