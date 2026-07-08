fn run() -> u16 {
    let initial_speed = 10;
    let initial_hours = 3;
    let total_hours = 7;
    let remaining_hours = total_hours - initial_hours;
    initial_speed * initial_hours + (initial_speed / 2) * remaining_hours
}