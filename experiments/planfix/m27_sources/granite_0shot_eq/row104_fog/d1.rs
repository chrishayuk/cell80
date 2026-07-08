fn run() -> u16 {
    let minutes_per_3_miles = 10;
    let total_miles = 42;
    let num_segments = total_miles // 3;
    let answer = minutes_per_3_miles * num_segments;
    answer
}