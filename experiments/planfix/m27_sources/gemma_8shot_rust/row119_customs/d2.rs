fn run() -> u16 {
    let initial_containers = 2;
    let capacity = 5;
    let total_vehicles = 30;

    // Initial vehicles: 2 * 5 = 10
    let initial_vehicles = initial_containers * capacity;

    // New vehicles arrived: 30 - 10 = 20
    let new_vehicles = total_vehicles - initial_vehicles;

    // Containers on second day: 20 / 5 = 4
    new_vehicles / capacity
}