fn run() -> u16 {
    // Lee's current time (T_L_current) = 38 seconds.
    // T_L_old = T_G_old - 2 => T_G_old = T_L_old + 2.
    // Assuming T_L_old = 38, then Gerald's old time (T_G_old) = 38 + 2 = 40 seconds.

    // Gerald improves speed by 10%. New time T_G_new = T_G_old / 1.1
    // T_G_new = T_G_old * (10/11).
    let t_g_old = 38 + 2; // 40

    // Calculate (T_G_old * 10) / 11 using integer arithmetic.
    (t_g_old * 10) / 11
}