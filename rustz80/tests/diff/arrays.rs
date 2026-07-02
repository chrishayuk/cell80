//! Local arrays: literals, `[v; N]` fill, byte arrays, indexing.

use crate::harness::*;

#[test]
fn arrays() {
    // literal-indexed read/write
    check!({
        let mut a = [0u16; 4];
        a[0] = 10u16;
        a[1] = 20u16;
        a[2] = 30u16;
        a[3] = 40u16;
        a[1] + a[3]
    }); // 60
        // array literal + variable index (needs `as usize` — valid host Rust)
    check!({
        let a = [3u16, 1u16, 4u16, 1u16, 5u16];
        let mut sum = 0u16;
        let mut i = 0u16;
        while i < 5u16 {
            sum = sum + a[i as usize];
            i = i + 1u16;
        }
        sum
    }); // 14
        // fill via loop, read back
    check!({
        let mut sq = [0u16; 8];
        let mut i = 0u16;
        while i < 8u16 {
            sq[i as usize] = i * i;
            i = i + 1u16;
        }
        sq[7]
    }); // 49
        // in-place reverse, then read both ends
    check!({
        let mut a = [1u16, 2u16, 3u16, 4u16, 5u16];
        let mut i = 0u16;
        while i < 2u16 {
            let t = a[i as usize];
            a[i as usize] = a[(4u16 - i) as usize];
            a[(4u16 - i) as usize] = t;
            i = i + 1u16;
        }
        a[0] * 100u16 + a[4]
    }); // 5*100 + 1 = 501
}

#[test]
fn byte_arrays() {
    // u8 arrays store/load one byte per element; values widen to u16 with `as`.
    check!({
        let mut a = [0u8; 4];
        a[2] = 200u8;
        a[2] as u16
    }); // 200
    check!({
        let a = [10u8, 20u8, 30u8, 250u8];
        a[0] as u16 + a[3] as u16
    }); // 260
        // Low-byte truncation must match `as u8`.
    check!({
        let mut a = [0u8; 2];
        a[0] = 300u16 as u8;
        a[0] as u16
    }); // 300 as u8 = 44
        // Fill a byte array in a loop, read back.
    check!({
        let mut a = [0u8; 5];
        let mut i = 0u16;
        while i < 5u16 {
            a[i as usize] = (i * 10u16) as u8;
            i = i + 1u16;
        }
        a[4] as u16
    }); // 40
}

#[test]
fn array_fill() {
    // `[v; N]` block fill — word (const / zero / runtime value) and byte — vs rustc.
    fn host() -> u16 {
        let a = [7u16; 10]; // word, const
        let b = [0u16; 5]; // zero
        let n = 3u16;
        let c = [n; 8]; // word, runtime value
        let d = [5u8; 4]; // byte
        let mut s = 0u16;
        for i in 0..10 {
            s = s.wrapping_add(a[i]);
        }
        for i in 0..5 {
            s = s.wrapping_add(b[i]);
        }
        for i in 0..8 {
            s = s.wrapping_add(c[i]);
        }
        for i in 0..4 {
            s = s.wrapping_add(d[i] as u16);
        }
        s // 70 + 0 + 24 + 20 = 114
    }
    let src = "
        fn run() -> u16 {
            let a = [7u16; 10];
            let b = [0u16; 5];
            let n = 3u16;
            let c = [n; 8];
            let d = [5u8; 4];
            let mut s = 0u16;
            let mut i = 0u16;
            while i < 10u16 { s = s.wrapping_add(a[i as usize]); i = i + 1u16; }
            let mut j = 0u16;
            while j < 5u16 { s = s.wrapping_add(b[j as usize]); j = j + 1u16; }
            let mut k = 0u16;
            while k < 8u16 { s = s.wrapping_add(c[k as usize]); k = k + 1u16; }
            let mut m = 0u16;
            while m < 4u16 { s = s.wrapping_add(d[m as usize] as u16); m = m + 1u16; }
            s
        }
    ";
    let prog = rustz80::compile_program(src).expect("compile");
    assert_eq!(run_program(&prog, "run"), host()); // 114
}
