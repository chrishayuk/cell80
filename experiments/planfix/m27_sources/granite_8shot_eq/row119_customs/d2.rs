fn run() -> u16 {
    let initial_containers = 2;
    let vehicles_per_container = 5;
    let initial_vehicles = initial_containers * vehicles_per_container;
    let total_vehicles = 30;
    let additional_vehicles = total_vehicles - initial_vehicles;
    let additional_containers = additional_vehicles / vehicles_per_container;
    let answer = additional_containers;
    answer
}