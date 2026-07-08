fn run() -> u16 {
    let month_1 = 60;
    // Month 2: 3 * month_1
    let month_2 = 3 * month_1;
    // Month 3: month_2 reduced by 30% (i.e., 70% of month_2)
    let month_3 = month_2 * 7 / 10;
    month_1 + month_2 + month_3
}