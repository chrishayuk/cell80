fn run() -> u16 {
    let total_time = 7;
    let time_segment_1 = 3;
    let speed_segment_1 = 10;
    let speed_segment_2 = 5;

    let distance_1 = time_segment_1 * speed_segment_1;
    let remaining_time = total_time - time_segment_1;
    let distance_2 = remaining_time * speed_segment_2;

    distance_1 + distance_2
}