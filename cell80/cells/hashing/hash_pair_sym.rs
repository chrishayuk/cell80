//! Order-independent hash of two values (hash_pair_sym(a,b) == hash_pair_sym(b,a) always) by mixing the commutative sum and product of a and b through an avalanche chain, unlike hash_pair's order-sensitive left-to-right multiply-xor fold.
//! tags: hash, mix, pair, symmetric, commutative, order-independent, fingerprint, key, combine
fn run(a: u16, b: u16) -> u16 {
    let sum = a.wrapping_add(b);
    let prod: u32 = (a as u32).wrapping_mul(b as u32);
    let mut h: u32 = (sum as u32).wrapping_mul(0x9E3779B9u32);
    h = h ^ prod;
    h = (h ^ (h >> 13u32)).wrapping_mul(0x85EBCA6Bu32);
    h = (h ^ (h >> 13u32)).wrapping_mul(0xC2B2AE35u32);
    h = h ^ (h >> 16u32);
    let lo = h as u16;
    let hi = (h >> 16u32) as u16;
    lo ^ hi
}
