//! Single equivalent percent discount of applying two successive percent discounts pct_a then pct_b: pct_a + pct_b - pct_a*pct_b/100 (floored at 0), computed in u32 internally -- distinct from discount_percent (which applies one discount to a value) by combining two discount rates into one before any value is touched.
//! tags: percent, discount, combine, successive, compound, chain, stacked, markdown, off
fn run(pct_a: u16, pct_b: u16) -> u16 {
    let a = pct_a as u32;
    let b = pct_b as u32;
    let sum = a + b;
    let reduction = a * b / 100u32;
    let r = if sum >= reduction { sum - reduction } else { 0u32 };
    r as u16
}
