fn run() -> u16 {
    let m1 = 60;
    let m2 = 3 * m1;
    // M3 is M2 reduced by 30%, so it retains 7/10 of M2
    let m3 = (m2 * 7) / 10;
    m1 + m2 + m3
}