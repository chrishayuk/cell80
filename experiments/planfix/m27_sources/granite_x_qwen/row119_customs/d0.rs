fn run() -> u16 {
    let initial_containers = 2 * 5;
    let current_total = 30;
    let vehicles_per_container = 5;
    
    if current_total > initial_containers {
        (current_total - initial_containers) / vehicles_per_container
    } else {
        0
    }
}