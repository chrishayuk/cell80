//! The owned IEEE binary32 softfloat kernels — the F0 wave of
//! `docs/real-valued-cells-amendment.md`.
//!
//! Integer arithmetic all the way down: sign/exponent/mantissa unpack, align, operate,
//! round-to-nearest-even, repack. rustc's `f32` *basic* ops are bit-specified IEEE on
//! every host (no libm involved), so release rustc is the portable golden reference —
//! `tests/diff/f32_ops.rs` asserts bit equality post NaN-canonicalization, on both
//! compile targets, across an edge bank and a seeded random bank (H-F1: there is no
//! "close enough" band). Subnormals are handled exactly (gradual underflow, as rustc
//! does), signed zeros preserved, Inf/NaN propagated per IEEE. Every NaN produced is
//! the canonical quiet NaN `0x7FC0_0000` (hardware NaN payloads differ across
//! x86/ARM; WASM canonicalizes for the same reason).
//!
//! Kernels never `halt()` — unlike the checked-arithmetic family, an in-kernel trap
//! would diverge from the golden reference. The escalate-not-lie discipline lives at
//! the cell boundary (`finite_result`, codes `0xFF07`/`0xFF08`), not here.
//!
//! Dialect constraints that shaped the text: u32 shifts take **literal amounts only**,
//! so variable-distance shifts are shift-by-1 loops (`f32_shr_jam`, which also jams
//! the out-shifted bits into a sticky bit); at most **two u32 params** per fn, so
//! `f32_pack` takes `(sign << 16) | exponent` as one word; single-exit bodies keep
//! every kernel eligible for the single-site inline fold. Internal formats: `m30` =
//! 24-bit significand + 6 guard/round/sticky bits (6, not 3, so a post-subtraction
//! normalize shift cannot move a rounding boundary across the jam bit); exponents ride
//! a +300 offset where intermediate values can go negative.

/// Restricted-dialect source of the kernel five (+ two helpers), appended to the cell
/// prelude by `cell80` and compiled per cell — DCE prunes whatever a cell doesn't call.
pub const F32_KERNELS: &str = r#"
fn f32_shr_jam(x: u32, n: u32) -> u32 {
    let mut r = x;
    if n > 31u32 {
        r = 0u32;
        if x != 0u32 {
            r = 1u32;
        }
    } else {
        let mut i = n;
        let mut st = 0u32;
        while i != 0u32 {
            st = st | (r & 1u32);
            r = r >> 1u32;
            i = i - 1u32;
        }
        r = r | st;
    }
    r
}

fn f32_pack(m30: u32, se: u32) -> u32 {
    let s = se >> 16u32;
    let e = se & 0xFFFFu32;
    let mut m = m30 >> 6u32;
    let low = m30 & 63u32;
    if low > 32u32 || (low == 32u32 && (m & 1u32) == 1u32) {
        m = m + 1u32;
    }
    let mut r = 0u32;
    if e >= 255u32 {
        r = (s << 31u32) | 0x7F800000u32;
    } else {
        r = (s << 31u32) + ((e - 1u32) << 23u32) + m;
    }
    r
}

fn fadd(a: u32, b: u32) -> u32 {
    let mut result = 0u32;
    let mut done = 0u32;
    let amag = a & 0x7FFFFFFFu32;
    let bmag = b & 0x7FFFFFFFu32;
    if amag > 0x7F800000u32 || bmag > 0x7F800000u32 {
        result = 0x7FC00000u32;
        done = 1u32;
    } else if amag == 0x7F800000u32 {
        result = a;
        if bmag == 0x7F800000u32 && (a >> 31u32) != (b >> 31u32) {
            result = 0x7FC00000u32;
        }
        done = 1u32;
    } else if bmag == 0x7F800000u32 {
        result = b;
        done = 1u32;
    }
    if done == 0u32 {
        let mut x = a;
        let mut y = b;
        if bmag > amag {
            x = b;
            y = a;
        }
        let xs = x >> 31u32;
        let ys = y >> 31u32;
        let xm = x & 0x7FFFFFFFu32;
        let ym = y & 0x7FFFFFFFu32;
        if ym == 0u32 {
            result = x;
            if xm == 0u32 {
                result = (xs & ys) << 31u32;
            }
        } else {
            let mut ex = xm >> 23u32;
            let mut mx = xm & 0x7FFFFFu32;
            let mut ey = ym >> 23u32;
            let mut my = ym & 0x7FFFFFu32;
            if ex == 0u32 {
                ex = 1u32;
            } else {
                mx = mx | 0x800000u32;
            }
            if ey == 0u32 {
                ey = 1u32;
            } else {
                my = my | 0x800000u32;
            }
            let mx6 = mx << 6u32;
            let my6 = f32_shr_jam(my << 6u32, ex - ey);
            if xs == ys {
                let mut sum = mx6 + my6;
                if sum >= 0x40000000u32 {
                    sum = (sum >> 1u32) | (sum & 1u32);
                    ex = ex + 1u32;
                }
                result = f32_pack(sum, (xs << 16u32) | ex);
            } else {
                let diff = mx6 - my6;
                if diff == 0u32 {
                    result = 0u32;
                } else {
                    let mut d = diff;
                    while d < 0x20000000u32 && ex > 1u32 {
                        d = d << 1u32;
                        ex = ex - 1u32;
                    }
                    result = f32_pack(d, (xs << 16u32) | ex);
                }
            }
        }
    }
    result
}

fn fsub(a: u32, b: u32) -> u32 {
    fadd(a, b ^ 0x80000000u32)
}

fn fmul(a: u32, b: u32) -> u32 {
    let s = (a >> 31u32) ^ (b >> 31u32);
    let amag = a & 0x7FFFFFFFu32;
    let bmag = b & 0x7FFFFFFFu32;
    let mut result = 0u32;
    let mut done = 0u32;
    if amag > 0x7F800000u32 || bmag > 0x7F800000u32 {
        result = 0x7FC00000u32;
        done = 1u32;
    } else if amag == 0x7F800000u32 || bmag == 0x7F800000u32 {
        result = (s << 31u32) | 0x7F800000u32;
        if amag == 0u32 || bmag == 0u32 {
            result = 0x7FC00000u32;
        }
        done = 1u32;
    } else if amag == 0u32 || bmag == 0u32 {
        result = s << 31u32;
        done = 1u32;
    }
    if done == 0u32 {
        let mut ea = (amag >> 23u32) + 300u32;
        let mut ma = amag & 0x7FFFFFu32;
        if ea == 300u32 {
            ea = 301u32;
            while ma < 0x800000u32 {
                ma = ma << 1u32;
                ea = ea - 1u32;
            }
        } else {
            ma = ma | 0x800000u32;
        }
        let mut eb = (bmag >> 23u32) + 300u32;
        let mut mb = bmag & 0x7FFFFFu32;
        if eb == 300u32 {
            eb = 301u32;
            while mb < 0x800000u32 {
                mb = mb << 1u32;
                eb = eb - 1u32;
            }
        } else {
            mb = mb | 0x800000u32;
        }
        let ah = ma >> 16u32;
        let al = ma & 0xFFFFu32;
        let bh = mb >> 16u32;
        let bl = mb & 0xFFFFu32;
        let lo = al * bl;
        let mid = ah * bl + al * bh;
        let hi = ah * bh;
        let hi48 = (hi << 16u32) + mid + (lo >> 16u32);
        let low16 = lo & 0xFFFFu32;
        let mut e = ea + eb - 427u32;
        let mut q26 = 0u32;
        let mut st = low16;
        if hi48 >= 0x80000000u32 {
            q26 = hi48 >> 6u32;
            st = st | (hi48 & 63u32);
            e = e + 1u32;
        } else {
            q26 = hi48 >> 5u32;
            st = st | (hi48 & 31u32);
        }
        if st != 0u32 {
            st = 1u32;
        }
        let mut m30 = (q26 << 4u32) | st;
        if e <= 300u32 {
            m30 = f32_shr_jam(m30, 301u32 - e);
            e = 301u32;
        }
        result = f32_pack(m30, (s << 16u32) | (e - 300u32));
    }
    result
}

fn fdiv(a: u32, b: u32) -> u32 {
    let s = (a >> 31u32) ^ (b >> 31u32);
    let amag = a & 0x7FFFFFFFu32;
    let bmag = b & 0x7FFFFFFFu32;
    let mut result = 0u32;
    let mut done = 0u32;
    if amag > 0x7F800000u32 || bmag > 0x7F800000u32 {
        result = 0x7FC00000u32;
        done = 1u32;
    } else if amag == 0x7F800000u32 {
        result = (s << 31u32) | 0x7F800000u32;
        if bmag == 0x7F800000u32 {
            result = 0x7FC00000u32;
        }
        done = 1u32;
    } else if bmag == 0x7F800000u32 {
        result = s << 31u32;
        done = 1u32;
    } else if bmag == 0u32 {
        result = (s << 31u32) | 0x7F800000u32;
        if amag == 0u32 {
            result = 0x7FC00000u32;
        }
        done = 1u32;
    } else if amag == 0u32 {
        result = s << 31u32;
        done = 1u32;
    }
    if done == 0u32 {
        let mut ea = (amag >> 23u32) + 300u32;
        let mut ma = amag & 0x7FFFFFu32;
        if ea == 300u32 {
            ea = 301u32;
            while ma < 0x800000u32 {
                ma = ma << 1u32;
                ea = ea - 1u32;
            }
        } else {
            ma = ma | 0x800000u32;
        }
        let mut eb = (bmag >> 23u32) + 300u32;
        let mut mb = bmag & 0x7FFFFFu32;
        if eb == 300u32 {
            eb = 301u32;
            while mb < 0x800000u32 {
                mb = mb << 1u32;
                eb = eb - 1u32;
            }
        } else {
            mb = mb | 0x800000u32;
        }
        let mut e = ea - eb + 427u32;
        let mut num = ma;
        if num < mb {
            num = num << 1u32;
            e = e - 1u32;
        }
        let mut q26 = 0u32;
        let mut i = 0u32;
        while i < 26u32 {
            q26 = q26 << 1u32;
            if num >= mb {
                num = num - mb;
                q26 = q26 | 1u32;
            }
            num = num << 1u32;
            i = i + 1u32;
        }
        let mut st = 0u32;
        if num != 0u32 {
            st = 1u32;
        }
        let mut m30 = (q26 << 4u32) | st;
        if e <= 300u32 {
            m30 = f32_shr_jam(m30, 301u32 - e);
            e = 301u32;
        }
        result = f32_pack(m30, (s << 16u32) | (e - 300u32));
    }
    result
}

fn fsqrt(a: u32) -> u32 {
    let amag = a & 0x7FFFFFFFu32;
    let mut result = 0u32;
    let mut done = 0u32;
    if amag > 0x7F800000u32 {
        result = 0x7FC00000u32;
        done = 1u32;
    } else if amag == 0u32 {
        result = a;
        done = 1u32;
    } else if (a >> 31u32) == 1u32 {
        result = 0x7FC00000u32;
        done = 1u32;
    } else if amag == 0x7F800000u32 {
        result = a;
        done = 1u32;
    }
    if done == 0u32 {
        let mut ea = (amag >> 23u32) + 300u32;
        let mut ma = amag & 0x7FFFFFu32;
        if ea == 300u32 {
            ea = 301u32;
            while ma < 0x800000u32 {
                ma = ma << 1u32;
                ea = ea - 1u32;
            }
        } else {
            ma = ma | 0x800000u32;
        }
        let r = ea & 1u32;
        let mut hi = 0u32;
        if r == 0u32 {
            hi = ma << 8u32;
        } else {
            hi = (ma << 1u32) << 6u32;
        }
        let mut root = 0u32;
        let mut rem = 0u32;
        let mut i = 0u32;
        while i < 26u32 {
            rem = (rem << 2u32) | (hi >> 30u32);
            hi = hi << 2u32;
            let t = (root << 2u32) | 1u32;
            root = root << 1u32;
            if rem >= t {
                rem = rem - t;
                root = root | 1u32;
            }
            i = i + 1u32;
        }
        let mut st = 0u32;
        if rem != 0u32 {
            st = 1u32;
        }
        let m30 = (root << 4u32) | st;
        let t_off = (ea - r + 150u32) >> 1u32;
        let e = t_off - 162u32 + r;
        result = f32_pack(m30, e);
    }
    result
}
"#;
