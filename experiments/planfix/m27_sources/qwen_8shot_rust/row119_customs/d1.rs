fn run() -> u16 {
    let initial_containers = 2;
    let vehicles_per_container = 5;
    let total_vehicles_after_first_day = 30;

    // Calculate the number of vehicles cleared on the first day.
    let vehicles_cleared_first_day = initial_containers * vehicles_per_container;

    // The remaining vehicles must have come from containers brought in on the second day.
    let vehicles_from_second_day = total_vehicles_after_first_day - vehicles_cleared_first_day;

    // Calculate the number of containers brought in on the second day.
    let containers_brought_in_second_day = vehicles_from_second_day / vehicles_per_container

    containers_brought_in_second_day
}