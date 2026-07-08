fn run() -> u16 {
    let f1_rings = 20;
    // F2 rings: 20 + (20 / 4)
    let f2_rings = 20 + 20 / 4;
    // F3 rings: 60 + 10
    let f3_rings = 60 + 10;
    // F4 rings: 60
    let total = f1_rings + f2_rings + f3_rings + 60;
    total
}