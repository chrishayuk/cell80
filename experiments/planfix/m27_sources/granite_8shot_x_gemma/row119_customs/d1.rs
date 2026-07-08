fn run() -> u16 {
    let initial_containers = 2;
    let vehicles_per_container = 5;
    let total_vehicles = 30;

    // Vehicles from initial import
    let initial_vehicles = initial_containers * vehicles_per_container;

    // Remaining vehicles to reach total of 30
    let remaining_vehicles = total_vehicles - initial_vehicles;

    // Calculate number of new containers (each with 5 vehicles)
    remaining_vehicles / vehicles_per_container
}