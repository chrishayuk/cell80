//! Struct-literal initialization: storing a `Struct { … }` literal into a set of
//! consecutive slots. Split out of `stmt.rs` so the per-field logic can be **recursive**
//! — a nested struct field (`pos: Point { x, y }`) stores the sub-struct's own fields at
//! the field's base, to any depth. Every field kind lowers here: scalar, `u32`, tuple,
//! `[u16/u8/u32; N]` arrays, `[Cell; N]` struct-element arrays, and nested structs.

use super::expr::{coerce32, has_effects, lower_expr, lower_expr16};
use super::layout::{field_offset, member_name, struct_slots, FieldDef};
use super::Ctx;
use crate::ir::*;

/// Store a struct literal `lit` (with layout `fields`) into the slots starting at
/// `base`. Each field's value is lowered and assigned at `base + field_offset`; a
/// nested struct field recurses into `store_struct_literal` at its own base.
pub(crate) fn store_struct_literal(
    base: usize,
    fields: &[FieldDef],
    lit: &syn::ExprStruct,
    ctx: &mut Ctx,
    body: &mut Vec<Stmt>,
) -> Result<(), String> {
    for fv in &lit.fields {
        let fname = member_name(&fv.member)?;
        let off = field_offset(fields, &fname)?;
        let fd = fields.iter().find(|f| f.name == fname);
        let slots = fd.map_or(1, |f| f.slots);
        // A `[Cell; N]` struct-element field — `[Cell { … }; N]` or `[c0, c1, …]`;
        // store each element's sub-fields.
        if let Some(es) = fd.and_then(|f| f.elem_struct.clone()) {
            let efields = ctx
                .struct_fields(&es)
                .ok_or_else(|| format!("unknown struct {es}"))?;
            let esize = struct_slots(&efields).max(1);
            let elems: Vec<&syn::Expr> = match &fv.expr {
                syn::Expr::Repeat(r) => vec![&*r.expr; slots / esize],
                syn::Expr::Array(arr) => arr.elems.iter().collect(),
                _ => {
                    return Err(format!(
                        "array field `{fname}` must be `[{es} {{ … }}; N]` or `[…]`"
                    ))
                }
            };
            for (e, ev) in elems.iter().enumerate() {
                let syn::Expr::Struct(slit) = ev else {
                    return Err(format!("element of `{fname}` must be a `{es}` literal"));
                };
                for fv2 in &slit.fields {
                    let foff = field_offset(&efields, &member_name(&fv2.member)?)?;
                    let v = lower_expr(&fv2.expr, ctx)?.0;
                    body.push(Stmt::Assign(base + off + e * esize + foff, v));
                }
            }
            continue;
        }
        // A **nested struct** field (`pos: Point { x, y }`) — store the sub-struct's
        // fields at this field's base. Recurses to any depth.
        if let Some(sub) = fd.and_then(|f| f.struct_ty.clone()) {
            let syn::Expr::Struct(slit) = &fv.expr else {
                return Err(format!(
                    "struct field `{fname}` must be a `{sub}` literal (`{sub} {{ … }}`)"
                ));
            };
            let sfields = ctx
                .struct_fields(&sub)
                .ok_or_else(|| format!("unknown struct {sub}"))?;
            store_struct_literal(base + off, &sfields, slit, ctx, body)?;
            continue;
        }
        let packed_len = fd.and_then(|f| f.packed_len);
        let wide_len = fd.and_then(|f| f.wide_len);
        match &fv.expr {
            // A `[u32; N]` field initialised `[v; N]`: per-element wide
            // stores (two slots each).
            syn::Expr::Repeat(r) if wide_len.is_some() => {
                let n = wide_len.unwrap();
                let (v, vw) = lower_expr(&r.expr, ctx)?;
                let v = coerce32(v, vw);
                if has_effects(&v) {
                    return Err(format!(
                        "field `{fname}`: a `[v; N]` u32 initialiser needs a \
                         simple value — bind it first: `let v = …;`"
                    ));
                }
                for i in 0..n {
                    body.push(Stmt::Assign32(base + off + 2 * i, v.clone()));
                }
            }
            // A `[u32; N]` field initialised `[e0, e1, …]`.
            syn::Expr::Array(arr) if wide_len.is_some() => {
                let n = wide_len.unwrap();
                if arr.elems.len() != n {
                    return Err(format!("array field `{fname}` expects {n} values"));
                }
                for (i, e) in arr.elems.iter().enumerate() {
                    let (v, vw) = lower_expr(e, ctx)?;
                    body.push(Stmt::Assign32(base + off + 2 * i, coerce32(v, vw)));
                }
            }
            // A tuple field is initialised by a tuple literal — one value per slot.
            syn::Expr::Tuple(t) => {
                if t.elems.len() != slots {
                    return Err(format!("tuple field `{fname}` expects {slots} values"));
                }
                for (i, e) in t.elems.iter().enumerate() {
                    let v = lower_expr16(e, ctx, "struct field (u16 slots)")?;
                    body.push(Stmt::Assign(base + off + i, v));
                }
            }
            // A byte-packed `[u8; N]` field initialised `[v; N]`: every slot
            // holds the byte doubled (`v | v << 8`), one `Fill` (an odd tail
            // byte lands in the field's own padding — harmless).
            syn::Expr::Repeat(r) if packed_len.is_some() => {
                let v = lower_expr16(&r.expr, ctx, "byte array element")?;
                let lo = Expr::Bin(
                    BinOp::And,
                    Box::new(v.clone()),
                    Box::new(Expr::Lit(0xFF)),
                    Width::Word,
                );
                let both = Expr::Bin(
                    BinOp::Or,
                    Box::new(lo.clone()),
                    Box::new(Expr::Bin(
                        BinOp::Shl,
                        Box::new(lo),
                        Box::new(Expr::Lit(8)),
                        Width::Word,
                    )),
                    Width::Word,
                );
                body.push(Stmt::Fill {
                    base: base + off,
                    count: slots,
                    value: both,
                });
            }
            // An array field initialised `[v; N]` — fill its `slots` slots.
            syn::Expr::Repeat(r) => {
                for i in 0..slots {
                    let v = lower_expr16(&r.expr, ctx, "struct field (u16 slots)")?;
                    body.push(Stmt::Assign(base + off + i, v));
                }
            }
            // A byte-packed `[u8; N]` field initialised `[e0, e1, …]`: two
            // bytes per slot, little-endian (`e0 | e1 << 8`).
            syn::Expr::Array(arr) if packed_len.is_some() => {
                let n = packed_len.unwrap();
                if arr.elems.len() != n {
                    return Err(format!("array field `{fname}` expects {n} values"));
                }
                let byte = |e: &syn::Expr, ctx: &mut Ctx| -> Result<Expr, String> {
                    Ok(Expr::Bin(
                        BinOp::And,
                        Box::new(lower_expr16(e, ctx, "byte array element")?),
                        Box::new(Expr::Lit(0xFF)),
                        Width::Word,
                    ))
                };
                for (slot, pair) in arr.elems.iter().collect::<Vec<_>>().chunks(2).enumerate() {
                    let lo = byte(pair[0], ctx)?;
                    let v = match pair.get(1) {
                        Some(hi) => Expr::Bin(
                            BinOp::Or,
                            Box::new(lo),
                            Box::new(Expr::Bin(
                                BinOp::Shl,
                                Box::new(byte(hi, ctx)?),
                                Box::new(Expr::Lit(8)),
                                Width::Word,
                            )),
                            Width::Word,
                        ),
                        None => lo,
                    };
                    body.push(Stmt::Assign(base + off + slot, v));
                }
            }
            // An array field initialised `[e0, e1, …]`.
            syn::Expr::Array(arr) => {
                if arr.elems.len() != slots {
                    return Err(format!("array field `{fname}` expects {slots} values"));
                }
                for (i, e) in arr.elems.iter().enumerate() {
                    let v = lower_expr16(e, ctx, "struct field (u16 slots)")?;
                    body.push(Stmt::Assign(base + off + i, v));
                }
            }
            // A `u32` field initialises wide (two slots, one `Assign32`).
            _ if fd.is_some_and(|d| d.width == Width::DWord) => {
                let (v, vw) = lower_expr(&fv.expr, ctx)?;
                body.push(Stmt::Assign32(base + off, coerce32(v, vw)));
            }
            _ if slots == 1 => {
                let v = lower_expr16(&fv.expr, ctx, "struct field (u16 slots)")?;
                body.push(Stmt::Assign(base + off, v));
            }
            _ => return Err(format!("field `{fname}` expects {slots} values")),
        }
    }
    Ok(())
}
