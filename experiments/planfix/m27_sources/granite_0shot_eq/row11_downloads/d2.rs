fn run() -> u16 {
    let downloads_month1 = 60;
    let downloads_month2 = 3 * downloads_month1;
    let downloads_month3 = downloads_month2 - (30 / 100) * downloads_month2;
    let total_downloads = downloads_month1 + downloads_month2 + downloads_month3;
    let answer = total_downloads;
    answer
}