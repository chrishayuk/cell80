//! Reassemble n's decimal digits sorted ascending (smallest digit most significant): e.g. 4213 -> 1234, 120 -> 12 (a sorted-in leading zero just contributes a value-0 place during reconstruction, so it drops naturally). Distinct from digit_reverse (positional reverse, not a sort) and is_repdigit (checks whether the digits are already uniform, doesn't transform them). Digits are extracted into a small fixed-size local array (max 5 slots, since 65535 is u16's longest decimal form), bubble-sorted in place, then reassembled -- the same local-array-within-a-single-call technique bell_number/stirling_first/fubini_number use, just as a plain local rather than a state-cell field, since a free function has no fields to hold it in.
//! tags: number, digit, sort, ascending, rearrange, decimal, digits, permutation, array, math
fn run(n: u16) -> u16 {
    let mut arr: [u16; 5] = [0u16; 5];
    let mut count = 0u16;
    if n == 0u16 {
        arr[0] = 0u16;
        count = 1u16;
    } else {
        let mut v = n;
        while v != 0u16 {
            arr[count as usize] = v % 10u16;
            v = v / 10u16;
            count = count + 1u16;
        }
    }
    let mut i = 0u16;
    while i < count {
        let mut j = 0u16;
        while j < count - 1u16 - i {
            if arr[j as usize] > arr[(j + 1u16) as usize] {
                let tmp = arr[j as usize];
                arr[j as usize] = arr[(j + 1u16) as usize];
                arr[(j + 1u16) as usize] = tmp;
            }
            j = j + 1u16;
        }
        i = i + 1u16;
    }
    let mut result = 0u16;
    let mut k = 0u16;
    while k < count {
        result = result * 10u16 + arr[k as usize];
        k = k + 1u16;
    }
    result
}
