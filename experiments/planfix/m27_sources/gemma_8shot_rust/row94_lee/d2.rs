fn run() -> u16 {
    // Lee's time (T_L) = 38
    // Gerald's original time (T_G) = T_L + 2 = 40
    let t_g = 38 + 2;

    // Speed increases by 10%, so the new time is divided by 1.1 (or 11/10).
    // New Time = T_G / 1.1 = T_G * 10 / 11
    let numerator = t_g * 10; // 40 * 10 = 400
    let denominator = 11;

    numerator / denominator
}