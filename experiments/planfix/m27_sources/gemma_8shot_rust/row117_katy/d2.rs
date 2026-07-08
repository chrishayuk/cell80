fn run() -> u16 {
    let total_volume = 120;
    let sugar_ratio = 7;
    let water_ratio = 13;
    let total_parts = sugar_ratio + water_ratio;
    total_volume * sugar_ratio / total_parts
}