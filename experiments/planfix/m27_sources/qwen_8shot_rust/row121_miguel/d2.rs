fn run() -> u16 {
    let packs_per_week = 2;
    let sheets_per_pack = 30;
    let weeks_in_month = 4; // Assuming 4 weeks in a month
    packs_per_week * sheets_per_pack * weeks_in_month
}