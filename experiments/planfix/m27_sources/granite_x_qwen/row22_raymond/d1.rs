fn run() -> u16 {
    let sams_age = 31;
    let raymond_age = sams_age - 6;
    let son_birth_year = raymond_age - 23;
    (son_birth_year.max(0)) as u16
}