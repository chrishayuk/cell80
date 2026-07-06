fn run() -> u16 {
    let m1: u16 = 60;
    let m2: u16 = 3 * m1;
    // Calculate M3 (70% of M2). We multiply by 7 and divide by 10 to maintain integer arithmetic.
    let m3: u16 = (m2 * 7) / 10;
    m1 + m2 + m3
}