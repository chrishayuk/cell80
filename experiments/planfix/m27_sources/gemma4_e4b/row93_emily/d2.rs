fn run() -> u16 {
    // James is 10. Corey is 1 year older than James (Corey = 10 + 1).
    let corey_age: u16 = 10 + 1;

    // Amy is 2 years younger than Corey (Amy = Corey - 2).
    let amy_age: u16 = corey_age - 2;

    // Jackson's age is 5 years younger than Amy (Jackson = Amy - 5).
    let jackson_age: u16 = amy_age - 5;

    jackson_age
}