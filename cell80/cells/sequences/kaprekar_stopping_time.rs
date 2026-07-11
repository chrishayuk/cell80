//! Number of Kaprekar-routine iterations (sort digits descending minus ascending, repeat) needed for a zero-padded 4-digit n to reach the Kaprekar constant 6174, using a fixed 5-comparator unrolled sorting network on a 4-slot local array rather than digit_sort_asc/digit_sort_desc's variable-length bubble sort over up to 5 digits -- the "count of iterations, not the sequence" framing collatz_stopping_time established, applied to Kaprekar's descending-minus-ascending recurrence instead of 3n+1.
//! tags: number, kaprekar, sequence, digits, sort, sorting-network, iterate, steps, count, bounded, escalate
//! limits: escalates (halt 0xFF06, out_of_domain) if n > 9999, if all 4 zero-padded digits are identical (the routine then stalls at 0 forever), or if 6174 isn't reached within the proven 7-step bound
fn run(n: u16) -> u16 {
    if n > 9999u16 { halt(0xFF06u16); }
    let d0 = n / 1000u16;
    let d1 = (n / 100u16) % 10u16;
    let d2 = (n / 10u16) % 10u16;
    let d3 = n % 10u16;
    if (d0 == d1) && (d1 == d2) && (d2 == d3) { halt(0xFF06u16); }

    let mut v = n;
    let mut steps = 0u16;
    while v != 6174u16 {
        if steps >= 7u16 { halt(0xFF06u16); }

        let a0 = v / 1000u16;
        let a1 = (v / 100u16) % 10u16;
        let a2 = (v / 10u16) % 10u16;
        let a3 = v % 10u16;

        let mut arr: [u16; 4] = [0u16; 4];
        arr[0] = a0;
        arr[1] = a1;
        arr[2] = a2;
        arr[3] = a3;

        // Fixed 5-comparator unrolled sorting network (optimal for 4 elements),
        // sorting arr ascending in place: (0,1),(2,3),(0,2),(1,3),(1,2).
        if arr[0] > arr[1] { let t = arr[0]; arr[0] = arr[1]; arr[1] = t; }
        if arr[2] > arr[3] { let t = arr[2]; arr[2] = arr[3]; arr[3] = t; }
        if arr[0] > arr[2] { let t = arr[0]; arr[0] = arr[2]; arr[2] = t; }
        if arr[1] > arr[3] { let t = arr[1]; arr[1] = arr[3]; arr[3] = t; }
        if arr[1] > arr[2] { let t = arr[1]; arr[1] = arr[2]; arr[2] = t; }

        let asc = arr[0] * 1000u16 + arr[1] * 100u16 + arr[2] * 10u16 + arr[3];
        let desc = arr[3] * 1000u16 + arr[2] * 100u16 + arr[1] * 10u16 + arr[0];
        v = desc - asc;
        steps = steps + 1u16;
    }
    steps
}
