fn run() -> u16 {
    let initial_containers = 2;
    let vehicles_per_container = 5;
    let total_vehicles = 30;
    (total_vehicles / vehicles_per_container) - initial_containers
}