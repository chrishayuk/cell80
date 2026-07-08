fn run() -> u16 {
    let first_month = 60;
    let second_month = first_month * 3;
    let third_month = (second_month as f64 - third_month as f64 * 0.30) as u16;
    first_month + (second_month + third_month)
}