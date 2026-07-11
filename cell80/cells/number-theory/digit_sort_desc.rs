//! Reassemble n's decimal digits sorted descending, largest digit most significant (e.g. 4213 -> 4321); the complement of digit_sort_asc, same extraction/local-array/bubble-sort technique with the comparison flipped, so any n with two distinct digit values yields a different result from the ascending sibling.
//! tags: number, digits, sort, descending, digit-sort, decimal, math, bubble-sort
//! limits: escalates (halt 0xFF05, needs_wider_math) if the reassembled value would exceed 65535
fn run(n: u16) -> u16 {
    let mut v = n;
    let mut digits: [u16; 5] = [0u16; 5];
    let mut count = 0u16;
    while v != 0u16 {
        digits[count as usize] = v % 10u16;
        v = v / 10u16;
        count = count + 1u16;
    }
    let mut i = 0u16;
    while i < count {
        let mut j = 0u16;
        while j + 1u16 < count - i {
            if digits[j as usize] < digits[(j + 1u16) as usize] {
                let tmp = digits[j as usize];
                digits[j as usize] = digits[(j + 1u16) as usize];
                digits[(j + 1u16) as usize] = tmp;
            }
            j = j + 1u16;
        }
        i = i + 1u16;
    }
    let mut result = 0u32;
    let mut k = 0u16;
    while k < count {
        result = result * 10u32 + digits[k as usize] as u32;
        k = k + 1u16;
    }
    if result > 65535u32 { halt(0xFF05u16); }
    result as u16
}
