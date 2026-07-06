fn run() -> u16 {
    let r1: u16 = 20;
    let r4: u16 = 60;
    let r3: u16 = r4 + 10;
    // R2 = R1 * (5/4)
    let r2: u16 = (r1 * 5) / 4;

    r1 + r2 + r3 + r4
}