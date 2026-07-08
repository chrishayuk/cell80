fn run() -> u16 {
    let harry_sleep = 9;
    // James slept 2/3 of Harry's sleep: (2 * 9) / 3 = 6
    let james_sleep = 2 * harry_sleep / 3;
    harry_sleep - james_sleep
}