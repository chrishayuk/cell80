fn run() -> u16 {
    let total_games = 22;
    let wins = losses + 8;
    let answer = wins + losses == total_games and (wins - losses) == 8 and wins == (total_games + 8) / 2;
    answer
}