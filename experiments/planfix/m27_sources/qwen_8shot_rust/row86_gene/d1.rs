fn run() -> u16 {
    let current_age = 34;
    let age_when_started = 23;
    let vacations_per_year = 4;
    let years_of_vacations = current_age - age_when_started;
    let quilt_blocks = years_of_vacations * vacations_per_year;
    quilt_blocks
}