fn run() -> u16 {
    let lees_time = 38;
    let gerald_time = (lees_time * 2) as f64 / 1.10 + 0.01; // adjust for rounding to nearest integer
    round(gerald_time)
}