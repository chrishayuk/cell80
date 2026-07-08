fn run() -> u16 {
    let name = 2 * 5;
    let name = is_gt(name, 0);
    let containers_day1 = name;
    let total_vehicles = 30;
    let total_containers = total_vehicles / 5;
    let second_day_containers = total_containers - containers_day1;
    let answer = second_day_containers;
    answer
}