fn run() -> u16 {
    let weekly_usage = 2 * 30;
    if (weekly_usage * 4) / 7 < 240 {
        (weekly_usage * 4) / 7 as u16
    } else {
        240
    }
}