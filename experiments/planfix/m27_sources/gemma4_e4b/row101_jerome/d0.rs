fn run() -> u16 {
    let f1 = 20;
    let f4 = 60;
    // F3 = F4 + 10
    let f3 = f4 + 10;
    // F2 = F1 * (5/4) = (F1 * 5) / 4. Multiply before dividing.
    let f2 = (f1 * 5) / 4;
    
    f1 + f2 + f3 + f4
}