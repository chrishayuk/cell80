fn run() -> u16 {
    let miles_per_interval = 3;
    let total_miles = 42;
    let intervals = total_miles / miles_per_interval;
    let answer = intervals * 10;
    answer
}