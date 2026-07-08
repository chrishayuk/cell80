fn run() -> u16 {
    let containers_initial = 2;
    let vehicles_per_container = 5;
    let total_vehicles_initial = containers_initial * vehicles_per_container;
    let additional_vehicles = total_vehicles - total_vehicles_initial;
    let additional_containers = additional_vehicles / vehicles_per_container;
    let answer = additional_containers;
    answer
}