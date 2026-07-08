fn run() -> u16 {
    let initial_speed = 10;
    let initial_time = 3;
    let remaining_time = 7 - initial_time;
    let final_speed = 5;
    let distance_initial = initial_speed * initial_time;
    let distance_final = final_speed * remaining_time;
    let answer = distance_initial + distance_final;
    answer
}