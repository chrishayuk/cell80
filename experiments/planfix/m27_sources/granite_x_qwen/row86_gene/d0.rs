fn run() -> u16 {
    let years = 11; // 34 - 23 + 1 (inclusive range)
    let blocks_per_year = 4;
    let total_blocks = (years * blocks_per_year) as u16;
    total_blocks
}