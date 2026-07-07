fn run() -> u16 {
    let first_month_downloads = 60;
    let second_month_downloads = first_month_downloads * 3;
    let third_month_downloads = second_month_downloads - (second_month_downloads * 30 / 100);
    first_month_downloads + second_month_downloads + third_month_downloads
}