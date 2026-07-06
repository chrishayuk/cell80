fn run() -> u16 {
    // Ratio of Sugar:Water is 7:13. Total parts = 7 + 13 = 20.
    // Total amount used is 120.
    // Value per part (k) = 120 / 20 = 6.
    // Sugar used = 7 * k = 7 * 6 = 42.

    let total_parts = 7 + 13; // 20
    let scaling_factor = 120 / total_parts; // 6
    let sugar_amount = 7 * scaling_factor; // 42
    sugar_amount
}