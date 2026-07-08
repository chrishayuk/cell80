fn run() -> u16 {
    let initial_containers = 2;
    let initial_cars_per_container = 5;
    let total_cars_after_second_day = 30;

    // Calculate the number of cars inspected initially.
    let initial_total_cars = initial_containers * initial_cars_per_container;

    // The difference between total cars after second day and initial total gives new containers.
    let new_containers = (total_cars_after_second_day - initial_total_cars) / initial_cars_per_container;
    new_containers
}