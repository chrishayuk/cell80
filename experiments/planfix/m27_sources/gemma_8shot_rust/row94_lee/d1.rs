fn run() -> u16 {
    // Lee's time (L) = 38 seconds.
    // L = G_initial - 2  => G_initial = L + 2
    let initial_gerald_time = 38 + 2; // 40 seconds

    // Gerald improved speed by 10%, meaning his time decreases by 10%.
    // New Time = Initial Time * (1 - 0.1) = Initial Time * 9 / 10
    initial_gerald_time * 9 / 10
}