fn run() -> u16 {
    let apples_morisette = 5;
    let oranges_morisette = 8;
    let apples_kael = 2 * apples_morisette;
    let oranges_kael = oranges_morisette / 2;
    let total_apples = apples_morisette + apples_kael;
    let total_oranges = oranges_morisette + oranges_kael;
    let total_fruits = total_apples + total_oranges;
    let answer = total_fruits;
    answer
}