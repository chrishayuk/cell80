fn run() -> u16 {
    // Day 1 vehicles: 2 containers * 5 vehicles/container = 10
    let day1_vehicles = 2 * 5;
    
    // Vehicles added on Day 2 (V2): Total vehicles - Day 1 vehicles
    let day2_vehicles = 30 - day1_vehicles;
    
    // Containers on Day 2 (C2): V2 / 5 vehicles/container
    day2_vehicles / 5
}