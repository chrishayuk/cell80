fn run() -> u16 {
23
let mut total_miles = 0;
if total_miles < 10 {
    total_miles += 10 * 3;
} else {
    total_miles += 5 * (7 - 3);
}
total_miles
}