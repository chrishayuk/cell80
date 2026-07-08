fn run() -> u16 {
    let total_games = 22;
    // Let W be wins, L be losses. W + L = 22 and W = L + 8.
    // (L + 8) + L = 22
    // 2L + 8 = 22
    // 2L = 14
    // L = 7
    // W = 7 + 8 = 15
    let losses_times_2 = total_games - 8; // 2L = 14
    (losses_times_2 / 2) + 8
}