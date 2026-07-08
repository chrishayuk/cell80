fn run() -> u16 {
    let morisette_apples = 5;
    let morisette_oranges = 8;
    let kael_apples = morisette_apples * 2;
    let kael_oranges = morisette_oranges / 2;
    let total_fruits = (morisette_apples + kael_apples) + (morisette_oranges + kael_oranges);
    let answer = total_fruits;
    answer
}