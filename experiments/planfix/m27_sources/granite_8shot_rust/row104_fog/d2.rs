fn run() -> u16 {
    let speed = (3 / 10) * 60; // convert 3 miles in 10 minutes to miles per hour
    let total_distance = 42;
    (total_distance * 60) / speed as u16
}