fn run() -> u16 {
    let start_age = 23;
    let current_age = 34;
    let vacations_per_year = 4;
    let age_range_years = (current_age - start_age) as u16;

    let quilt_blocks_total = age_range_years * vacations_per_year;
    quilt_blocks_total
}