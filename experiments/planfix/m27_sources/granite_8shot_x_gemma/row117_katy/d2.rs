fn run() -> u16 {
    let ratio_sugar = 7;
    let ratio_water = 13;
    let total = 120;
    let parts = ratio_sugar + ratio_water;
    (ratio_sugar * (total / parts))
}