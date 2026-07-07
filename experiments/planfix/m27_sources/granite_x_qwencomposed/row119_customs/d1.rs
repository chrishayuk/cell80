fn run() -> u16 {
    let initial_containers = 2;
    let vehicles_per_container = 5;
    let total_vehicles = 30;
    
    let vehicles_initially = initial_containers * vehicles_per_container;
    let remaining_vehicles = total_vehicles - vehicles_initially;
    let additional_containers = remaining_vehicles / vehicles_per_container;
    
    additional_containers
}