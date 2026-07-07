fn run() -> u16 {
    let lees_time = 38;
    // Lee used to be two seconds faster than Gerald.
    let gerald_old_time = lees_time + 2;
    // Gerald's diet improved his speed by 10%, so we calculate the new time.
    let gerald_new_time = gerald_old_time * 9 / 10;
    gerald_new_time
}