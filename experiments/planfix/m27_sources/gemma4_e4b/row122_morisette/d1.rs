fn run() -> u16 {
    let morisette_apples = 5;
    let morisette_oranges = 8;

    // Kael brought twice the apples (2 * 5)
    let kael_apples = 2 * morisette_apples;

    // Kael brought half the oranges (8 / 2)
    let kael_oranges = morisette_oranges / 2;

    // Total fruits = (M apples + K apples) + (M oranges + K oranges)
    (morisette_apples + kael_apples) + (morisette_oranges + kael_oranges)
}