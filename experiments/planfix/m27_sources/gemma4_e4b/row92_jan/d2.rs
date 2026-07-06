fn run() -> u16 {
    // Cindy's pets (C) = 4
    // Marcia's pets (M) = C + 2 = 4 + 2 = 6
    let marcia_pets = 4 + 2;

    // Jan's pets (J) = 3 * M = 3 * 6 = 18
    let jan_pets = marcia_pets * 3;

    // Total pets (T) = C + M + J = 4 + 6 + 18 = 28
    4 + marcia_pets + jan_pets
}