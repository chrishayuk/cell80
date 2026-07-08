fn run() -> u16 {
    let total_games = 22;
    let wins_losses_sum = total_games * 2 - total_games; // 44 - 22
    (wins_losses_sum + 8) / 2
}