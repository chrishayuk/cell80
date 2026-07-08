fn run() -> u16 {
    let v4 = 60;
    let v3 = v4 + 10;
    let v1 = 20;
    // V2 rings: one-quarter more than V1 (V1 * 5 / 4)
    let v2 = v1 + v1 * 5 / 4;
    v1 + v2 + v3 + v4
}