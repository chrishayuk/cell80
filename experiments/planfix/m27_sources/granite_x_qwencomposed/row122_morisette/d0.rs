fn run() -> u16 {
    let morisette_apples = 5;
    let morisette_oranges = 8;
    let kaels_apples = morisette_apples * 2;
    let kaels_oranges = morisette_oranges / 2;
    let total_fruits = (morisette_apples + morisette_oranges) + (kaels_apples + kaels_oranges);
    total_fruits
}