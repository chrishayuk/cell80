//! Expression lowering: a `syn::Expr` → IR [`Expr`] plus its inferred [`Width`].
//! Also the field/index access helpers (constant slot for a by-value struct, an
//! indirect load/store through the pointer for `self`-style receivers) and method
//! calls (`wrapping_*`, prelude-handle routing, or `obj.m(a) → Type::m(&obj, a)`).

use super::generics::{call_target, resolve_generic};
use super::layout::{field_offset, member_name, resolve_enum_path, struct_slots};
use super::Ctx;
use crate::ir::*;

/// Lower an expression that must fit a 16-bit context (a slot store, a call register,
/// a comparison operand, an index): a `u32` value is a clean lowering error — never a
/// codegen panic. `what` names the context in the message.
pub(crate) fn lower_expr16(expr: &syn::Expr, ctx: &mut Ctx, what: &str) -> Result<Expr, String> {
    let (e, w) = lower_expr(expr, ctx)?;
    if w == Width::DWord {
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
    if w == Width::DWord {
        e
    } else if let Expr::Lit(k) = e {
        Expr::Lit32(k as u32)
    } else {
        Expr::Widen(Box::new(e))
    }
}

/// The byte address of `a[i].field` for a local struct-element array `[Cell; N]`:
/// `&a + index*(elem_stride) + field_offset` (all in bytes). Errs if `a` isn't a
/// struct-element array.
pub(crate) fn elem_field_addr(
    ix: &syn::ExprIndex,
    member: &syn::Member,
    ctx: &mut Ctx,
) -> Result<Expr, String> {
    let (base_addr, elem_struct) = array_base(&ix.expr, ctx)?;
    let efields = ctx
        .struct_fields(&elem_struct)
        .ok_or_else(|| format!("unknown struct {elem_struct}"))?;
    let fname = member_name(member)?;
    if let Some(f) = efields.iter().find(|f| f.name == fname) {
        if f.width == Width::DWord {
            return Err(format!(
                "u32 field `{fname}` of a struct-array element is not supported yet"
            ));
        }
    }
    let foff = field_offset(&efields, &fname)?;
    let stride = (struct_slots(&efields) * 2) as u16;
    let idx = lower_expr16(&ix.index, ctx, "array index")?;
    // base + index*stride (+ field_offset)
    let elem = Expr::Bin(
        BinOp::Add,
        Box::new(base_addr),
        Box::new(Expr::MulConst(Box::new(idx), stride)),
        Width::Word,
    );
    Ok(if foff == 0 {
        elem
    } else {
        Expr::Bin(
            BinOp::Add,
            Box::new(elem),
            Box::new(Expr::Lit((foff * 2) as u16)),
            Width::Word,
        )
    })
}

/// Lower an expression, returning its IR and inferred width (`u8`/`u16`).
pub(crate) fn lower_expr(expr: &syn::Expr, ctx: &mut Ctx) -> Result<(Expr, Width), String> {
    match expr {
        syn::Expr::Lit(l) => match &l.lit {
            syn::Lit::Int(i) => {
                if i.suffix() == "u32" {
                    return Ok((
                        Expr::Lit32(i.base10_parse::<u32>().map_err(|e| e.to_string())?),
                        Width::DWord,
                    ));
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
                let base = ctx.vars.base(&name);
                match ctx.vars.ty(&name) {
                    Width::DWord => Ok((Expr::Var32(base), Width::DWord)),
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
                              (`-5i16`) or cast first (`x as i16`)"
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
            if ew == Width::DWord {
                return Ok(match tw {
                    Width::Byte => (
                        Expr::Trunc(Box::new(Expr::Trunc32(Box::new(e)))),
                        Width::Byte,
                    ),
                    Width::Word | Width::SWord => (Expr::Trunc32(Box::new(e)), tw),
                    Width::DWord => (e, Width::DWord),
                });
            }
            if tw == Width::DWord {
                if ew == Width::SWord {
                    return Err("`i16 as u32` sign-extends in Rust, which the dialect \
                                doesn't do — take the bits explicitly (`x as u16 as u32`)"
                        .into());
                }
                // Widen a 16-bit value up to `u32` (zero-extend), so a `u16` can feed a wide
                // intermediate (e.g. `part as u32 * 100`). `Byte`/`Word` widen identically —
                // the value is held in `HL` and the high word is zeroed.
                return Ok((Expr::Widen(Box::new(e)), Width::DWord));
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
            if lowered.iter().any(|(_, w)| *w == Width::DWord) {
                return Err(format!(
                    "u32 call arguments are not supported yet (`{name}` — args pass in 16-bit \
                     registers); narrow with `as u16`"
                ));
            }
            let args: Vec<Expr> = lowered.iter().map(|(e, _)| e.clone()).collect();

            // A call to a generic function instantiates a specialized copy.
            let is_generic = ctx.mono.borrow().generics.contains_key(&name);
            if is_generic {
                let (gargs, ret_w) = resolve_generic(&name, &turbofish, &lowered, ctx)?;
                let inst = ctx.mono.borrow_mut().request(&name, gargs);
                return Ok((Expr::Call(inst, args), ret_w));
            }
            if !turbofish.is_empty() {
                return Err(format!("`{name}` is not a generic function"));
            }
            Ok((Expr::Call(name, args), Width::Word)) // non-generic calls assume u16 returns
        }
        other => Err(format!(
            "unsupported expression: {} — the dialect accepts integer/bool arithmetic, \
             comparisons, calls, indexing, field access, casts, and `if`/`match` values; \
             restructure around those",
            describe_expr(other)
        )),
    }
}

fn bin_op(op: &syn::BinOp) -> Result<BinOp, String> {
    Ok(match op {
        syn::BinOp::Add(_) => BinOp::Add,
        syn::BinOp::Sub(_) => BinOp::Sub,
        syn::BinOp::Mul(_) => BinOp::Mul,
        syn::BinOp::Div(_) => BinOp::Div,
        syn::BinOp::Rem(_) => BinOp::Rem,
        syn::BinOp::BitOr(_) => BinOp::Or,
        syn::BinOp::BitAnd(_) => BinOp::And,
        syn::BinOp::BitXor(_) => BinOp::Xor,
        syn::BinOp::Shl(_) => BinOp::Shl,
        syn::BinOp::Shr(_) => BinOp::Shr,
        _ => {
            return Err(
                "unsupported operator — the dialect has `+ - * / % & | ^ << >>`, \
                 comparisons, and `&&`/`||` (no `..`, no overloading)"
                    .into(),
            )
        }
    })
}

/// The comparison op for a `syn` binop, if it is one (the value form of `<`/`<=`/…).
fn cmp_op(op: &syn::BinOp) -> Option<Cmp> {
    Some(match op {
        syn::BinOp::Lt(_) => Cmp::Lt,
        syn::BinOp::Le(_) => Cmp::Le,
        syn::BinOp::Gt(_) => Cmp::Gt,
        syn::BinOp::Ge(_) => Cmp::Ge,
        syn::BinOp::Eq(_) => Cmp::Eq,
        syn::BinOp::Ne(_) => Cmp::Ne,
        _ => return None,
    })
}

/// `&&` → `Some(true)`, `||` → `Some(false)`, else `None`.
fn logic_op(op: &syn::BinOp) -> Option<bool> {
    match op {
        syn::BinOp::And(_) => Some(true),
        syn::BinOp::Or(_) => Some(false),
        _ => None,
    }
}

/// Whether `e` is an integer literal (a constant shift amount uses the unrolled path).
fn is_int_literal(e: &syn::Expr) -> bool {
    matches!(e, syn::Expr::Lit(l) if matches!(l.lit, syn::Lit::Int(_)))
}

/// A shift amount — must be an integer literal (variable shifts are unsupported).
fn lit_shift_amount(e: &syn::Expr) -> Result<u8, String> {
    if let syn::Expr::Lit(l) = e {
        if let syn::Lit::Int(i) = &l.lit {
            return i.base10_parse::<u8>().map_err(|e| e.to_string());
        }
    }
    Err("shift amount must be an integer literal".into())
}

/// Lower a binary expression. Shifts take a constant RHS for a `u32` LHS (16-bit
/// shifts also take a runtime amount); a `u32` operand makes the op 32-bit (`Bin32`),
/// zero-extending a mixed 16-bit side. `u32` comparisons are not supported yet.
fn lower_binary(b: &syn::ExprBinary, ctx: &mut Ctx) -> Result<(Expr, Width), String> {
    // A comparison used as a value (`(a < b) as u16`, `let f = a == b;`) materialises to
    // a `0`/`1` bool. In condition position a comparison stays a tight `Cond` (handled by
    // `lower_cond`), so this only fires when a comparison is a real value.
    if let Some(cmp) = cmp_op(&b.op) {
        let (le, lw) = lower_expr(&b.left, ctx)?;
        let (re, rw) = lower_expr(&b.right, ctx)?;
        if lw == Width::DWord || rw == Width::DWord {
            return Err(
                "u32 comparisons are not supported yet — compare the words (`as u16`, `>> 16`)"
                    .into(),
            );
        }
        let signed = lw == Width::SWord || rw == Width::SWord;
        let (lhs, rhs) = (Box::new(le), Box::new(re));
        return Ok((
            Expr::Cmp {
                cmp,
                lhs,
                rhs,
                signed,
            },
            Width::Byte,
        ));
    }
    // Short-circuit `&&` / `||` on bool operands → a `0`/`1` value.
    if let Some(and) = logic_op(&b.op) {
        let lhs = Box::new(lower_expr16(&b.left, ctx, "`&&`/`||` operand")?);
        let rhs = Box::new(lower_expr16(&b.right, ctx, "`&&`/`||` operand")?);
        return Ok((Expr::Logic { and, lhs, rhs }, Width::Byte));
    }
    let op = bin_op(&b.op)?;
    if matches!(op, BinOp::Shl | BinOp::Shr) {
        let (le, lw) = lower_expr(&b.left, ctx)?;
        // A runtime (non-literal) 16-bit shift amount → a counted shift loop. `u32`
        // shifts and literal amounts keep the unrolled constant path below.
        if lw != Width::DWord && !is_int_literal(&b.right) {
            let (ae, aw) = lower_expr(&b.right, ctx)?;
            // A `u32` amount is fine in rustc (`x << y32`) — only its low byte counts.
            let amount = Box::new(if aw == Width::DWord {
                Expr::Trunc32(Box::new(ae))
            } else {
                ae
            });
            return Ok((
                Expr::ShiftVar {
                    left: matches!(op, BinOp::Shl),
                    e: Box::new(le),
                    amount,
                    w: lw,
                },
                lw,
            ));
        }
        let k = lit_shift_amount(&b.right)?;
        if lw == Width::DWord {
            return Ok((
                Expr::Shift32 {
                    left: matches!(op, BinOp::Shl),
                    e: Box::new(le),
                    k,
                },
                Width::DWord,
            ));
        }
        return Ok((
            Expr::Bin(op, Box::new(le), Box::new(Expr::Lit(k as u16)), lw),
            lw,
        ));
    }
    let (le, lw) = lower_expr(&b.left, ctx)?;
    let (re, rw) = lower_expr(&b.right, ctx)?;
    if lw == Width::DWord || rw == Width::DWord {
        // Full 32-bit arithmetic: `+ - * / %` and `| & ^`. A 16-bit side zero-extends
        // (the unsuffixed-literal mixing rustc allows, `part as u32 * 100`).
        return Ok((
            Expr::Bin32(op, Box::new(coerce32(le, lw)), Box::new(coerce32(re, rw))),
            Width::DWord,
        ));
    }
    Ok((Expr::Bin(op, Box::new(le), Box::new(re), lw), lw))
}

/// What a field access resolves to: the receiver's base slot, the field's slot offset,
/// whether the receiver is a pointer (`self`) or a by-value local, the field's slot
/// count and value width (`DWord` = a two-slot `u32`), and (for a `[Cell; N]` field)
/// its element struct.
struct FieldRef {
    base: usize,
    off: usize,
    is_ptr: bool,
    slots: usize,
    width: Width,
    elem_struct: Option<String>,
}

/// Resolve `obj.field` (and a tuple element of a struct field, `obj.field.N`).
fn field_target(f: &syn::ExprField, ctx: &mut Ctx) -> Result<FieldRef, String> {
    // `obj.field.N` — a tuple element (one slot) at the field's offset + N.
    if let syn::Expr::Field(inner) = &*f.base {
        let syn::Member::Unnamed(idx) = &f.member else {
            return Err("nested struct fields are not supported".into());
        };
        let r = field_target(inner, ctx)?;
        return Ok(FieldRef {
            off: r.off + idx.index as usize,
            slots: 1,
            width: Width::Word,
            elem_struct: None,
            ..r
        });
    }
    let obj = path_ident(&f.base)?;
    let (base, sname, is_ptr) = ctx
        .vars
        .receiver(&obj)
        .ok_or_else(|| format!("{obj} is not a struct"))?;
    let fields = ctx
        .struct_fields(&sname)
        .ok_or_else(|| format!("unknown struct {sname}"))?;
    let name = member_name(&f.member)?;
    let off = field_offset(&fields, &name)?;
    let fd = fields.iter().find(|d| d.name == name);
    Ok(FieldRef {
        base,
        off,
        is_ptr,
        slots: fd.map_or(1, |d| d.slots),
        width: fd.map_or(Width::Word, |d| d.width),
        elem_struct: fd.and_then(|d| d.elem_struct.clone()),
    })
}

/// Scale an element index by a byte stride (stride 1 passes through; powers of two
/// shift via [`Expr::MulConst`]).
fn scaled(idx: Expr, stride: u16) -> Expr {
    if stride == 1 {
        idx
    } else {
        Expr::MulConst(Box::new(idx), stride)
    }
}

/// Lower `&CONST` or `&CONST[i]` to the (symbolic) address of packed const data.
fn lower_const_ref(referent: &syn::Expr, ctx: &mut Ctx) -> Result<(Expr, Width), String> {
    match referent {
        syn::Expr::Path(_) => {
            let name = path_ident(referent)?;
            let consts = ctx.consts.borrow();
            let Some(d) = consts.get(&name) else {
                return Err(format!(
                    "`&{name}` — the dialect borrows only const data (`&CONST`, \
                     `&CONST[i]`); `{name}` is not a data const"
                ));
            };
            if d.is_ref {
                return Err(format!(
                    "`&{name}` is a reference to a reference — a `&str`/`&[u8; N]` \
                     const is already an address; pass `{name}` directly"
                ));
            }
            Ok((Expr::ConstAddr(name), Width::Word))
        }
        syn::Expr::Index(ix) => {
            let name = path_ident(&ix.expr)?;
            let (stride, len) = {
                let consts = ctx.consts.borrow();
                let Some(d) = consts.get(&name) else {
                    return Err(format!(
                        "`&{name}[…]` — `{name}` is not a data const (the dialect \
                         borrows only const data)"
                    ));
                };
                if d.stride == 0 {
                    return Err(format!("`{name}` is not an array const"));
                }
                (d.stride, d.len)
            };
            // A literal index is bounds-checked here; a runtime index is the
            // program's responsibility (same as local array indexing).
            if let syn::Expr::Lit(l) = &*ix.index {
                if let syn::Lit::Int(i) = &l.lit {
                    let v = i.base10_parse::<u16>().map_err(|e| e.to_string())?;
                    if v >= len {
                        return Err(format!("`&{name}[{v}]` is out of bounds (length {len})"));
                    }
                }
            }
            let idx = lower_expr16(&ix.index, ctx, "const element index")?;
            Ok((
                Expr::Bin(
                    BinOp::Add,
                    Box::new(Expr::ConstAddr(name)),
                    Box::new(scaled(idx, stride)),
                    Width::Word,
                ),
                Width::Word,
            ))
        }
        other => Err(format!(
            "cannot borrow {} — the dialect borrows only const data (`&CONST`, \
             `&CONST[i]`); `&`/`&mut` locals exist only as method receivers",
            describe_expr(other)
        )),
    }
}

/// The byte base address of an indexable array + its element struct — for a local array
/// var (`a`) or a struct field that is an array of structs (`self.cells`).
pub(crate) fn array_base(arr: &syn::Expr, ctx: &mut Ctx) -> Result<(Expr, String), String> {
    match arr {
        syn::Expr::Path(_) => {
            let name = path_ident(arr)?;
            let elem = ctx
                .vars
                .elem_struct(&name)
                .ok_or_else(|| format!("`{name}` is not a struct-element array"))?;
            Ok((Expr::AddrOf(ctx.vars.base(&name)), elem))
        }
        syn::Expr::Field(ff) => {
            let r = field_target(ff, ctx)?;
            let elem = r
                .elem_struct
                .ok_or("that field is not a struct-element array")?;
            // The field's first byte: `self_ptr + off*2`, or the by-value slot address.
            let base = if r.is_ptr {
                Expr::Bin(
                    BinOp::Add,
                    Box::new(Expr::Var(r.base)),
                    Box::new(Expr::Lit((r.off * 2) as u16)),
                    Width::Word,
                )
            } else {
                Expr::AddrOf(r.base + r.off)
            };
            Ok((base, elem))
        }
        other => Err(format!(
            "cannot index {} — index a named local array (`arr[i]`) or an array field \
             (`self.arr[i]`)",
            describe_expr(other)
        )),
    }
}

/// Lower an index read `base[idx]`: a local array (`arr[i]`) or an array *field*
/// reached through a struct receiver (`self.arr[i]`).
fn lower_index_read(ix: &syn::ExprIndex, ctx: &mut Ctx) -> Result<(Expr, Width), String> {
    // `s.as_bytes()[i]` on a `&str` param — a byte load at `s + 2 + i` (past the
    // u16 length prefix). The one accepted indexed-method shape.
    if let syn::Expr::MethodCall(mc) = &*ix.expr {
        if mc.method == "as_bytes" {
            if let Ok(name) = path_ident(&mc.receiver) {
                if ctx.vars.str_param(&name) {
                    let base = ctx.vars.base(&name);
                    let idx = lower_expr16(&ix.index, ctx, "byte index")?;
                    let addr = Expr::Bin(
                        BinOp::Add,
                        Box::new(Expr::Var(base)),
                        Box::new(Expr::Bin(
                            BinOp::Add,
                            Box::new(idx),
                            Box::new(Expr::Lit(2)),
                            Width::Word,
                        )),
                        Width::Word,
                    );
                    return Ok((Expr::LoadAt(Box::new(addr), Width::Byte), Width::Byte));
                }
            }
        }
    }
    if let syn::Expr::Field(f) = &*ix.expr {
        let r = field_target(f, ctx)?;
        if r.elem_struct.is_some() {
            return Err(
                "a struct-array element isn't a scalar — read a field, e.g. `s.cells[i].x`".into(),
            );
        }
        let idx = lower_expr16(&ix.index, ctx, "array index")?;
        let e = if r.is_ptr {
            // `self.arr[i]` → *(self + off*2 + i*2)
            Expr::PtrIndex {
                ptr: Box::new(Expr::Var(r.base)),
                off: r.off * 2,
                index: Box::new(idx),
            }
        } else {
            // by-value struct local: the array's first slot is `base + off`.
            Expr::Index(r.base + r.off, Box::new(idx), Width::Word)
        };
        return Ok((e, Width::Word));
    }
    let arr = path_ident(&ix.expr)?;
    if ctx.vars.str_param(&arr) {
        return Err(format!(
            "`{arr}[…]` — a `&str` isn't directly indexable (that's real Rust too); \
             read a byte with `{arr}.as_bytes()[i]`"
        ));
    }
    if ctx.vars.elem_struct(&arr).is_some() {
        return Err(format!(
            "a struct-array element isn't a scalar — read a field, e.g. `{arr}[i].x`"
        ));
    }
    // `t[i]` through an element pointer (`t: &[u8; N]` param, `let t = &CONST;`):
    // a load at `t + i*stride` from packed data.
    if let Some((w, stride)) = ctx.vars.elem_ptr(&arr) {
        let base = ctx.vars.base(&arr);
        let idx = lower_expr16(&ix.index, ctx, "array index")?;
        let addr = Expr::Bin(
            BinOp::Add,
            Box::new(Expr::Var(base)),
            Box::new(scaled(idx, stride)),
            Width::Word,
        );
        return Ok((Expr::LoadAt(Box::new(addr), w), w));
    }
    // `CONST[i]` — a load straight out of the const-data section.
    if !ctx.vars.is_declared(&arr) {
        let meta = ctx
            .consts
            .borrow()
            .get(&arr)
            .map(|d| (d.elem_width, d.stride));
        if let Some((elem_width, stride)) = meta {
            let Some(w) = elem_width else {
                return Err(format!(
                    "`{arr}[i]` — this const's elements aren't scalars; take the \
                     element's address instead (`&{arr}[i]`)"
                ));
            };
            let idx = lower_expr16(&ix.index, ctx, "array index")?;
            let addr = Expr::Bin(
                BinOp::Add,
                Box::new(Expr::ConstAddr(arr)),
                Box::new(scaled(idx, stride)),
                Width::Word,
            );
            return Ok((Expr::LoadAt(Box::new(addr), w), w));
        }
    }
    let base = ctx.vars.base(&arr);
    let w = ctx.vars.ty(&arr);
    let idx = lower_expr16(&ix.index, ctx, "array index")?;
    Ok((Expr::Index(base, Box::new(idx), w), w))
}

/// Lower an index store `base[idx] = rhs` (mirror of [`lower_index_read`]).
pub(crate) fn lower_index_store(
    ix: &syn::ExprIndex,
    rhs: &syn::Expr,
    ctx: &mut Ctx,
) -> Result<Stmt, String> {
    if let syn::Expr::Field(f) = &*ix.expr {
        let r = field_target(f, ctx)?;
        let idx = lower_expr16(&ix.index, ctx, "array index")?;
        let val = lower_expr16(rhs, ctx, "array element (u16 slots)")?;
        return Ok(if r.is_ptr {
            Stmt::PtrStoreIndex {
                ptr: Box::new(Expr::Var(r.base)),
                off: r.off * 2,
                index: Box::new(idx),
                value: val,
            }
        } else {
            Stmt::StoreIndex(r.base + r.off, idx, val, Width::Word)
        });
    }
    if let syn::Expr::MethodCall(mc) = &*ix.expr {
        if mc.method == "as_bytes" {
            if let Ok(name) = path_ident(&mc.receiver) {
                if ctx.vars.str_param(&name) {
                    return Err(format!(
                        "cannot assign through `{name}.as_bytes()` — a `&str` is \
                         read-only; build output in a `[u8; N]` field instead"
                    ));
                }
            }
        }
    }
    let arr = path_ident(&ix.expr)?;
    if ctx.vars.str_param(&arr) {
        return Err(format!(
            "cannot assign through `{arr}` — a `&str` is read-only"
        ));
    }
    if ctx.vars.elem_ptr(&arr).is_some() {
        return Err(format!(
            "cannot assign through `{arr}` — a `&[T; N]` reference is read-only"
        ));
    }
    if !ctx.vars.is_declared(&arr) && ctx.consts.borrow().get(&arr).is_some() {
        return Err(format!("cannot assign to const data `{arr}`"));
    }
    let base = ctx.vars.base(&arr);
    let w = ctx.vars.ty(&arr);
    let idx = lower_expr16(&ix.index, ctx, "array index")?;
    let val = lower_expr16(rhs, ctx, "array element (u16 slots)")?;
    Ok(Stmt::StoreIndex(base, idx, val, w))
}

/// Read `obj.field` — a constant slot for a by-value struct, an indirect load
/// through the pointer for `self`-style receivers. A `u32` field reads wide
/// (`Var32` / `Deref32`), so the expression carries `Width::DWord`.
fn lower_field_read(f: &syn::ExprField, ctx: &mut Ctx) -> Result<(Expr, Width), String> {
    // `a[i].field` — a field of a struct-array element at a computed address.
    if let syn::Expr::Index(ix) = &*f.base {
        return Ok((
            Expr::LoadAt(Box::new(elem_field_addr(ix, &f.member, ctx)?), Width::Word),
            Width::Word,
        ));
    }
    let r = field_target(f, ctx)?;
    if r.width == Width::DWord {
        return Ok(if r.is_ptr {
            (
                Expr::Deref32(Box::new(Expr::Var(r.base)), r.off * 2),
                Width::DWord,
            )
        } else {
            (Expr::Var32(r.base + r.off), Width::DWord)
        });
    }
    if r.slots != 1 {
        return Err("this field is not a scalar (read a tuple field by element: `.0`)".into());
    }
    Ok(if r.is_ptr {
        (
            Expr::Deref(Box::new(Expr::Var(r.base)), r.off * 2),
            Width::Word,
        )
    } else {
        (Expr::Var(r.base + r.off), Width::Word)
    })
}

/// Write `obj.field = val` (`vw` is the value's lowered width). A `u32` field stores
/// wide (`Assign32` / `Store32`), zero-extending a 16-bit value; a `u32` value into a
/// 16-bit field is an error.
pub(crate) fn lower_field_store(
    f: &syn::ExprField,
    val: Expr,
    vw: Width,
    ctx: &mut Ctx,
) -> Result<Stmt, String> {
    // `a[i].field = v` — store a field of a struct-array element at a computed address.
    if let syn::Expr::Index(ix) = &*f.base {
        if vw == Width::DWord {
            return Err(
                "u32 value in a 16-bit context (struct-array element field) — narrow with \
                 `as u16`"
                    .into(),
            );
        }
        return Ok(Stmt::StoreAt(
            elem_field_addr(ix, &f.member, ctx)?,
            val,
            Width::Word,
        ));
    }
    let r = field_target(f, ctx)?;
    if r.width == Width::DWord {
        let val = coerce32(val, vw);
        return Ok(if r.is_ptr {
            Stmt::Store32(Expr::Var(r.base), r.off * 2, val)
        } else {
            Stmt::Assign32(r.base + r.off, val)
        });
    }
    if vw == Width::DWord {
        return Err("cannot assign a u32 value to a 16-bit field — narrow with `as u16`".into());
    }
    if r.slots != 1 {
        return Err("this field is not a scalar (assign a tuple field by element: `.0`)".into());
    }
    if r.is_ptr {
        Ok(Stmt::Store(Expr::Var(r.base), r.off * 2, val))
    } else {
        Ok(Stmt::Assign(r.base + r.off, val))
    }
}

/// Lower a method call: the `wrapping_*` value ops, or `obj.m(a, b)` →
/// `Type::m(&obj, a, b)` (`self` passed as a leading pointer).
pub(crate) fn lower_method_call(
    m: &syn::ExprMethodCall,
    ctx: &mut Ctx,
) -> Result<(Expr, Width), String> {
    let method = m.method.to_string();
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
        // A `u32` receiver/argument makes it a 32-bit op (all `Bin32` arithmetic is
        // mod-2^32, i.e. wrapping, already).
        if rw == Width::DWord || aw == Width::DWord {
            return Ok((
                Expr::Bin32(op, Box::new(coerce32(recv, rw)), Box::new(coerce32(re, aw))),
                Width::DWord,
            ));
        }
        return Ok((Expr::Bin(op, Box::new(recv), Box::new(re), rw), rw));
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

/// Does this IR expression have (or could it have) side effects — so it must not be
/// duplicated? Calls may mutate state; `inport` reads a device; `halt` stops the run.
fn has_effects(e: &Expr) -> bool {
    match e {
        Expr::Call(..) | Expr::InPort(_) | Expr::Halt(_) => true,
        Expr::Lit(_) | Expr::Var(_) | Expr::AddrOf(_) | Expr::ConstAddr(_) => false,
        Expr::Lit32(_) | Expr::Var32(_) => false,
        Expr::Bin(_, a, b, _) | Expr::Bin32(_, a, b) => has_effects(a) || has_effects(b),
        Expr::Cmp { lhs, rhs, .. } | Expr::Logic { lhs, rhs, .. } => {
            has_effects(lhs) || has_effects(rhs)
        }
        Expr::Index(_, i, _) => has_effects(i),
        Expr::Trunc(x) | Expr::Trunc32(x) | Expr::Widen(x) | Expr::Peek(x) => has_effects(x),
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
