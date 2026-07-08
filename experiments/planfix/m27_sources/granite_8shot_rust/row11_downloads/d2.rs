fn run() -> u16 {
    let month1 = 60;
    let month2 = 3 * month1; // 180
    let month3 = (100 - 30) / 100 * month2; // 70% of 180 = 126
    month1 + month2 + month3
}