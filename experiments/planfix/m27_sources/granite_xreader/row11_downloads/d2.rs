fn run() -> u16 {
    // Month 1: 60
    // Month 2: 3 * 60 = 180
    // Month 3: M2 reduced by 30% (70%). M2 * 7 / 10.
    // Total = M1 + M2 + M3
    (60 + (3 * 60) + (((3 * 60) * 7) / 10))
}