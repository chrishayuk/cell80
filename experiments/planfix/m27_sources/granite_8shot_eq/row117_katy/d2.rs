fn run() -> u16 {
    let total_units = 120;
    let sugar_ratio = 7;
    let water_ratio = 13;
    let answer = total_units * sugar_ratio / (sugar_ratio + water_ratio);
    answer
}