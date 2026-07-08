fn run() -> u16 {
    let initial_containers = 2;
    let vehicles_per_container = 5;
    let total_vehicles = 30;
    let added_vehicles = total_vehicles - (initial_containers * vehicles_per_container);
    added_vehicles / vehicles_per_container
}