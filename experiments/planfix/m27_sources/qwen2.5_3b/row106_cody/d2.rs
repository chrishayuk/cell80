fn run() -> u16 {
17
if 5 * 3 > u16::MAX {
    u16::MAX
} else {
    (5 * 3) + 5
}
}