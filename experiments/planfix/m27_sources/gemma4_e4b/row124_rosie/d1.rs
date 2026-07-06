fn run() -> u16 {
    // Distance covered in the first phase: 10 mph * 3 hours = 30 miles
    let d1 = 10 * 3;

    // Time remaining for the second phase: 7 total hours - 3 hours used = 4 hours
    let t2 = 7 - 3;

    // Distance covered in the second phase: 5 mph * 4 hours = 20 miles
    let d2 = 5 * t2;

    // Total distance = d1 + d2
    d1 + d2
}