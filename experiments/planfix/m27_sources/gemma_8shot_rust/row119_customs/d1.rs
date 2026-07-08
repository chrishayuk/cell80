fn run() -> u16 {
    let initial_containers = 2;
    let vehicles_per_container = 5;
    let total_vehicles = 30;

    // Calculate initial vehicles: 2 * 5 = 10
    let initial_vehicles = initial_containers * vehicles_per_container;

    // Calculate added vehicles: 30 - 10 = 20
    let added_vehicles = total_vehicles - initial_vehicles;

    // Calculate containers added on Day 2: 20 / 5 = 4
    added_vehicles / vehicles_per_container
}