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

/// Restricted-dialect source of the whole family: the kernel five, the comparison
/// trio (`feq`/`flt`/`fle` — Rust semantics exactly: NaN compares false, -0 == +0;
/// `>`/`>=` lower as swapped `flt`/`fle`), the F1 set (conversions
/// `int_to_f32`/`f32_to_int_trunc`/`q16_to_f32`/`f32_to_q16` — the `f32_to_*`
/// pair halts typed `0xFF08 float_domain` on NaN/out-of-range, deliberate boundary
/// behaviour, *not* rustc's saturating cast; the rounding family
/// `ftrunc`/`ffloor`/`fceil`/`fround` — `fround` is Rust's round-half-away, not RNE;
/// and `fmin`/`fmax` — Rust "NaN is missing data" semantics, with two deterministic
/// pins where rustc itself is unspecified: `-0 < +0`, and a *signaling* NaN is
/// ignored like a quiet one), and two helpers. Appended to the cell prelude by
/// `cell80` and compiled per cell — DCE prunes whatever a cell doesn't call.
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


fn feq(a: u32, b: u32) -> u32 {
    let amag = a & 0x7FFFFFFFu32;
    let bmag = b & 0x7FFFFFFFu32;
    let mut r = 0u32;
    if amag <= 0x7F800000u32 && bmag <= 0x7F800000u32 {
        if a == b || (amag == 0u32 && bmag == 0u32) {
            r = 1u32;
        }
    }
    r
}

fn flt(a: u32, b: u32) -> u32 {
    let amag = a & 0x7FFFFFFFu32;
    let bmag = b & 0x7FFFFFFFu32;
    let mut r = 0u32;
    if amag <= 0x7F800000u32 && bmag <= 0x7F800000u32 {
        let sa = a >> 31u32;
        let sb = b >> 31u32;
        if sa != sb {
            if sa == 1u32 && (amag != 0u32 || bmag != 0u32) {
                r = 1u32;
            }
        } else if sa == 0u32 {
            if amag < bmag {
                r = 1u32;
            }
        } else if bmag < amag {
            r = 1u32;
        }
    }
    r
}

fn fle(a: u32, b: u32) -> u32 {
    let amag = a & 0x7FFFFFFFu32;
    let bmag = b & 0x7FFFFFFFu32;
    let mut r = 0u32;
    if amag <= 0x7F800000u32 && bmag <= 0x7F800000u32 {
        let sa = a >> 31u32;
        let sb = b >> 31u32;
        if sa != sb {
            if sa == 1u32 || (amag == 0u32 && bmag == 0u32) {
                r = 1u32;
            }
        } else if sa == 0u32 {
            if amag <= bmag {
                r = 1u32;
            }
        } else if bmag <= amag {
            r = 1u32;
        }
    }
    r
}


fn int_to_f32(i: u32) -> u32 {
    let mut result = 0u32;
    if i != 0u32 {
        let mut m = i;
        let mut e = 158u32;
        while m < 0x80000000u32 {
            m = m << 1u32;
            e = e - 1u32;
        }
        let mut m30 = m >> 2u32;
        if m & 3u32 != 0u32 {
            m30 = m30 | 1u32;
        }
        result = f32_pack(m30, e);
    }
    result
}

fn q16_to_f32(q: u32) -> u32 {
    let mut result = 0u32;
    if q != 0u32 {
        let mut m = q;
        let mut e = 142u32;
        while m < 0x80000000u32 {
            m = m << 1u32;
            e = e - 1u32;
        }
        let mut m30 = m >> 2u32;
        if m & 3u32 != 0u32 {
            m30 = m30 | 1u32;
        }
        result = f32_pack(m30, e);
    }
    result
}

fn f32_to_int_trunc(a: u32) -> u32 {
    let amag = a & 0x7FFFFFFFu32;
    let mut result = 0u32;
    let mut bad = 0u32;
    if amag > 0x7F800000u32 {
        bad = 1u32;
    } else if amag < 0x3F800000u32 {
        result = 0u32;
    } else if (a >> 31u32) == 1u32 {
        bad = 1u32;
    } else if amag >= 0x4F800000u32 {
        bad = 1u32;
    } else {
        let e = (amag >> 23u32) - 127u32;
        let m24 = (amag & 0x7FFFFFu32) | 0x800000u32;
        if e <= 23u32 {
            let mut r = m24;
            let mut k = 23u32 - e;
            while k != 0u32 {
                r = r >> 1u32;
                k = k - 1u32;
            }
            result = r;
        } else {
            let mut r = m24;
            let mut k = e - 23u32;
            while k != 0u32 {
                r = r << 1u32;
                k = k - 1u32;
            }
            result = r;
        }
    }
    if bad != 0u32 {
        halt(0xFF08u16);
    }
    result
}

fn f32_to_q16(a: u32) -> u32 {
    let amag = a & 0x7FFFFFFFu32;
    let mut result = 0u32;
    let mut bad = 0u32;
    if amag > 0x7F800000u32 {
        bad = 1u32;
    } else if amag < 0x37800000u32 {
        result = 0u32;
    } else if (a >> 31u32) == 1u32 {
        bad = 1u32;
    } else if amag >= 0x47800000u32 {
        bad = 1u32;
    } else {
        let e = (amag >> 23u32) - 111u32;
        let m24 = (amag & 0x7FFFFFu32) | 0x800000u32;
        if e <= 23u32 {
            let mut r = m24;
            let mut k = 23u32 - e;
            while k != 0u32 {
                r = r >> 1u32;
                k = k - 1u32;
            }
            result = r;
        } else {
            let mut r = m24;
            let mut k = e - 23u32;
            while k != 0u32 {
                r = r << 1u32;
                k = k - 1u32;
            }
            result = r;
        }
    }
    if bad != 0u32 {
        halt(0xFF08u16);
    }
    result
}

fn ftrunc(a: u32) -> u32 {
    let amag = a & 0x7FFFFFFFu32;
    let s = a & 0x80000000u32;
    let mut result = a;
    if amag > 0x7F800000u32 {
        result = 0x7FC00000u32;
    } else if amag < 0x3F800000u32 {
        result = s;
    } else if amag < 0x4B000000u32 {
        let mut mask = 0x7FFFFFu32;
        let mut k = (amag >> 23u32) - 127u32;
        while k != 0u32 {
            mask = mask >> 1u32;
            k = k - 1u32;
        }
        result = s | (amag & (0xFFFFFFFFu32 ^ mask));
    }
    result
}

fn ffloor(a: u32) -> u32 {
    let amag = a & 0x7FFFFFFFu32;
    let s = a & 0x80000000u32;
    let mut result = a;
    if amag > 0x7F800000u32 {
        result = 0x7FC00000u32;
    } else if amag < 0x3F800000u32 {
        result = s;
        if s != 0u32 && amag != 0u32 {
            result = 0xBF800000u32;
        }
    } else if amag < 0x4B000000u32 {
        let mut mask = 0x7FFFFFu32;
        let mut k = (amag >> 23u32) - 127u32;
        while k != 0u32 {
            mask = mask >> 1u32;
            k = k - 1u32;
        }
        let mut m = amag & (0xFFFFFFFFu32 ^ mask);
        if s != 0u32 && (amag & mask) != 0u32 {
            m = m + mask + 1u32;
        }
        result = s | m;
    }
    result
}

fn fceil(a: u32) -> u32 {
    let amag = a & 0x7FFFFFFFu32;
    let s = a & 0x80000000u32;
    let mut result = a;
    if amag > 0x7F800000u32 {
        result = 0x7FC00000u32;
    } else if amag < 0x3F800000u32 {
        result = s;
        if s == 0u32 && amag != 0u32 {
            result = 0x3F800000u32;
        }
    } else if amag < 0x4B000000u32 {
        let mut mask = 0x7FFFFFu32;
        let mut k = (amag >> 23u32) - 127u32;
        while k != 0u32 {
            mask = mask >> 1u32;
            k = k - 1u32;
        }
        let mut m = amag & (0xFFFFFFFFu32 ^ mask);
        if s == 0u32 && (amag & mask) != 0u32 {
            m = m + mask + 1u32;
        }
        result = s | m;
    }
    result
}

fn fround(a: u32) -> u32 {
    let amag = a & 0x7FFFFFFFu32;
    let s = a & 0x80000000u32;
    let mut result = a;
    if amag > 0x7F800000u32 {
        result = 0x7FC00000u32;
    } else if amag < 0x3F000000u32 {
        result = s;
    } else if amag < 0x3F800000u32 {
        result = s | 0x3F800000u32;
    } else if amag < 0x4B000000u32 {
        let mut mask = 0x7FFFFFu32;
        let mut k = (amag >> 23u32) - 127u32;
        while k != 0u32 {
            mask = mask >> 1u32;
            k = k - 1u32;
        }
        let half = (mask >> 1u32) + 1u32;
        let m = (amag + half) & (0xFFFFFFFFu32 ^ mask);
        result = s | m;
    }
    result
}

fn fmin(a: u32, b: u32) -> u32 {
    let amag = a & 0x7FFFFFFFu32;
    let bmag = b & 0x7FFFFFFFu32;
    let mut result = 0u32;
    if amag > 0x7F800000u32 && bmag > 0x7F800000u32 {
        result = 0x7FC00000u32;
    } else if amag > 0x7F800000u32 {
        result = b;
    } else if bmag > 0x7F800000u32 {
        result = a;
    } else if flt(a, b) == 1u32 {
        result = a;
    } else if flt(b, a) == 1u32 {
        result = b;
    } else {
        result = a;
        if (b >> 31u32) == 1u32 {
            result = b;
        }
    }
    result
}

fn fmax(a: u32, b: u32) -> u32 {
    let amag = a & 0x7FFFFFFFu32;
    let bmag = b & 0x7FFFFFFFu32;
    let mut result = 0u32;
    if amag > 0x7F800000u32 && bmag > 0x7F800000u32 {
        result = 0x7FC00000u32;
    } else if amag > 0x7F800000u32 {
        result = b;
    } else if bmag > 0x7F800000u32 {
        result = a;
    } else if flt(a, b) == 1u32 {
        result = b;
    } else if flt(b, a) == 1u32 {
        result = a;
    } else {
        result = a;
        if (b >> 31u32) == 0u32 {
            result = b;
        }
    }
    result
}
"#;

/// Transitive kernel dependencies — which helper `Func`s each kernel's `Func` needs
/// when the f32 sugar auto-appends them (a name here must match a `fn` in
/// [`F32_KERNELS`]). Kept honest by the diff bank: a missing dep is an unresolved
/// symbol at encode, and every kernel is exercised source-level in `f32_ops.rs`.
pub(crate) const KERNEL_DEPS: &[(&str, &[&str])] = &[
    ("fadd", &["f32_shr_jam", "f32_pack"]),
    ("fsub", &["fadd", "f32_shr_jam", "f32_pack"]),
    ("fmul", &["f32_shr_jam", "f32_pack"]),
    ("fdiv", &["f32_shr_jam", "f32_pack"]),
    ("fsqrt", &["f32_pack"]),
    ("feq", &[]),
    ("flt", &[]),
    ("fle", &[]),
    ("int_to_f32", &["f32_pack"]),
    ("q16_to_f32", &["f32_pack"]),
    ("f32_to_int_trunc", &[]),
    ("f32_to_q16", &[]),
    ("ftrunc", &[]),
    ("ffloor", &[]),
    ("fceil", &[]),
    ("fround", &[]),
    ("fmin", &["flt"]),
    ("fmax", &["flt"]),
];

/// Where the resident kernel bank lives in the Cell VM's map: code at
/// [`BANK_ORG`], the bank's *own* locals at [`BANK_SCRATCH`] (disjoint from the
/// calling cell's scratch at `0x9000+`, or a kernel call would clobber its
/// caller's locals), everything below the stack.
pub const BANK_ORG: u16 = 0xC000;
/// The bank's private register file: `0xB800..0xC000` (state ends well below).
pub const BANK_SCRATCH: u16 = 0xB800;

/// The bank membership: the arithmetic five, the comparison trio, and the two
/// helpers — the heavy shared family. Conversions/rounding/min-max stay
/// per-cell (they're small and often absent); when compiled banked they resolve
/// `f32_pack`/`f32_shr_jam`/`flt` into the bank by name.
pub const BANK_FNS: &[&str] = &[
    "fadd",
    "fsub",
    "fmul",
    "fdiv",
    "fsqrt",
    "feq",
    "flt",
    "fle",
    "f32_shr_jam",
    "f32_pack",
];

/// The call-boundary shapes of the bank fns (`(wide first, wide second, wide
/// ret)`), for seeding `wide_sigs` when the definitions aren't local.
pub(crate) const BANK_WIDE_SIGS: &[(&str, (bool, bool, bool))] = &[
    ("fadd", (true, true, true)),
    ("fsub", (true, true, true)),
    ("fmul", (true, true, true)),
    ("fdiv", (true, true, true)),
    ("fsqrt", (true, false, true)),
    ("feq", (true, true, true)),
    ("flt", (true, true, true)),
    ("fle", (true, true, true)),
    ("f32_shr_jam", (true, true, true)),
    ("f32_pack", (true, true, true)),
];

/// The resident kernel bank: [`BANK_FNS`] compiled once at [`BANK_ORG`] with
/// locals at [`BANK_SCRATCH`]. Deterministic (same compiler ⇒ same bytes), so
/// its content identity can enter a cartridge's artifact-hash context the way
/// the trap table's semantics already do. Built lazily, cached for the process.
pub struct KernelBank {
    /// The bank image, loaded verbatim at [`BANK_ORG`].
    pub code: Vec<u8>,
    /// Absolute entry addresses by kernel name.
    pub symbols: std::collections::HashMap<String, u16>,
}

pub fn kernel_bank() -> &'static KernelBank {
    use std::sync::OnceLock;
    static BANK: OnceLock<KernelBank> = OnceLock::new();
    BANK.get_or_init(|| {
        let file: syn::File = syn::parse_str(F32_KERNELS).expect("kernel source parses");
        let kept: Vec<syn::Item> = file
            .items
            .into_iter()
            .filter(|item| match item {
                syn::Item::Fn(f) => BANK_FNS.contains(&f.sig.ident.to_string().as_str()),
                _ => false,
            })
            .collect();
        let file = syn::File {
            shebang: None,
            attrs: Vec::new(),
            items: kept,
        };
        let funcs = crate::lower::lower_program(&file, &crate::lower::PreludeConfig::default())
            .expect("bank lowers");
        let (code, symbols) =
            crate::codegen::codegen_bank(&funcs, BANK_ORG, BANK_SCRATCH).expect("bank encodes");
        assert!(
            BANK_ORG as usize + code.len() <= 0xFF00,
            "kernel bank outgrew its region ({} bytes)",
            code.len()
        );
        KernelBank { code, symbols }
    })
}
