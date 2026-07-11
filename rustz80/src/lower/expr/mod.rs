//! Expression lowering: a `syn::Expr` → IR [`Expr`] plus its inferred [`Width`].
//! The dispatch (`lower_expr`) and the shared width/effect helpers live here; the
//! topic modules carry the rest — [`binary`] (operators, comparisons, shifts, the
//! f32 kernel routing), [`place`] (field/index/const access: a constant slot for a
//! by-value struct, an indirect load/store through `self`-style pointers), and
//! [`methods`] (`wrapping_*`, bit/str methods, prelude-handle routing,
//! `obj.m(a) → Type::m(&obj, a)`).

mod binary;
mod methods;
mod place;

use binary::lower_binary;
pub(crate) use binary::{f32_cmp_call, f32_operand_effects, require_f32_pair};
pub(crate) use methods::lower_method_call;
pub(crate) use place::{array_base, lower_field_store, lower_index_store};
use place::{lower_const_ref, lower_field_read, lower_index_read};

use super::generics::{call_target, resolve_generic};
use super::layout::resolve_enum_path;
use super::Ctx;
use crate::ir::*;

/// Lower an expression that must fit a 16-bit context (a slot store, a call register,
/// a comparison operand, an index): a `u32` value is a clean lowering error — never a
/// codegen panic. `what` names the context in the message.
pub(crate) fn lower_expr16(expr: &syn::Expr, ctx: &mut Ctx, what: &str) -> Result<Expr, String> {
    let (e, w) = lower_expr(expr, ctx)?;
    if w.is_wide() {
        return Err(format!(
            "u32 value in a 16-bit context ({what}) — narrow with `as u16`"
        ));
    }
    Ok(e)
}

/// Coerce a lowered operand into a 32-bit position: a `u32` passes through, a literal
/// becomes a `u32` literal, and any other 16-bit value zero-extends (`Widen`) — this is
/// the unsuffixed-literal mixing rustc itself allows (`part as u32 * 100`).
pub(crate) fn coerce32(e: Expr, w: Width) -> Expr {
    coerce32s(e, w, false)
}

/// [`coerce32`] into a lane of known signedness: a 16-bit side entering an **i32**
/// lane sign-extends when it is itself signed (`i16`) — rustc's `as` semantics —
/// and zero-extends otherwise (a non-negative literal reads the same either way).
pub(crate) fn coerce32s(e: Expr, w: Width, signed: bool) -> Expr {
    if w.is_wide() {
        e
    } else if let Expr::Lit(k) = e {
        Expr::Lit32(k as u32)
    } else if signed && w == Width::SWord {
        Expr::SignExtend(Box::new(e))
    } else {
        Expr::Widen(Box::new(e))
    }
}

/// Lower an expression, returning its IR and inferred width (`u8`/`u16`).
pub(crate) fn lower_expr(expr: &syn::Expr, ctx: &mut Ctx) -> Result<(Expr, Width), String> {
    match expr {
        syn::Expr::Lit(l) => match &l.lit {
            syn::Lit::Int(i) => {
                // `2f32` (no decimal point) tokenizes as an *int* literal with a
                // float suffix — it is an f32 value, same compile-time RNE parse
                // as `2.0f32` (and `2f64` is out, like every f64).
                if i.suffix() == "f32" {
                    let v: f32 = i
                        .base10_digits()
                        .parse()
                        .map_err(|e| format!("bad f32 literal: {e}"))?;
                    return Ok((Expr::Lit32(v.to_bits()), Width::F32));
                }
                if i.suffix() == "f64" {
                    return Err(format!(
                        "`{}` — f64 is out of the dialect (demand-gated, no named \
                         customer; the F-wave amendment); use `f32`",
                        i.token()
                    ));
                }
                if i.suffix() == "u32" {
                    return Ok((
                        Expr::Lit32(i.base10_parse::<u32>().map_err(|e| e.to_string())?),
                        Width::DWord,
                    ));
                }
                if i.suffix() == "i32" {
                    // Parse the magnitude (a negative literal arrives as `-` around
                    // it, so `-2147483648i32`'s magnitude must parse); bits are u32.
                    let m = i.base10_parse::<i64>().map_err(|e| e.to_string())?;
                    if m > 0x8000_0000i64 {
                        return Err(format!("`{m}` is out of range for i32"));
                    }
                    return Ok((Expr::Lit32(m as u32), Width::SDWord));
                }
                if i.suffix() == "i16" {
                    // Parse the magnitude (a negative literal arrives as `-` around it,
                    // so `-32768i16`'s magnitude 32768 must parse); the bits are u16.
                    let m = i.base10_parse::<u16>().map_err(|e| e.to_string())?;
                    if m > 32768 {
                        return Err(format!("`{m}` is out of range for i16"));
                    }
                    return Ok((Expr::Lit(m), Width::SWord));
                }
                let w = if i.suffix() == "u8" {
                    Width::Byte
                } else {
                    Width::Word
                };
                Ok((
                    Expr::Lit(i.base10_parse::<u16>().map_err(|e| e.to_string())?),
                    w,
                ))
            }
            syn::Lit::Bool(b) => Ok((Expr::Lit(b.value as u16), Width::Byte)),
            // A string literal is interned into the const-data pool (length-prefixed:
            // a little-endian u16 length at `s`, byte `i` at `s + 2 + i` — the Phase S
            // wire format) and evaluates to its address — so
            // `frame.text(x, y, "SCORE")` hands the routine a pointer.
            syn::Lit::Str(s) => {
                let name = ctx.consts.borrow_mut().intern_str(&s.value())?;
                Ok((Expr::ConstAddr(name), Width::Word))
            }
            // A byte literal is a `u8` value — ASCII work reads as itself
            // (`c >= b'0' && c <= b'9'`) instead of magic numbers.
            syn::Lit::Byte(b) => Ok((Expr::Lit(b.value() as u16), Width::Byte)),
            // A byte-string literal is packed const data — raw bytes, no length
            // prefix (its `&[u8; N]` type carries the length) — and evaluates to
            // its address, deduplicated by content.
            syn::Lit::ByteStr(bs) => {
                let name = ctx.consts.borrow_mut().intern_bytes(&bs.value())?;
                Ok((Expr::ConstAddr(name), Width::Word))
            }
            // An `f32`-suffixed literal converts at compile time — Rust's own
            // decimal→binary32 parse is correctly rounded (RNE), so the bits match
            // what rustc gives the same token. The suffix is *required*: an
            // unsuffixed decimal (`12.5`) belongs to the canon pass's exact-decimal
            // lane (the fraction tier), and in rustc it would infer f64 anyway.
            syn::Lit::Float(fl) => {
                if fl.suffix() == "f64" {
                    return Err(format!(
                        "`{}` — f64 is out of the dialect (demand-gated, no named \
                         customer; the F-wave amendment); use `f32`",
                        fl.token()
                    ));
                }
                if fl.suffix() != "f32" {
                    return Err(format!(
                        "`{}` — an unsuffixed decimal is not a dialect value: suffix it \
                         `f32` for binary32 (the owned softfloat tier), or leave it to \
                         the canon pass's exact-decimal lifting (the fraction tier)",
                        fl.token()
                    ));
                }
                let v: f32 = fl.base10_parse().map_err(|e| e.to_string())?;
                Ok((Expr::Lit32(v.to_bits()), Width::F32))
            }
            other => Err(format!(
                "unsupported literal: {} — the dialect's values are integers, bools, and \
                 byte literals (`b'a'`), plus (as data) string and byte-string literals; \
                 floats/chars are out — for fractional values use fixed-point on \
                 integers, e.g. Q8.8: `(a * w) >> 8`; for a character use `b'a'`",
                describe_lit(other)
            )),
        },
        syn::Expr::Path(p) => match resolve_enum_path(&p.path, ctx.enums) {
            Some(v) => Ok((Expr::Lit(v), Width::Word)),
            None => {
                let name = path_ident(expr)?;
                // A const-generic parameter is substituted by its instance value.
                if let Some(v) = ctx.const_args.get(&name) {
                    return Ok((Expr::Lit(*v), Width::Word));
                }
                // Program consts (a declared local shadows them): a scalar const
                // substitutes as a literal; a `&str` const's name *is* its address.
                if !ctx.vars.is_declared(&name) {
                    let consts = ctx.consts.borrow();
                    if let Some((v, w)) = consts.scalars.get(&name) {
                        return Ok((Expr::Lit(*v), *w));
                    }
                    if let Some(d) = consts.get(&name) {
                        if d.is_ref {
                            return Ok((Expr::ConstAddr(name), Width::Word));
                        }
                        return Err(format!(
                            "const `{name}` is data, not a value — index it (`{name}[i]`) \
                             or pass its address (`&{name}`)"
                        ));
                    }
                }
                if ctx.vars.wide_array(&name) {
                    return Err(format!(
                        "`{name}` is a `[u32; N]` array, not a value — index it \
                         (`{name}[i]`)"
                    ));
                }
                let base = ctx.vars.base(&name);
                match ctx.vars.ty(&name) {
                    Width::DWord => Ok((Expr::Var32(base), Width::DWord)),
                    Width::SDWord => Ok((Expr::Var32(base), Width::SDWord)),
                    Width::F32 => Ok((Expr::Var32(base), Width::F32)),
                    w => Ok((Expr::Var(base), w)),
                }
            }
        },
        syn::Expr::Paren(p) => lower_expr(&p.expr, ctx),
        // `!b` — logical NOT on the `0`/`1` boolean convention (a `bool` field/flag is a
        // single `0`/`1` slot; comparisons and `&&`/`||` already produce `0`/`1`). Yields
        // `1` iff the operand is `0`. The integer bitwise-NOT meaning of `!` is out of the
        // subset — `!` is for booleans (matches rustc on `bool` operands, which is what the
        // differential oracle checks). In condition position `lower_cond` negates instead.
        syn::Expr::Unary(u) => match u.op {
            syn::UnOp::Not(_) => Ok((
                Expr::Cmp {
                    cmp: Cmp::Eq,
                    lhs: Box::new(lower_expr16(&u.expr, ctx, "`!` operand")?),
                    rhs: Box::new(Expr::Lit(0)),
                    signed: false,
                },
                Width::Byte,
            )),
            // `-x` on a signed (`i16`) operand: two's-complement negation (wrapping —
            // `0 - x` shares the unsigned subtract bits). A negative literal folds.
            syn::UnOp::Neg(_) => {
                let (e, w) = lower_expr(&u.expr, ctx)?;
                match w {
                    // `-x` on f32 flips the sign bit — exactly rustc's negation
                    // (a pure bit op: works on NaN/Inf/zeros identically).
                    Width::F32 => Ok(match e {
                        Expr::Lit32(bits) => (Expr::Lit32(bits ^ 0x8000_0000), Width::F32),
                        e => (
                            Expr::Bin32(
                                BinOp::Xor,
                                Box::new(e),
                                Box::new(Expr::Lit32(0x8000_0000)),
                                false,
                            ),
                            Width::F32,
                        ),
                    }),
                    // `-x` on i32: two's-complement negation over the 32-bit nodes.
                    Width::SDWord => Ok(match e {
                        Expr::Lit32(m) => (Expr::Lit32(m.wrapping_neg()), Width::SDWord),
                        e => (
                            Expr::Bin32(BinOp::Sub, Box::new(Expr::Lit32(0)), Box::new(e), true),
                            Width::SDWord,
                        ),
                    }),
                    Width::SWord => Ok(match e {
                        Expr::Lit(m) => (Expr::Lit(m.wrapping_neg()), Width::SWord),
                        e => (
                            Expr::Bin(
                                BinOp::Sub,
                                Box::new(Expr::Lit(0)),
                                Box::new(e),
                                Width::SWord,
                            ),
                            Width::SWord,
                        ),
                    }),
                    _ => Err("unary `-` needs a signed operand — suffix the literal \
                              (`-5i16` / `-5i32`) or cast first (`x as i16`)"
                        .into()),
                }
            }
            _ => Err(
                "unary `*` is not in the subset (no raw pointers; `!` is for \
                      bools, `-` for i16)"
                    .into(),
            ),
        },
        // `e as u8` truncates to a byte; `as u16`/`as usize` is a no-op (16-bit); a `u32`
        // narrows to its low word/byte (`Trunc32`).
        syn::Expr::Cast(c) => {
            let (e, ew) = lower_expr(&c.expr, ctx)?;
            let tw = ctx.width_of_type(&c.ty);
            if ew == Width::F32 || tw == Width::F32 {
                return Err("`as` casts to/from f32 are not in the dialect — numeric \
                            conversion kernels (`int_to_f32`, `f32_to_int_trunc`, …) \
                            arrive with the F1 wave"
                    .into());
            }
            if ew.is_int_wide() {
                return Ok(match tw {
                    Width::Byte => (
                        Expr::Trunc(Box::new(Expr::Trunc32(Box::new(e)))),
                        Width::Byte,
                    ),
                    Width::Word | Width::SWord => (Expr::Trunc32(Box::new(e)), tw),
                    // 32 → 32 is a bit-identity; the value adopts the target's
                    // signedness (`i32 as u32` / `u32 as i32` — rustc semantics).
                    Width::DWord | Width::SDWord => (e, tw),
                    Width::F32 => unreachable!("f32 casts rejected above"),
                });
            }
            if tw.is_int_wide() {
                // 16 → 32: `i16` sign-extends (rustc's `as` — the A2 explicit bridge),
                // `u8`/`u16` zero-extend; the high word takes the fill.
                // `x as u16 as u32` stays the take-the-bits spelling.
                let wide = if ew == Width::SWord {
                    Expr::SignExtend(Box::new(e))
                } else {
                    Expr::Widen(Box::new(e))
                };
                return Ok((wide, tw));
            }
            if tw == Width::Byte {
                Ok((Expr::Trunc(Box::new(e)), Width::Byte))
            } else {
                Ok((e, Width::Word))
            }
        }
        // `&CONST` / `&CONST[i]` — the address of const data (or of one packed
        // element). This is what lets a routed prelude routine receive a pointer to
        // real tile/string bytes: `frame.tile(&HERO, x, y)`. Borrows of anything
        // else stay out of the subset (locals live in 2-byte slots, so a `&local`
        // would not point at packed data).
        syn::Expr::Reference(r) => lower_const_ref(&r.expr, ctx),
        syn::Expr::Field(f) => lower_field_read(f, ctx),
        syn::Expr::Index(ix) => lower_index_read(ix, ctx),
        syn::Expr::Binary(b) => lower_binary(b, ctx),
        syn::Expr::MethodCall(m) => lower_method_call(m, ctx),
        syn::Expr::Call(c) => {
            let (name, turbofish) = call_target(&c.func)?;
            // `peek(addr)` intrinsic — read a byte from raw memory.
            if name == "peek" {
                let addr = c.args.first().ok_or("peek(addr) needs an address")?;
                return Ok((
                    Expr::Peek(Box::new(lower_expr16(addr, ctx, "peek address")?)),
                    Width::Byte,
                ));
            }
            if name == "inport" {
                let port = c.args.first().ok_or("inport(port) needs a port")?;
                return Ok((
                    Expr::InPort(Box::new(lower_expr16(port, ctx, "inport port")?)),
                    Width::Byte,
                ));
            }
            // `halt(code)` — Cell80: stop the run with a status code (no-op on Spectrum).
            if name == "halt" {
                let code = c.args.first().ok_or("halt(code) needs a code")?;
                return Ok((
                    Expr::Halt(Box::new(lower_expr16(code, ctx, "halt code")?)),
                    Width::Word,
                ));
            }
            if c.args.len() > 3 {
                return Err("more than 3 call arguments not supported yet".into());
            }
            let lowered = c
                .args
                .iter()
                .map(|a| lower_expr(a, ctx))
                .collect::<Result<Vec<_>, String>>()?;

            // A call to a generic function instantiates a specialized copy.
            let is_generic = ctx.mono.borrow().generics.contains_key(&name);
            if is_generic {
                if lowered.iter().any(|(_, w)| w.is_wide()) {
                    return Err(format!(
                        "u32/f32 arguments to a generic (`{name}`) are not supported — \
                         type args erase to 16-bit; use a plain `fn` with a `u32` \
                         first parameter"
                    ));
                }
                let args: Vec<Expr> = lowered.iter().map(|(e, _)| e.clone()).collect();
                let (gargs, ret_w) = resolve_generic(&name, &turbofish, &lowered, ctx)?;
                let inst = ctx.mono.borrow_mut().request(&name, gargs);
                return Ok((Expr::Call(inst, args), ret_w));
            }
            if !turbofish.is_empty() {
                return Err(format!("`{name}` is not a generic function"));
            }
            // The four conversion kernels are *typed builtins* — intercepted before
            // the signature lookup so they are f32-typed even when the cell prelude's
            // text (whose signatures are bits-level u32) is present. int↔f32 crossings
            // exist ONLY through these; `as` casts stay rejected.
            if let "int_to_f32" | "q16_to_f32" | "f32_to_int_trunc" | "f32_to_q16" = name.as_str() {
                if lowered.len() != 1 {
                    return Err(format!("`{name}` takes exactly one argument"));
                }
                let (e, w) = lowered.into_iter().next().unwrap();
                let to_f32 = name.starts_with("int_") || name.starts_with("q16_");
                if to_f32 {
                    if w == Width::F32 {
                        return Err(format!(
                            "`{name}` takes an integer (u32/u16) — this value is \
                             already f32"
                        ));
                    }
                    ctx.mark_f32(match name.as_str() {
                        "int_to_f32" => "int_to_f32",
                        _ => "q16_to_f32",
                    });
                    return Ok((Expr::Call(name, vec![coerce32(e, w)]), Width::F32));
                }
                if w != Width::F32 {
                    return Err(format!(
                        "`{name}` takes an f32 — this value is an integer; it is \
                         already in integer representation"
                    ));
                }
                ctx.mark_f32(match name.as_str() {
                    "f32_to_int_trunc" => "f32_to_int_trunc",
                    _ => "f32_to_q16",
                });
                return Ok((Expr::Call(name, vec![e]), Width::DWord));
            }
            // The explicit bit reinterprets (Rust's `f32::from_bits`/`to_bits`
            // shape): **zero-cost** — the value IS the bits, only the
            // representation tag changes, no kernel runs. The one legal int↔f32
            // crossing besides the F1 value-conversion kernels, and it is loud by
            // name. Exists for the `u32[N]`-array-of-f32-bits envelope (the
            // dialect has no `[f32; N]` fields): a cell walks the array and
            // reinterprets each element explicitly.
            if let "f32_from_bits" | "f32_to_bits" = name.as_str() {
                if lowered.len() != 1 {
                    return Err(format!("`{name}` takes exactly one argument"));
                }
                let (e, w) = lowered.into_iter().next().unwrap();
                if name == "f32_from_bits" {
                    if w == Width::F32 {
                        return Err("`f32_from_bits` takes raw u32 bits — this value is \
                             already f32"
                            .into());
                    }
                    return Ok((coerce32(e, w), Width::F32));
                }
                if w != Width::F32 {
                    return Err("`f32_to_bits` takes an f32 — this value is already an \
                         integer bit pattern"
                        .into());
                }
                return Ok((e, Width::DWord));
            }
            // A known plain fn: the call boundary is typed (docs 10 §Calls) — a wide
            // first slot takes (or widens to) a u32; a wide value in a 16-bit slot
            // stays an error; the return width comes from the signature.
            if let Some(sig) = ctx.fn_sigs.get(&name) {
                if lowered.len() != sig.args.len() {
                    return Err(format!(
                        "`{name}` takes {} argument(s), got {}",
                        sig.args.len(),
                        lowered.len()
                    ));
                }
                // A two-wide call reorders evaluation: the first u32 goes *last*
                // (the stack shape, docs 10 §Calls) while the rest keep their order.
                // The only observable reordering is the first arg against the others,
                // so it is sound unless the first arg *and* some later arg both carry
                // effects — then at most one may, and the caller hoists the rest.
                if sig.args.iter().filter(|w| w.is_wide()).count() >= 2
                    && has_effects(&lowered[0].0)
                    && lowered[1..].iter().any(|(e, _)| has_effects(e))
                {
                    return Err(format!(
                        "arguments to `{name}` (two u32 parameters) reorder evaluation — \
                         the first argument is computed last, so it and another argument \
                         cannot both have side effects; hoist a call to a `let` binding"
                    ));
                }
                let mut args = Vec::with_capacity(lowered.len());
                for (i, ((e, w), sw)) in lowered.into_iter().zip(&sig.args).enumerate() {
                    if sw.is_wide() {
                        // The slot is wide; the *representation* must also agree —
                        // f32 bits never silently pose as u32 or vice versa.
                        if *sw == Width::F32 && w != Width::F32 {
                            return Err(format!(
                                "argument {} of `{name}` is f32 — this value is {} \
                                 (conversions are explicit; the F1 kernels)",
                                i + 1,
                                if w == Width::DWord { "u32" } else { "16-bit" }
                            ));
                        }
                        if *sw == Width::DWord && w == Width::F32 {
                            return Err(format!(
                                "argument {} of `{name}` is u32 — this value is f32 \
                                 (conversions are explicit; the F1 kernels)",
                                i + 1
                            ));
                        }
                        if sw.is_int_wide() && w.is_int_wide() && *sw != w {
                            return Err(format!(
                                "argument {} of `{name}` mixes i32 and u32 — cast \
                                 explicitly (`as i32` / `as u32`)",
                                i + 1
                            ));
                        }
                        args.push(coerce32s(e, w, *sw == Width::SDWord));
                    } else {
                        if w.is_wide() {
                            return Err(format!(
                                "argument {} of `{name}` is 16-bit — narrow with `as u16` \
                                 (only *leading* wide parameters ride HL:DE/stack)",
                                i + 1
                            ));
                        }
                        args.push(e);
                    }
                }
                let ret_w = if sig.ret.is_wide() {
                    sig.ret
                } else {
                    Width::Word
                };
                return Ok((Expr::Call(name, args), ret_w));
            }
            // An unknown callee (a prelude route, an appended kernel): 16-bit only.
            if lowered.iter().any(|(_, w)| w.is_wide()) {
                return Err(format!(
                    "u32/f32 call arguments are not supported for `{name}` (unknown \
                     signature — args pass in 16-bit registers); narrow with `as u16`"
                ));
            }
            let args: Vec<Expr> = lowered.iter().map(|(e, _)| e.clone()).collect();
            Ok((Expr::Call(name, args), Width::Word))
        }
        other => Err(format!(
            "unsupported expression: {} — the dialect accepts integer/bool arithmetic, \
             comparisons, calls, indexing, field access, casts, and `if`/`match` values; \
             restructure around those",
            describe_expr(other)
        )),
    }
}

/// Does this IR expression have (or could it have) side effects — so it must not be
/// duplicated? Calls may mutate state; `inport` reads a device; `halt` stops the run.
pub(crate) fn has_effects(e: &Expr) -> bool {
    match e {
        Expr::Call(..) | Expr::InPort(_) | Expr::Halt(_) => true,
        Expr::Lit(_) | Expr::Var(_) | Expr::AddrOf(_) | Expr::ConstAddr(_) => false,
        Expr::Lit32(_) | Expr::Var32(_) => false,
        Expr::Bin(_, a, b, _) | Expr::Bin32(_, a, b, _) => has_effects(a) || has_effects(b),
        Expr::Cmp { lhs, rhs, .. }
        | Expr::Logic { lhs, rhs, .. }
        | Expr::Cmp32 { lhs, rhs, .. } => has_effects(lhs) || has_effects(rhs),
        Expr::Index(_, i, _) => has_effects(i),
        Expr::Trunc(x)
        | Expr::Trunc32(x)
        | Expr::Widen(x)
        | Expr::SignExtend(x)
        | Expr::Peek(x) => has_effects(x),
        Expr::Deref(p, _) | Expr::Deref32(p, _) => has_effects(p),
        Expr::PtrIndex { ptr, index, .. } => has_effects(ptr) || has_effects(index),
        Expr::MulConst(x, _) => has_effects(x),
        Expr::LoadAt(a, _) => has_effects(a),
        Expr::ShiftVar { e, amount, .. } => has_effects(e) || has_effects(amount),
        Expr::Shift32 { e, .. } => has_effects(e),
    }
}

/// Route `<handle>.<method>(args)` to the configured prelude function (the receiver
/// is dropped — see [`super::PreludeConfig`]).
fn lower_prelude_call(
    handle: &str,
    method: &str,
    args: &syn::punctuated::Punctuated<syn::Expr, syn::token::Comma>,
    ctx: &mut Ctx,
) -> Result<(Expr, Width), String> {
    let name = ctx
        .prelude
        .lookup(handle, method)
        .ok_or_else(|| format!("prelude method {handle}::{method} is not routed"))?
        .to_string();
    let lowered = args
        .iter()
        .map(|a| lower_expr16(a, ctx, "prelude-call argument"))
        .collect::<Result<_, String>>()?;
    Ok((Expr::Call(name, lowered), Width::Word))
}

pub(crate) fn path_ident(expr: &syn::Expr) -> Result<String, String> {
    match expr {
        syn::Expr::Path(p) => p
            .path
            .get_ident()
            .map(|i| i.to_string())
            .ok_or_else(|| "expected a simple variable".into()),
        other => Err(format!(
            "expected a plain variable name here, got {} — bind the value with `let` \
             first, then use the name",
            describe_expr(other)
        )),
    }
}

pub(crate) fn path_str(p: &syn::Path) -> Result<String, String> {
    p.get_ident()
        .map(|i| i.to_string())
        .ok_or_else(|| "expected a struct name".into())
}

// ── diagnostics ─────────────────────────────────────────────────────────────────────

/// A human name for a syn expression — user-facing diagnostics never dump the syn
/// Debug tree. Where the dialect has an accepted rewrite, the name says so.
pub(crate) fn describe_expr(e: &syn::Expr) -> &'static str {
    match e {
        syn::Expr::Array(_) => "an array literal",
        syn::Expr::Assign(_) => "an assignment (assignments are statements — end with `;`)",
        syn::Expr::Async(_) | syn::Expr::Await(_) => "async code (not in the dialect)",
        syn::Expr::Binary(_) => "a binary expression",
        syn::Expr::Block(_) => {
            "a block expression (blocks-as-values aren't supported — bind with `let` first)"
        }
        syn::Expr::Break(_) => "`break`",
        syn::Expr::Call(_) => "a function call",
        syn::Expr::Cast(_) => "an `as` cast",
        syn::Expr::Closure(_) => "a closure (not in the dialect — write a named `fn`)",
        syn::Expr::Continue(_) => "`continue`",
        syn::Expr::Field(_) => "a field access",
        syn::Expr::ForLoop(_) => "a `for` loop (loops are statements, not values)",
        syn::Expr::If(_) => "an `if`",
        syn::Expr::Index(_) => "an index expression",
        syn::Expr::Lit(_) => "a literal",
        syn::Expr::Loop(_) => "a `loop` (loops are statements, not values)",
        syn::Expr::Macro(_) => "a macro call (no macros in the dialect)",
        syn::Expr::Match(_) => "a `match`",
        syn::Expr::MethodCall(_) => "a method call",
        syn::Expr::Paren(_) => "a parenthesized expression",
        syn::Expr::Path(_) => "a name/path",
        syn::Expr::Range(_) => "a range (ranges only appear as `for` bounds)",
        syn::Expr::Reference(_) => {
            "a borrow (`&`/`&mut` only exist as method receivers in the dialect)"
        }
        syn::Expr::Repeat(_) => "an array-repeat literal (`[v; N]`)",
        syn::Expr::Return(_) => "a `return`",
        syn::Expr::Struct(_) => "a struct literal",
        syn::Expr::Try(_) => "`?` (no `Result` in the dialect — return sentinel values)",
        syn::Expr::Tuple(_) => "a tuple literal",
        syn::Expr::Unary(_) => "a unary expression",
        syn::Expr::While(_) => "a `while` loop (loops are statements, not values)",
        _ => "this expression",
    }
}

/// A human name for a literal kind (see [`describe_expr`]).
pub(crate) fn describe_lit(l: &syn::Lit) -> &'static str {
    match l {
        syn::Lit::Str(_) | syn::Lit::CStr(_) => "a string literal",
        syn::Lit::ByteStr(_) => "a byte-string literal",
        syn::Lit::Byte(_) => "a byte literal",
        syn::Lit::Char(_) => "a character literal (use a byte literal, `b'a'`)",
        syn::Lit::Int(_) => "an integer literal",
        syn::Lit::Float(_) => "a float literal",
        syn::Lit::Bool(_) => "a bool literal",
        _ => "this literal",
    }
}

/// [`describe_lit`] for a literal appearing as a `match` pattern.
pub(crate) fn describe_lit_kind(l: &syn::Lit) -> &'static str {
    describe_lit(l)
}

/// A human name for a syn statement (see [`describe_expr`]).
pub(crate) fn describe_stmt(s: &syn::Stmt) -> &'static str {
    match s {
        syn::Stmt::Local(_) => "a `let` binding",
        syn::Stmt::Item(_) => "a nested item (declare fns/structs at the top level)",
        syn::Stmt::Expr(e, _) => describe_expr(e),
        syn::Stmt::Macro(_) => "a macro call (no macros in the dialect)",
    }
}
