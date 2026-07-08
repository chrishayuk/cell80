fn run() -> u16 {
    let lee_time = 38;
    let diff = 2;
    let g_original = lee_time + diff;
    // New time = Original Time / (1 + 0.1) = Original Time * 10 / 11
    g_original * 10 / 11
}