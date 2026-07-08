fn run() -> u16 {
    // Total sales: 88000
    // Ratio: 10 + 1 = 11 parts
    let total_sales = 88000;
    // Sales of the smaller artist (B): 88000 / 11 = 8000
    // Sales of Harald (A): Total - B = 88000 - 8000 = 80000
    total_sales - (total_sales / 11)
}