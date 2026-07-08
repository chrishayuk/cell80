fn run() -> u16 {
    let initial = 60;
    let second_month = initial * 3;
    let third_month = second_month - (second_month * 30 / 100);
    initial + second_month + third_month
}