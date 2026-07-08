fn run() -> u16 {
    let downloads_first_month = 60;
    let downloads_second_month = 3 * downloads_first_month;
    let downloads_third_month = downloads_second_month - (30/100) * downloads_second_month;
    let total_downloads = downloads_first_month + downloads_second_month + downloads_third_month;
    let answer = total_downloads;
    answer
}