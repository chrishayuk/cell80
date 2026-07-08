fn run() -> u16 {
    let fourth_visitor = 60;
    let third_visitor = fourth_visitor + fourth_visitor / 4;
    let second_visitor = fourth_visitor * 5 / 4;
    let first_visitor = fourth_visitor * 5;
    first_visitor + second_visitor + third_visitor + fourth_visitor
}