fn run() -> u16 {
    let harry_hours = 9;
    let james_fraction = 2 / 3;
    let james_hours = harry_hours * james_fraction;
    let answer = abs_diff(harry_hours, james_hours);
    answer
}