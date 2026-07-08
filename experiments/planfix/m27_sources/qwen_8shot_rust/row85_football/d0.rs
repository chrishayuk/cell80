fn run() -> u16 {
    let total_games = 22;
    let wins_more_than_losses = 8;
    let losses = (total_games + wins_more_than_losses) / 2;
    total_games - losses
}