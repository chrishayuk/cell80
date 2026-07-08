fn run() -> u16 {
    let initial_containers = 2;
    let vehicles_per_container = 5;
    let total_vehicles_after_first_day = 30;

    // Calculate vehicles cleared on the first day
    let vehicles_cleared_first_day = initial_containers * vehicles_per_container;

    // Calculate containers imported on the second day
    (total_vehicles_after_first_day - vehicles_cleared_first_day) / vehicles_per_container
}