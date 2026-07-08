fn run() -> u16 {
    let james_age = 10;
    // Corey's age (C) is 1 year older than James: C = 10 + 1 = 11
    let corey_age = james_age + 1;
    // Amy's age (A) is 2 years younger than Corey: A = 11 - 2 = 9
    let amy_age = corey_age - 2;
    // Jackson's age (Jk) is 5 years younger than Amy: Jk = 9 - 5 = 4
    amy_age - 5
}