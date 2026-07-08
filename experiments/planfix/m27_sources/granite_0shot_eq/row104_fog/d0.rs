fn run() -> u16 {
    let minutes_per_3_miles = 10;
    let total_miles = 42;
    let number_of_3_mile_segments = total_miles / 3;
    let total_minutes = minutes_per_3_miles * number_of_3_mile_segments;
    let answer = total_minutes;
    answer
}