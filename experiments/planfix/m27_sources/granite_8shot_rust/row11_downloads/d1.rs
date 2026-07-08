fn run() -> u16 {
    let month1 = 60;
    let month2 = month1 * 3;
    let month3 = (month2 - month2 * 30 / 100) as u16;
    month1 + month2 + month3
}