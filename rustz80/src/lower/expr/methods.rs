//! Method-call lowering: wrapping/saturating/bit/str methods and handle routing.

use super::super::Ctx;
use super::*;
use crate::ir::*;

/// Lower a method call: the `wrapping_*` value ops, or `obj.m(a, b)` →
/// `Type::m(&obj, a, b)` (`self` passed as a leading pointer).
pub(crate) fn lower_method_call(
    m: &syn::ExprMethodCall,
    ctx: &mut Ctx,
) -> Result<(Expr, Width), String> {
    let method = m.method.to_string();
    // f32's method surface. Kernel-backed: `.sqrt()`, the rounding family
    // (`.floor()`/`.ceil()`/`.trunc()`/`.round()` — `round` is Rust's
    // half-away-from-zero), and `.min()`/`.max()` (Rust "NaN is missing data",
    // with -0 < +0 and sNaN-ignored pinned deterministic). Pure-bits sugar:
    // `.abs()`, `.copysign(b)`, and the classification trio
    // `.is_nan()`/`.is_finite()`/`.is_subnormal()` (inline compares, no kernel).
    if let "sqrt" | "abs" | "floor" | "ceil" | "trunc" | "round" | "min" | "max" | "copysign"
    | "is_nan" | "is_finite" | "is_subnormal" = method.as_str()
    {
        let (recv, rw) = lower_expr(&m.receiver, ctx)?;
        if rw != Width::F32 {
            return Err(format!(
                "`.{method}()` is defined on f32 — for integers use the kernel \
                 helpers (`isqrt`, `imin`/`imax`, `iabs_diff`)"
            ));
        }
        let unary_kernel = match method.as_str() {
            "sqrt" => Some("fsqrt"),
            "floor" => Some("ffloor"),
            "ceil" => Some("fceil"),
            "trunc" => Some("ftrunc"),
            "round" => Some("fround"),
            _ => None,
        };
        if let Some(kernel) = unary_kernel {
            if !m.args.is_empty() {
                return Err(format!("`.{method}()` takes no arguments"));
            }
            ctx.mark_f32(kernel);
            return Ok((Expr::Call(kernel.to_string(), vec![recv]), Width::F32));
        }
        if method == "min" || method == "max" {
            let arg = m
                .args
                .first()
                .ok_or("`.min()`/`.max()` take one argument")?;
            let (ae, aw) = lower_expr(arg, ctx)?;
            require_f32_pair(rw, aw)?;
            if f32_operand_effects(&recv) && f32_operand_effects(&ae) {
                return Err(format!(
                    "both operands of `.{method}()` have side effects and the wide \
                     call convention reorders them — hoist one to a `let` binding"
                ));
            }
            let kernel = if method == "min" { "fmin" } else { "fmax" };
            ctx.mark_f32(kernel);
            return Ok((Expr::Call(kernel.to_string(), vec![recv, ae]), Width::F32));
        }
        if method == "abs" {
            if !m.args.is_empty() {
                return Err("`.abs()` takes no arguments".into());
            }
            return Ok((
                Expr::Bin32(
                    BinOp::And,
                    Box::new(recv),
                    Box::new(Expr::Lit32(0x7FFF_FFFF)),
                    false,
                ),
                Width::F32,
            ));
        }
        if method == "copysign" {
            let arg = m.args.first().ok_or("`.copysign()` takes one argument")?;
            let (ae, aw) = lower_expr(arg, ctx)?;
            require_f32_pair(rw, aw)?;
            // magnitude of recv | sign of arg — pure bits, rustc-identical
            return Ok((
                Expr::Bin32(
                    BinOp::Or,
                    Box::new(Expr::Bin32(
                        BinOp::And,
                        Box::new(recv),
                        Box::new(Expr::Lit32(0x7FFF_FFFF)),
                        false,
                    )),
                    Box::new(Expr::Bin32(
                        BinOp::And,
                        Box::new(ae),
                        Box::new(Expr::Lit32(0x8000_0000)),
                        false,
                    )),
                    false,
                ),
                Width::F32,
            ));
        }
        // classification trio — the receiver evaluates once per compare, so it
        // must be pure (a var or literal); effectful receivers bind first
        if !m.args.is_empty() {
            return Err(format!("`.{method}()` takes no arguments"));
        }
        if has_effects(&recv) {
            return Err(format!(
                "`.{method}()` re-reads its receiver — bind the value first: `let v = …;`"
            ));
        }
        let mag = |e: &Expr| {
            Expr::Bin32(
                BinOp::And,
                Box::new(e.clone()),
                Box::new(Expr::Lit32(0x7FFF_FFFF)),
                false,
            )
        };
        let cmp32 = |cmp, lhs, rhs: u32| Expr::Cmp32 {
            cmp,
            lhs: Box::new(lhs),
            rhs: Box::new(Expr::Lit32(rhs)),
            signed: false,
        };
        let e = match method.as_str() {
            "is_nan" => cmp32(Cmp::Gt, mag(&recv), 0x7F80_0000),
            "is_finite" => cmp32(Cmp::Lt, mag(&recv), 0x7F80_0000),
            _ => Expr::Logic {
                and: true,
                lhs: Box::new(cmp32(Cmp::Lt, mag(&recv), 0x0080_0000)),
                rhs: Box::new(cmp32(Cmp::Ne, mag(&recv), 0)),
            },
        };
        return Ok((e, Width::Byte));
    }
    if let "wrapping_add" | "wrapping_sub" | "wrapping_mul" | "wrapping_div" | "wrapping_rem" =
        method.as_str()
    {
        let op = match method.as_str() {
            "wrapping_add" => BinOp::Add,
            "wrapping_sub" => BinOp::Sub,
            "wrapping_div" => BinOp::Div,
            "wrapping_rem" => BinOp::Rem,
            _ => BinOp::Mul,
        };
        let (recv, rw) = lower_expr(&m.receiver, ctx)?;
        let arg = m.args.first().ok_or("wrapping_* needs an argument")?;
        let (re, aw) = lower_expr(arg, ctx)?;
        if rw == Width::F32 || aw == Width::F32 {
            return Err(format!(
                "`.{method}()` is integer-only — f32 arithmetic is `+ - * /` \
                 (correctly rounded; it doesn't wrap)"
            ));
        }
        // A `u32` receiver/argument makes it a 32-bit op (all `Bin32` arithmetic is
        // mod-2^32, i.e. wrapping, already).
        if rw.is_int_wide() || aw.is_int_wide() {
            let signed = rw == Width::SDWord || aw == Width::SDWord;
            if signed && (rw == Width::DWord || aw == Width::DWord) {
                return Err("i32 and u32 don't mix — cast one side explicitly \
                            (`as i32` / `as u32`)"
                    .into());
            }
            return Ok((
                Expr::Bin32(
                    op,
                    Box::new(coerce32s(recv, rw, signed)),
                    Box::new(coerce32s(re, aw, signed)),
                    signed,
                ),
                if signed { Width::SDWord } else { Width::DWord },
            ));
        }
        return Ok((Expr::Bin(op, Box::new(recv), Box::new(re), rw), rw));
    }
    if let "saturating_add" | "saturating_sub" | "saturating_mul" = method.as_str() {
        return lower_saturating(&method, m, ctx);
    }
    if let "count_ones" | "leading_zeros" | "trailing_zeros" | "rotate_left" | "rotate_right"
    | "swap_bytes" = method.as_str()
    {
        return lower_bit_method(&method, m, ctx);
    }
    let recv = path_ident(&m.receiver)?;
    // Prelude handles (`frame`/`input`): route methods to intrinsic prelude fns.
    if let Some(handle) = ctx.vars.handle_of(&recv) {
        return lower_prelude_call(&handle, &method, &m.args, ctx);
    }
    // `&str` parameters: the accepted string methods (Phase S §2.1).
    if ctx.vars.str_param(&recv) {
        let base = ctx.vars.base(&recv);
        return lower_str_method(base, &recv, &method, &m.args, ctx);
    }
    let (base, sname, is_ptr) = ctx
        .vars
        .receiver(&recv)
        .ok_or_else(|| format!("method receiver {recv} is not a struct"))?;
    let self_ptr = if is_ptr {
        Expr::Var(base)
    } else {
        Expr::AddrOf(base)
    };
    let mut args = vec![self_ptr];
    for a in &m.args {
        args.push(lower_expr16(a, ctx, "method argument")?);
    }
    if args.len() > 3 {
        return Err("method receiver + args exceed 3 registers".into());
    }
    Ok((Expr::Call(format!("{sname}::{method}"), args), Width::Word))
}

/// Lower `a.saturating_add(b)` / `_sub` / `_mul` (u8/u16) — real Rust, so the
/// oracle checks it. Clamping desugars to branch-free mask arithmetic:
///
/// - `add`: `s = a + b (wrapping)`, overflow iff `s < a` → `s | (0 - ovf)`
/// - `sub`: `d = a - b (wrapping)`, in range iff `a >= b` → `d & (0 - ok)`
/// - `mul`: the full product (u16: via a u32 widen; u8: a 16-bit product),
///   overflow iff the high part is nonzero → `lo | (0 - ovf)`
///
/// Operands are re-read by the clamp, so they must be pure (`has_effects`).
/// `u32` saturating waits on 32-bit comparisons; `i16` clamps to a *signed* range
/// (`-32768..=32767`) that this mask trick doesn't express — both reject with
/// steering messages.
fn lower_saturating(
    method: &str,
    m: &syn::ExprMethodCall,
    ctx: &mut Ctx,
) -> Result<(Expr, Width), String> {
    let (recv, rw) = lower_expr(&m.receiver, ctx)?;
    let arg = m.args.first().ok_or("saturating_* needs an argument")?;
    let (re, aw) = lower_expr(arg, ctx)?;
    if rw == Width::SWord || aw == Width::SWord || rw == Width::SDWord || aw == Width::SDWord {
        return Err(
            "i16/i32 saturating_* is not supported — the clamp bounds are signed; \
             write the comparison explicitly"
                .into(),
        );
    }
    if has_effects(&recv) || has_effects(&re) {
        return Err(
            "saturating_* needs simple operands here (the clamp re-reads them) — \
             bind them first: `let x = …;`"
                .into(),
        );
    }
    // u32: the same mask trick over the 32-bit nodes (`Cmp32` supplies the flag).
    if rw == Width::DWord || aw == Width::DWord {
        let (recv, re) = (coerce32(recv, rw), coerce32(re, aw));
        let bin32 = |op, a: Expr, b: Expr| Expr::Bin32(op, Box::new(a), Box::new(b), false);
        let cmp32 = |c, lhs: Expr, rhs: Expr| Expr::Cmp32 {
            cmp: c,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
            signed: false,
        };
        // `0 - flag` widened: `0xFFFF_FFFF` when set, `0` otherwise.
        let mask32 = |flag: Expr| bin32(BinOp::Sub, Expr::Lit32(0), Expr::Widen(Box::new(flag)));
        let e = match method {
            "saturating_add" => {
                let s = bin32(BinOp::Add, recv.clone(), re);
                let ovf = cmp32(Cmp::Lt, s.clone(), recv);
                bin32(BinOp::Or, s, mask32(ovf))
            }
            "saturating_sub" => {
                let ok = cmp32(Cmp::Ge, recv.clone(), re.clone());
                let d = bin32(BinOp::Sub, recv, re);
                bin32(BinOp::And, d, mask32(ok))
            }
            // No 64-bit product needed: with the wrapped product `p = a * b`,
            // overflow ⇔ `a != 0 && p / a != b` (the classic post-hoc check —
            // wrapping subtracts `k·2^32 ≥ 2^32 > a·b'` from the true product, so
            // the quotient can't land back on `b`). The division is a real cost
            // (one more trap) and short-circuits behind the zero test, which also
            // keeps the div-by-zero policy out of reach.
            _ => {
                let p = bin32(BinOp::Mul, recv.clone(), re.clone());
                let nz = cmp32(Cmp::Ne, recv.clone(), Expr::Lit32(0));
                let q = bin32(BinOp::Div, p.clone(), recv);
                let bad = cmp32(Cmp::Ne, q, re);
                let ovf = Expr::Logic {
                    and: true,
                    lhs: Box::new(nz),
                    rhs: Box::new(bad),
                };
                bin32(BinOp::Or, p, mask32(ovf))
            }
        };
        return Ok((e, Width::DWord));
    }
    // The receiver decides the width (as Rust's inference does for the argument).
    let w = rw;
    let bin = |op, a: Expr, b: Expr, w| Expr::Bin(op, Box::new(a), Box::new(b), w);
    let cmp = |c, lhs: Expr, rhs: Expr| Expr::Cmp {
        cmp: c,
        lhs: Box::new(lhs),
        rhs: Box::new(rhs),
        signed: false,
    };
    // `0 - flag` at width `w`: `0xFFFF`/`0xFF` when the flag is set, `0` otherwise.
    let mask = |flag: Expr, w| bin(BinOp::Sub, Expr::Lit(0), flag, w);
    let e = match method {
        "saturating_add" => {
            let s = bin(BinOp::Add, recv.clone(), re, w);
            let ovf = cmp(Cmp::Lt, s.clone(), recv);
            bin(BinOp::Or, s, mask(ovf, w), w)
        }
        "saturating_sub" => {
            let ok = cmp(Cmp::Ge, recv.clone(), re.clone());
            let d = bin(BinOp::Sub, recv, re, w);
            bin(BinOp::And, d, mask(ok, w), w)
        }
        _ => match w {
            // u8: the full product fits a 16-bit word.
            Width::Byte => {
                let p = bin(BinOp::Mul, recv, re, Width::Word);
                let ovf = cmp(Cmp::Gt, p.clone(), Expr::Lit(0xFF));
                bin(BinOp::Or, p, mask(ovf, Width::Byte), Width::Byte)
            }
            // u16: widen to a u32 product; overflow iff the high word is nonzero.
            _ => {
                let p = Expr::Bin32(
                    BinOp::Mul,
                    Box::new(Expr::Widen(Box::new(recv))),
                    Box::new(Expr::Widen(Box::new(re))),
                    false,
                );
                let hi = Expr::Trunc32(Box::new(Expr::Shift32 {
                    left: false,
                    e: Box::new(p.clone()),
                    k: 16,
                    signed: false,
                }));
                let ovf = cmp(Cmp::Ne, hi, Expr::Lit(0));
                bin(
                    BinOp::Or,
                    Expr::Trunc32(Box::new(p)),
                    mask(ovf, Width::Word),
                    Width::Word,
                )
            }
        },
    };
    Ok((e, w))
}

/// Lower the std bit methods (u8/u16) — every one is real Rust, so the oracle
/// checks it. The counting trio call tiny appended kernels (`__bits_*` — plain Z80
/// loops, same bytes both targets, **no traps**); `rotate_*`/`swap_bytes` desugar
/// to shift-and-or, re-reading their operands (pure operands required):
///
/// | method | u16 | u8 |
/// |---|---|---|
/// | `count_ones` | `CALL __bits_count_ones` | same (high byte is 0) |
/// | `leading_zeros` | `CALL __bits_leading_zeros` | that minus 8 |
/// | `trailing_zeros` | `CALL __bits_trailing_zeros` | of `x \| 0x100` (0 → 8) |
/// | `swap_bytes` | `(x << 8) \| (x >> 8)` | identity |
/// | `rotate_left(k)` | `(x << k') \| (x >> 16 - k')`, `k' = k % 16` | same at 8 |
///
/// Note the counting results are `u16` here where std returns `u32` — every
/// in-range use (`x.count_ones() as u16`, comparisons, arithmetic) agrees; the
/// value never exceeds 16.
fn lower_bit_method(
    method: &str,
    m: &syn::ExprMethodCall,
    ctx: &mut Ctx,
) -> Result<(Expr, Width), String> {
    let (recv, rw) = lower_expr(&m.receiver, ctx)?;
    if rw.is_int_wide() {
        return Err(format!(
            "u32/i32 `{method}` is not supported yet — split the words and combine"
        ));
    }
    if rw == Width::SWord {
        return Err(format!(
            "i16 `{method}` is not supported — cast to the bit pattern first (`as u16`)"
        ));
    }
    let call = |name: &str, arg: Expr| Expr::Call(name.to_string(), vec![arg]);
    let bin = |op, a: Expr, b: Expr, w| Expr::Bin(op, Box::new(a), Box::new(b), w);
    let byte = rw == Width::Byte;
    match method {
        "count_ones" => return Ok((call("__bits_count_ones", recv), Width::Word)),
        "leading_zeros" => {
            let lz = call("__bits_leading_zeros", recv);
            return Ok((
                if byte {
                    bin(BinOp::Sub, lz, Expr::Lit(8), Width::Word)
                } else {
                    lz
                },
                Width::Word,
            ));
        }
        "trailing_zeros" => {
            let arg = if byte {
                // Bit 8 caps a byte's trailing-zero count at 8 (and handles 0).
                bin(BinOp::Or, recv, Expr::Lit(0x100), Width::Word)
            } else {
                recv
            };
            return Ok((call("__bits_trailing_zeros", arg), Width::Word));
        }
        _ => {}
    }
    // The shift-and-or desugars re-read the value (and the rotate amount).
    if has_effects(&recv) {
        return Err(format!(
            "`{method}` needs a simple value here (the lowering re-reads it) — \
             bind it first: `let x = …;`"
        ));
    }
    let bits: u16 = if byte { 8 } else { 16 };
    let w = rw;
    if method == "swap_bytes" {
        // u8::swap_bytes is the identity.
        if byte {
            return Ok((recv, w));
        }
        return Ok((
            bin(
                BinOp::Or,
                bin(BinOp::Shl, recv.clone(), Expr::Lit(8), w),
                bin(BinOp::Shr, recv, Expr::Lit(8), w),
                w,
            ),
            w,
        ));
    }
    // rotate_left / rotate_right, amount `k` rotated mod the width.
    let left = method == "rotate_left";
    let arg = m.args.first().ok_or("rotate_* takes the rotate amount")?;
    if let syn::Expr::Lit(l) = arg {
        if let syn::Lit::Int(i) = &l.lit {
            // Constant amount: unrolled literal shifts (k' = 0 is the identity).
            let k = i.base10_parse::<u16>().map_err(|e| e.to_string())? % bits;
            if k == 0 {
                return Ok((recv, w));
            }
            let (a, b) = if left { (k, bits - k) } else { (bits - k, k) };
            return Ok((
                bin(
                    BinOp::Or,
                    bin(BinOp::Shl, recv.clone(), Expr::Lit(a), w),
                    bin(BinOp::Shr, recv, Expr::Lit(b), w),
                    w,
                ),
                w,
            ));
        }
    }
    // Runtime amount: `k' = k & (bits-1)`; the opposite shift is `bits - k'`
    // (`bits` when `k' = 0`, which shifts out to 0 — the identity falls out).
    // std's rotate amount is a `u32` — a wide amount narrows freely, the rotate
    // only reads `k % bits`.
    let k = match lower_expr(arg, ctx)? {
        (e, w) if w.is_int_wide() => Expr::Trunc32(Box::new(e)),
        (e, _) => e,
    };
    if has_effects(&k) {
        return Err(format!(
            "`{method}` needs a simple rotate amount here (it is re-read) — \
             bind it first: `let k = …;`"
        ));
    }
    let km = bin(BinOp::And, k, Expr::Lit(bits - 1), Width::Word);
    let opp = bin(BinOp::Sub, Expr::Lit(bits), km.clone(), Width::Word);
    let shift = |dir_left: bool, e: Expr, amount: Expr| Expr::ShiftVar {
        left: dir_left,
        e: Box::new(e),
        amount: Box::new(amount),
        w,
    };
    Ok((
        bin(
            BinOp::Or,
            shift(left, recv.clone(), km),
            shift(!left, recv, opp),
            w,
        ),
        w,
    ))
}

/// Lower a method call on a `&str` parameter (Phase S §2.1). Every accepted method
/// is real Rust with identical semantics, so `check_str!` keeps the rustc oracle:
///
/// | call | lowering |
/// |---|---|
/// | `s.len()` | 16-bit load at `s` (the u16 LE length prefix) |
/// | `s.is_empty()` | that load `== 0` |
/// | `s.as_bytes()[i]` | byte load at `s + 2 + i` (handled in [`lower_index_read`]) |
/// | `s.is_char_boundary(i)` | `i == 0 \|\| i == len \|\| (i < len && (b[i] & 0xC0) != 0x80)` |
///
/// `is_char_boundary` matches `str::is_char_boundary` exactly — including
/// `i > len` ⇒ `false` — and short-circuits so the byte read stays in bounds.
fn lower_str_method(
    base: usize,
    recv: &str,
    method: &str,
    args: &syn::punctuated::Punctuated<syn::Expr, syn::token::Comma>,
    ctx: &mut Ctx,
) -> Result<(Expr, Width), String> {
    // The length: a 16-bit load at the buffer address.
    let len = || Expr::LoadAt(Box::new(Expr::Var(base)), Width::Word);
    let cmp = |cmp, lhs: Expr, rhs: Expr| Expr::Cmp {
        cmp,
        lhs: Box::new(lhs),
        rhs: Box::new(rhs),
        signed: false,
    };
    match method {
        "len" => Ok((len(), Width::Word)),
        "is_empty" => Ok((cmp(Cmp::Eq, len(), Expr::Lit(0)), Width::Byte)),
        "as_bytes" => Err(format!(
            "`{recv}.as_bytes()` is only indexed in the dialect — read a byte with \
             `{recv}.as_bytes()[i]` (no slice values)"
        )),
        "is_char_boundary" => {
            let arg = args
                .first()
                .ok_or("`is_char_boundary` takes the byte index")?;
            let idx = lower_expr16(arg, ctx, "char-boundary index")?;
            // The index is reused across the comparisons, so it must be pure —
            // a call could be re-invoked with side effects.
            if has_effects(&idx) {
                return Err(format!(
                    "`{recv}.is_char_boundary(…)` needs a simple index here — bind it \
                     first: `let i = …;`"
                ));
            }
            // b[i] & 0xC0 != 0x80 — "not a UTF-8 continuation byte".
            let byte_addr = Expr::Bin(
                BinOp::Add,
                Box::new(Expr::Var(base)),
                Box::new(Expr::Bin(
                    BinOp::Add,
                    Box::new(idx.clone()),
                    Box::new(Expr::Lit(2)),
                    Width::Word,
                )),
                Width::Word,
            );
            let not_cont = cmp(
                Cmp::Ne,
                Expr::Bin(
                    BinOp::And,
                    Box::new(Expr::LoadAt(Box::new(byte_addr), Width::Byte)),
                    Box::new(Expr::Lit(0xC0)),
                    Width::Byte,
                ),
                Expr::Lit(0x80),
            );
            // i == 0 || i == len || (i < len && not_cont) — `i > len` is false,
            // exactly `str::is_char_boundary`.
            let in_bounds = Expr::Logic {
                and: true,
                lhs: Box::new(cmp(Cmp::Lt, idx.clone(), len())),
                rhs: Box::new(not_cont),
            };
            let tail = Expr::Logic {
                and: false,
                lhs: Box::new(cmp(Cmp::Eq, idx.clone(), len())),
                rhs: Box::new(in_bounds),
            };
            Ok((
                Expr::Logic {
                    and: false,
                    lhs: Box::new(cmp(Cmp::Eq, idx, Expr::Lit(0))),
                    rhs: Box::new(tail),
                },
                Width::Byte,
            ))
        }
        other => Err(format!(
            "`{other}` isn't a `&str` method in the dialect — a string is \
             length-prefixed bytes: `{recv}.len()`, `{recv}.is_empty()`, \
             `{recv}.as_bytes()[i]`, `{recv}.is_char_boundary(i)`; anything more \
             is host/escalation territory"
        )),
    }
}
