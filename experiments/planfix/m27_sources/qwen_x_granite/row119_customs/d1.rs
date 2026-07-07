fn run() -> u16 {
    let initial_containers = 2;
    let vehicles_per_container_initial = 5;
    let total_vehicles_after_second_day = 30;

    // Calculate number of vehicles after first day
    let vehicles_after_first_day = initial_containers * vehicles_per_container_initial;

    // Number of containers brought in on the second day
    let containers_brought_in_second_day = (total_vehicles_after_second_day - vehicles_after_first_day) / vehicles_per_container_initial;

    containers_brought_in_second_day
}