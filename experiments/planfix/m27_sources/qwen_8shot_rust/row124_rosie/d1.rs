fn run() -> u16 {
    let full_speed_hours = 3;
    let reduced_speed_hours = 7 - full_speed_hours;
    let full_speed_miles = full_speed_hours * 10;
    let reduced_speed_miles = reduced_speed_hours * 5;
    full_speed_miles + reduced_speed_miles
}