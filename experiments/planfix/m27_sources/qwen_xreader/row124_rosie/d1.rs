fn run() -> u16 {
    let total_hours = 7;
    let first_part_hours = 3;
    let second_part_hours = total_hours - first_part_hours;

    let distance_first_part = (10 * first_part_hours) as u16;
    let distance_second_part = (5 * second_part_hours) as u16;

    distance_first_part + distance_second_part
}