fn run() -> u16 {
    let first_month = 60;
    let second_month = first_month * 3;
    let third_month = (first_month * 3) - ((first_month * 3) / 10);
    first_month + second_month + third_month
}