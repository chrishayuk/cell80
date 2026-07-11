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
//!
//! **F2 — owned transcendentals** (`fexp`/`fln`/`fpow`, the amendment's §F2): class
//! **approximate**, NOT bit-exact-vs-rustc — rustc's answer here *is* platform libm's,
//! the thing being escaped. Each carries a declared ULP bound over its domain
//! (`//! accuracy:` in the consuming cell's manifest, verified in
//! `tests/diff/f32_trans.rs` against offline-MPFR golden tables — measured, never
//! assumed). Cody–Waite reduction + the Cephes single-precision minimax polynomials
//! (Moshier's coefficients, as u32 bit-pattern literals with the generator pinned in
//! `tests/data/gen_f32_trans_golden.py`), composed over the correctly-rounded F0 ops —
//! so cross-target bit-identity is inherited, and "kernels never halt" holds
//! unchanged (`fln(x<0)` → NaN, `fln(0)` → −Inf, `fexp` over/underflow → +Inf/0,
//! `fpow(x<0, y)` → NaN with `pow(0,0) = 1` and `pow(1, y) = pow(x, 0) = 1` pinned to
//! Rust's `powf`; domain policing stays at the cell boundary, `0xFF06`/`finite_result`).

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

fn fexp(x: u32) -> u32 {
    let mag = x & 0x7FFFFFFFu32;
    let sgn = x >> 31u32;
    let mut result = 0u32;
    let mut done = 0u32;
    if mag > 0x7F800000u32 {
        result = 0x7FC00000u32;
        done = 1u32;
    } else if mag == 0x7F800000u32 {
        result = 0u32;
        if sgn == 0u32 {
            result = 0x7F800000u32;
        }
        done = 1u32;
    } else if sgn == 0u32 && mag >= 0x42B17218u32 {
        result = 0x7F800000u32;
        done = 1u32;
    } else if sgn == 1u32 && mag >= 0x42CFF1B5u32 {
        result = 0u32;
        done = 1u32;
    } else if mag == 0u32 {
        result = 0x3F800000u32;
        done = 1u32;
    }
    if done == 0u32 {
        let nf = fround(fmul(x, 0x3FB8AA3Bu32));
        let nmag = nf & 0x7FFFFFFFu32;
        let mut nv = 0u32;
        if nmag != 0u32 {
            let mut m24 = (nmag & 0x7FFFFFu32) | 0x800000u32;
            let mut k = 150u32 - (nmag >> 23u32);
            while k != 0u32 {
                m24 = m24 >> 1u32;
                k = k - 1u32;
            }
            nv = m24;
        }
        let t1 = fmul(nf, 0x3F318000u32);
        let hi = fsub(x, t1);
        let t2 = fmul(nf, 0xB95E8083u32);
        let r = fsub(hi, t2);
        let mut p = 0x39506967u32;
        p = fadd(fmul(p, r), 0x3AB743CEu32);
        p = fadd(fmul(p, r), 0x3C088908u32);
        p = fadd(fmul(p, r), 0x3D2AA9C1u32);
        p = fadd(fmul(p, r), 0x3E2AAAAAu32);
        p = fadd(fmul(p, r), 0x3F000000u32);
        let er = fadd(fadd(fmul(fmul(r, r), p), r), 0x3F800000u32);
        let k1 = nv >> 1u32;
        let k2 = nv - k1;
        let mut s1 = 0u32;
        let mut s2 = 0u32;
        if (nf >> 31u32) == 1u32 {
            s1 = (127u32 - k1) << 23u32;
            s2 = (127u32 - k2) << 23u32;
        } else {
            s1 = (127u32 + k1) << 23u32;
            s2 = (127u32 + k2) << 23u32;
        }
        result = fmul(fmul(er, s1), s2);
    }
    result
}

fn fln(x: u32) -> u32 {
    let mag = x & 0x7FFFFFFFu32;
    let sgn = x >> 31u32;
    let mut result = 0u32;
    let mut done = 0u32;
    if mag > 0x7F800000u32 {
        result = 0x7FC00000u32;
        done = 1u32;
    } else if mag == 0u32 {
        result = 0xFF800000u32;
        done = 1u32;
    } else if sgn == 1u32 {
        result = 0x7FC00000u32;
        done = 1u32;
    } else if mag == 0x7F800000u32 {
        result = 0x7F800000u32;
        done = 1u32;
    }
    if done == 0u32 {
        let mut bits = mag;
        let mut esub = 0u32;
        if bits < 0x800000u32 {
            bits = fmul(bits, 0x4C000000u32);
            esub = 25u32;
        }
        let eb = bits >> 23u32;
        let xh = (bits & 0x7FFFFFu32) | 0x3F000000u32;
        let sub = 126u32 + esub;
        let mut emag = 0u32;
        let mut eneg = 0u32;
        if eb >= sub {
            emag = eb - sub;
        } else {
            emag = sub - eb;
            eneg = 1u32;
        }
        let mut x1 = 0u32;
        if xh < 0x3F3504F3u32 {
            if eneg == 1u32 {
                emag = emag + 1u32;
            } else if emag == 0u32 {
                emag = 1u32;
                eneg = 1u32;
            } else {
                emag = emag - 1u32;
            }
            x1 = fsub(fadd(xh, xh), 0x3F800000u32);
        } else {
            x1 = fsub(xh, 0x3F800000u32);
        }
        let z = fmul(x1, x1);
        let mut p = 0x3D9021BBu32;
        p = fadd(fmul(p, x1), 0xBDEBD1B8u32);
        p = fadd(fmul(p, x1), 0x3DEF251Au32);
        p = fadd(fmul(p, x1), 0xBDFE5D4Fu32);
        p = fadd(fmul(p, x1), 0x3E11E9BFu32);
        p = fadd(fmul(p, x1), 0xBE2AAE50u32);
        p = fadd(fmul(p, x1), 0x3E4CCEACu32);
        p = fadd(fmul(p, x1), 0xBE7FFFFCu32);
        p = fadd(fmul(p, x1), 0x3EAAAAAAu32);
        let mut y = fmul(fmul(x1, z), p);
        // fe = (float) signed e — built from bits directly (emag <= 176, always
        // exact; `int_to_f32` is a typed builtin whose F32 result couldn't take
        // the sign-bit OR below without crossing representations).
        let mut fe = 0u32;
        if emag != 0u32 {
            let mut t = emag;
            let mut pw = 0u32;
            while t > 1u32 {
                t = t >> 1u32;
                pw = pw + 1u32;
            }
            let mut mm = emag;
            let mut k = 23u32 - pw;
            while k != 0u32 {
                mm = mm << 1u32;
                k = k - 1u32;
            }
            fe = ((127u32 + pw) << 23u32) | (mm & 0x7FFFFFu32);
            if eneg == 1u32 {
                fe = fe | 0x80000000u32;
            }
        }
        y = fadd(y, fmul(fe, 0xB95E8083u32));
        y = fsub(y, fmul(z, 0x3F000000u32));
        let xy = fadd(x1, y);
        result = fadd(xy, fmul(fe, 0x3F318000u32));
    }
    result
}

fn fsin(x: u32) -> u32 {
    let mag = x & 0x7FFFFFFFu32;
    let mut result = 0u32;
    let mut done = 0u32;
    if mag >= 0x7F800000u32 {
        result = 0x7FC00000u32;
        done = 1u32;
    } else if mag > 0x46000000u32 {
        result = 0x7FC00000u32;
        done = 1u32;
    } else if mag == 0u32 {
        result = x;
        done = 1u32;
    }
    if done == 0u32 {
        let nf = fround(fmul(x, 0x3F22F983u32));
        let nmag = nf & 0x7FFFFFFFu32;
        let mut nv = 0u32;
        if nmag != 0u32 {
            let mut m24 = (nmag & 0x7FFFFFu32) | 0x800000u32;
            let mut k = 150u32 - (nmag >> 23u32);
            while k != 0u32 {
                m24 = m24 >> 1u32;
                k = k - 1u32;
            }
            nv = m24;
        }
        let mut q = nv & 3u32;
        if (nf >> 31u32) == 1u32 && q != 0u32 {
            q = 4u32 - q;
        }
        let t1 = fmul(nf, 0x3FC90000u32);
        let mut r = fsub(x, t1);
        let t2 = fmul(nf, 0x39FDA000u32);
        r = fsub(r, t2);
        let t3 = fmul(nf, 0x33A22169u32);
        r = fsub(r, t3);
        let z = fmul(r, r);
        let mut v = 0u32;
        if q == 0u32 || q == 2u32 {
            let mut p = 0xB94CA1F9u32;
            p = fadd(fmul(p, z), 0x3C08839Eu32);
            p = fadd(fmul(p, z), 0xBE2AAAA3u32);
            let rz = fmul(r, z);
            v = fadd(fmul(rz, p), r);
        } else {
            let mut p = 0x37CCF5CEu32;
            p = fadd(fmul(p, z), 0xBAB6061Au32);
            p = fadd(fmul(p, z), 0x3D2AAAA5u32);
            let zz = fmul(z, z);
            let a1 = fmul(zz, p);
            let a2 = fmul(z, 0xBF000000u32);
            let t = fadd(a1, a2);
            v = fadd(t, 0x3F800000u32);
        }
        if q >= 2u32 {
            v = v ^ 0x80000000u32;
        }
        result = v;
    }
    result
}

fn fcos(x: u32) -> u32 {
    let mag = x & 0x7FFFFFFFu32;
    let mut result = 0u32;
    let mut done = 0u32;
    if mag >= 0x7F800000u32 {
        result = 0x7FC00000u32;
        done = 1u32;
    } else if mag > 0x46000000u32 {
        result = 0x7FC00000u32;
        done = 1u32;
    } else if mag == 0u32 {
        result = 0x3F800000u32;
        done = 1u32;
    }
    if done == 0u32 {
        let nf = fround(fmul(x, 0x3F22F983u32));
        let nmag = nf & 0x7FFFFFFFu32;
        let mut nv = 0u32;
        if nmag != 0u32 {
            let mut m24 = (nmag & 0x7FFFFFu32) | 0x800000u32;
            let mut k = 150u32 - (nmag >> 23u32);
            while k != 0u32 {
                m24 = m24 >> 1u32;
                k = k - 1u32;
            }
            nv = m24;
        }
        let mut q = nv & 3u32;
        if (nf >> 31u32) == 1u32 && q != 0u32 {
            q = 4u32 - q;
        }
        let t1 = fmul(nf, 0x3FC90000u32);
        let mut r = fsub(x, t1);
        let t2 = fmul(nf, 0x39FDA000u32);
        r = fsub(r, t2);
        let t3 = fmul(nf, 0x33A22169u32);
        r = fsub(r, t3);
        let z = fmul(r, r);
        let mut v = 0u32;
        if q == 0u32 || q == 2u32 {
            let mut p = 0x37CCF5CEu32;
            p = fadd(fmul(p, z), 0xBAB6061Au32);
            p = fadd(fmul(p, z), 0x3D2AAAA5u32);
            let zz = fmul(z, z);
            let a1 = fmul(zz, p);
            let a2 = fmul(z, 0xBF000000u32);
            let t = fadd(a1, a2);
            v = fadd(t, 0x3F800000u32);
        } else {
            let mut p = 0xB94CA1F9u32;
            p = fadd(fmul(p, z), 0x3C08839Eu32);
            p = fadd(fmul(p, z), 0xBE2AAAA3u32);
            let rz = fmul(r, z);
            v = fadd(fmul(rz, p), r);
        }
        if q == 1u32 || q == 2u32 {
            v = v ^ 0x80000000u32;
        }
        result = v;
    }
    result
}

fn fatan2(y: u32, x: u32) -> u32 {
    let ymag = y & 0x7FFFFFFFu32;
    let xmag = x & 0x7FFFFFFFu32;
    let ys = y >> 31u32;
    let xs = x >> 31u32;
    let mut result = 0u32;
    let mut done = 0u32;
    if ymag > 0x7F800000u32 || xmag > 0x7F800000u32 {
        result = 0x7FC00000u32;
        done = 1u32;
    } else if ymag == 0u32 {
        result = ys << 31u32;
        if xs == 1u32 {
            result = 0x40490FDBu32 | (ys << 31u32);
        }
        done = 1u32;
    } else if xmag == 0u32 {
        result = 0x3FC90FDBu32 | (ys << 31u32);
        done = 1u32;
    } else if ymag == 0x7F800000u32 && xmag == 0x7F800000u32 {
        result = 0x3F490FDBu32;
        if xs == 1u32 {
            result = 0x4016CBE4u32;
        }
        result = result | (ys << 31u32);
        done = 1u32;
    } else if ymag == 0x7F800000u32 {
        result = 0x3FC90FDBu32 | (ys << 31u32);
        done = 1u32;
    } else if xmag == 0x7F800000u32 {
        result = ys << 31u32;
        if xs == 1u32 {
            result = 0x40490FDBu32 | (ys << 31u32);
        }
        done = 1u32;
    }
    if done == 0u32 {
        let mut num = ymag;
        let mut den = xmag;
        let mut inv = 0u32;
        if ymag > xmag {
            num = xmag;
            den = ymag;
            inv = 1u32;
        }
        let t = fdiv(num, den);
        let mut w = t;
        let mut bias = 0u32;
        if t > 0x3ED413CDu32 {
            let tm1 = fsub(w, 0x3F800000u32);
            let tp1 = fadd(w, 0x3F800000u32);
            w = fdiv(tm1, tp1);
            bias = 1u32;
        }
        let z = fmul(w, w);
        let mut p = 0x3DA4F0D1u32;
        p = fadd(fmul(p, z), 0xBE0E1B85u32);
        p = fadd(fmul(p, z), 0x3E4C925Fu32);
        p = fadd(fmul(p, z), 0xBEAAAA2Au32);
        let wz = fmul(w, z);
        let mut base = fadd(fmul(wz, p), w);
        if bias == 1u32 {
            base = fadd(base, 0x3F490FDBu32);
        }
        if inv == 1u32 {
            base = fsub(0x3FC90FDBu32, base);
        }
        if xs == 1u32 {
            base = fsub(0x40490FDBu32, base);
        }
        result = base | (ys << 31u32);
    }
    result
}

fn fpow(a: u32, b: u32) -> u32 {
    let amag = a & 0x7FFFFFFFu32;
    let bmag = b & 0x7FFFFFFFu32;
    let mut result = 0u32;
    let mut done = 0u32;
    if bmag == 0u32 {
        result = 0x3F800000u32;
        done = 1u32;
    } else if a == 0x3F800000u32 {
        result = 0x3F800000u32;
        done = 1u32;
    } else if amag > 0x7F800000u32 || bmag > 0x7F800000u32 {
        result = 0x7FC00000u32;
        done = 1u32;
    } else if (a >> 31u32) == 1u32 && amag != 0u32 {
        result = 0x7FC00000u32;
        done = 1u32;
    } else if amag == 0u32 {
        result = 0u32;
        if (b >> 31u32) == 1u32 {
            result = 0x7F800000u32;
        }
        done = 1u32;
    }
    if done == 0u32 {
        result = fexp(fmul(b, fln(a)));
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
    // F2 owned transcendentals (approximate class — declared ULP bounds, not
    // bit-exact-vs-rustc; see the F2 note in the module doc). Composed over the
    // correctly-rounded F0 ops, so their determinism story is F0's.
    (
        "fexp",
        &["fround", "fmul", "fsub", "fadd", "f32_shr_jam", "f32_pack"],
    ),
    ("fln", &["fadd", "fsub", "fmul", "f32_shr_jam", "f32_pack"]),
    (
        "fpow",
        &[
            "fexp",
            "fln",
            "fmul",
            "fround",
            "fsub",
            "fadd",
            "f32_shr_jam",
            "f32_pack",
        ],
    ),
    (
        "fsin",
        &["fround", "fmul", "fsub", "fadd", "f32_shr_jam", "f32_pack"],
    ),
    (
        "fcos",
        &["fround", "fmul", "fsub", "fadd", "f32_shr_jam", "f32_pack"],
    ),
    (
        "fatan2",
        &["fdiv", "fmul", "fsub", "fadd", "f32_shr_jam", "f32_pack"],
    ),
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
///
/// The F2 transcendentals (`fexp`/`fln`/`fpow`) **deliberately stay out**: adding
/// them changes the bank image ⇒ new bank hash ⇒ every existing banked artifact
/// hard-refuses to load (same-bank-or-refuse), and the size math is tight (the
/// family ≈ 4.6–6.3 KB vs ~5 KB of headroom under `0xFF00`). They ride the
/// non-bank append path (the `int_to_f32` precedent) — their `fadd`/`fmul` calls
/// still resolve into the bank — and residency waits for a deliberate, measured
/// rebank event.
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
