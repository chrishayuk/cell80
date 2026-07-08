fn run() -> u16 {
    let first_month = 60;
    let second_month = first_month * 3;
    let third_month = second_month * 9 / 10;
    let total_downloads = first_month + second_month + third_month;
    let answer = total_downloads;
    answer
}