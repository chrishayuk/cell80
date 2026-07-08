fn run() -> u16 {
    let month1 = 60;
    let month2 = month1 * 3;
    // Month 3: decreased by 30% from month2, so it's 70% of month2.
    let month3 = month2 * 7 / 10;
    month1 + month2 + month3
}