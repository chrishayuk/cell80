fn run() -> u16 {
    // Month 1 downloads
    let m1 = 60;
    
    // Month 2 downloads: 3 * M1
    let m2 = 3 * 60;
    
    // Month 3 downloads: M2 reduced by 30% (i.e., 70% of M2).
    // Calculation: (M2 * 7) / 10 to maintain integer arithmetic.
    let m3 = (m2 * 7) / 10;
    
    // Total downloads over three months
    m1 + m2 + m3
}