fn run() -> u16 {
    // Friend 1 rings (R1) = 20
    let r1 = 20;

    // Friend 4 rings (R4) = 60
    let r4 = 60;

    // Friend 2 rings (R2): 1/4 times more than R1.
    // R2 = R1 * (1 + 1/4) = R1 * 5 / 4
    let r2 = r1 * 5 / 4; // 20 * 5 / 4 = 25

    // Friend 3 rings (R3): 10 times more than R4.
    // R3 = R4 * (1 + 10) = R4 * 11
    let r3 = r4 * 11; // 60 * 11 = 660

    // Total rings = R1 + R2 + R3 + R4
    r1 + r2 + r3 + r4
}