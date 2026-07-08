fn run() -> u16 {
    // Morisette's apples: 5, oranges: 8
    let morisette_apples = 5;
    let morisette_oranges = 8;

    // Kael's apples (2 * 5)
    let kael_apples = 2 * morisette_apples;

    // Kael's oranges (8 / 2)
    let kael_oranges = morisette_oranges / 2;

    // Total fruits: (M apples + M oranges) + (K apples + K oranges)
    (morisette_apples + morisette_oranges) + (kael_apples + kael_oranges)
}