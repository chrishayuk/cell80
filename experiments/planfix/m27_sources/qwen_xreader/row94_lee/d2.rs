fn run() -> u16 {
    // T_L = 38 (Lee's time)
    // T_G = T_L + 2 = 40 (Gerald's original time)
    // S_G_old = 400 / T_G = 400 / 40 = 10 m/s
    // S_G_new = S_G_old * 1.1 = 10 * 11 / 10 = 11 m/s
    // T_G_new = 400 / S_G_new = 400 / 11

    let t_g_original = 38 + 2; // 40
    let s_g_old = 400 / t_g_original; // 10
    // Calculate new speed: S_G_new = S_G_old * 11 / 10 (to handle 1.1 increase using integers)
    let s_g_new = (s_g_old * 11) / 10; // 11

    // Calculate new time: T_G_new = 400 / S_G_new
    400 / s_g_new
}