fn run() -> u16 {
    let james_age = 10;
    // Assuming Corey is older than James to resolve ambiguity: C = J + 1
    let corey_age = james_age + 1;
    // Amy is 2 years younger than Corey: A = C - 2
    let amy_age = corey_age - 2;
    // Jackson is 5 years younger than Amy: K = A - 5
    amy_age - 5
}