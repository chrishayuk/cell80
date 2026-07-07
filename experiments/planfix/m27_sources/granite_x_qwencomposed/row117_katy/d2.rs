fn run() -> u16 {
    let total = 120;
    let mut sugar = 0;

    for cups in 1..=total {
        if (7 * cups + 13 * cups) == total {
            sugar = 7 * cups;
        }
    }

    sugar
}