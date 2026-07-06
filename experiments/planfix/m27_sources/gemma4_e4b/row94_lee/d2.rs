fn run() -> u16 {
    let lee_time: u16 = 38;
    // Gerald's original time = Lee's time + 2
    let gerald_original_time: u16 = lee_time + 2; // 40

    // Speed increased by 10% (factor of 1.1 or 11/10).
    // New Time = Original Time / 1.1
    // To use integer arithmetic, multiply numerator and denominator by 10:
    // New Time = (Original Time * 10) / 11
    (gerald_original_time * 10) / 11
}