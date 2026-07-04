//! Statement and control-flow lowering: `let` bindings (scalars, arrays, struct
//! literals), assignment, `if`/`while`/`for`/`loop`/`match`, `break`/`continue`/
//! `return`, and the conditions they branch on. `for` desugars to a counted loop and
//! `match` to an if-chain over a scrutinee temp — no codegen support needed for either.

use super::expr::{
    array_base, coerce32, lower_expr, lower_expr16, lower_field_store, lower_index_store,
    lower_method_call, path_ident, path_str,
};
use super::generics::infer_struct_args;
use super::layout::{
    elem_width, field_offset, lit_len, member_name, resolve_enum_path, struct_slots, FieldDef,
};
use super::Ctx;
use crate::ir::*;

/// Resolve a struct literal to its concrete `(name, field layout)` — a regular struct,
/// or a const-/generic struct *instance* whose arguments are inferred from the literal
/// (registering the instance + its methods on first use).
fn resolve_struct_literal(
    lit: &syn::ExprStruct,
    ctx: &mut Ctx,
) -> Result<(String, Vec<FieldDef>), String> {
    let sbase = path_str(&lit.path)?;
    if let Some(f) = ctx.structs.get(&sbase) {
        return Ok((sbase, f.clone()));
    }
    let gs = ctx.mono.borrow().generic_structs.get(&sbase).cloned();
    let Some(gs) = gs else {
        return Err(format!("unknown struct {sbase}"));
    };
    let args = infer_struct_args(&gs, lit, ctx)?;
    let mangled = ctx
        .mono
        .borrow_mut()
        .instantiate_struct(&sbase, args, ctx.structs)?;
    let fields = ctx.mono.borrow().struct_instances[&mangled].clone();
    Ok((mangled, fields))
}

/// An array length — an integer literal, or a const-generic parameter resolved to its
/// instance value (`let a = [0u16; N]` inside `fn f<const N: usize>()`).
fn array_len(e: &syn::Expr, ctx: &Ctx) -> Result<usize, String> {
    if let syn::Expr::Path(p) = e {
        if let Some(id) = p.path.get_ident() {
            if let Some(n) = ctx.const_args.get(&id.to_string()) {
                return Ok(*n as usize);
            }
        }
    }
    lit_len(e)
}

pub(crate) fn lower_local(
    local: &syn::Local,
    ctx: &mut Ctx,
    body: &mut Vec<Stmt>,
) -> Result<(), String> {
    let init = local.init.as_ref().ok_or("`let` needs an initializer")?;
    // `let (a, b) = …` — a tuple destructure (a tuple literal or a multi-value return).
    if let syn::Pat::Tuple(pt) = &local.pat {
        return lower_tuple_let(pt, &init.expr, ctx, body);
    }
    let name = pat_ident(&local.pat)?;
    match &*init.expr {
        // `[Cell { … }; N]` — a struct-element array; each element is `Cell`'s slots.
        syn::Expr::Repeat(r) if matches!(&*r.expr, syn::Expr::Struct(_)) => {
            let syn::Expr::Struct(slit) = &*r.expr else {
                unreachable!()
            };
            let n = array_len(&r.len, ctx)?;
            let elem_name = path_str(&slit.path)?;
            let efields = ctx
                .struct_fields(&elem_name)
                .ok_or_else(|| format!("unknown struct {elem_name}"))?;
            if efields.iter().any(|f| f.width == Width::DWord) {
                return Err(format!(
                    "struct-array elements with u32 fields are not supported yet ({elem_name})"
                ));
            }
            let stride = struct_slots(&efields);
            let base = ctx.vars.declare_struct_array(&name, n * stride, &elem_name);
            for i in 0..n {
                for fv in &slit.fields {
                    let foff = field_offset(&efields, &member_name(&fv.member)?)?;
                    let v = lower_expr16(&fv.expr, ctx, "struct field (u16 slots)")?;
                    body.push(Stmt::Assign(base + i * stride + foff, v));
                }
            }
        }
        syn::Expr::Repeat(r) => {
            // `[v; N]` — a block fill (one evaluation of `v`, repeated over N slots).
            let n = array_len(&r.len, ctx)?;
            let elem = elem_width(&r.expr);
            let base = ctx.vars.declare(&name, n, None, elem);
            let value = lower_expr16(&r.expr, ctx, "array element (u16 slots)")?;
            body.push(Stmt::Fill {
                base,
                count: n,
                value,
            });
        }
        syn::Expr::Array(arr) => {
            let elem = arr.elems.first().map(elem_width).unwrap_or(Width::Word);
            let base = ctx.vars.declare(&name, arr.elems.len(), None, elem);
            for (i, e) in arr.elems.iter().enumerate() {
                let v = lower_expr16(e, ctx, "array element (u16 slots)")?;
                body.push(Stmt::StoreIndex(base, Expr::Lit(i as u16), v, elem));
            }
        }
        syn::Expr::Struct(lit) => {
            // Resolve to a concrete struct: a regular struct, or a const-/generic
            // struct *instance* (`Buf$8`) inferred from this literal.
            let (sname, fields) = resolve_struct_literal(lit, ctx)?;
            let base = ctx.vars.declare(
                &name,
                struct_slots(&fields),
                Some(sname.clone()),
                Width::Word,
            );
            for fv in &lit.fields {
                let fname = member_name(&fv.member)?;
                let off = field_offset(&fields, &fname)?;
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
                let packed_len = fd.and_then(|f| f.packed_len);
                match &fv.expr {
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
                        for (slot, pair) in
                            arr.elems.iter().collect::<Vec<_>>().chunks(2).enumerate()
                        {
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
        }
        // `let t = &CONST;` — bind a read-only pointer to packed const data. A
        // scalar-array const keeps its element width/stride so `t[i]` loads through
        // the pointer; other const refs bind as an opaque address (pass it on).
        syn::Expr::Reference(r) => {
            let (addr, _) = lower_expr(&init.expr, ctx)?;
            let elem = if let syn::Expr::Path(p) = &*r.expr {
                p.path.get_ident().and_then(|id| {
                    ctx.consts
                        .borrow()
                        .get(&id.to_string())
                        .and_then(|d| d.elem_width.map(|w| (w, d.stride)))
                })
            } else {
                None
            };
            let base = match elem {
                Some((w, stride)) => ctx.vars.declare_elem_ptr(&name, w, stride),
                None => ctx.vars.declare(&name, 1, None, Width::Word),
            };
            body.push(Stmt::Assign(base, addr));
        }
        // `let x = if c { a } else { b };` / `let x = match … { … };` — the value-position
        // conditional lowers to its statement form assigning into `x`'s slot.
        cond @ (syn::Expr::If(_) | syn::Expr::Match(_)) => {
            let ann = match &local.pat {
                syn::Pat::Type(t) => Some(ctx.width_of_type(&t.ty)),
                _ => None,
            };
            let w = value_width(cond, ctx)?;
            if w == Width::DWord && matches!(ann, Some(a) if a != Width::DWord) {
                return Err(format!(
                    "cannot bind a u32 value to 16-bit `{name}` — narrow with `as u16`"
                ));
            }
            let dword = w == Width::DWord || ann == Some(Width::DWord);
            let base = ctx.vars.declare(
                &name,
                if dword { 2 } else { 1 },
                None,
                if dword { Width::DWord } else { w },
            );
            lower_value_into(base, dword, cond, ctx, body)?;
        }
        other => {
            let (e, ty) = lower_expr(other, ctx)?;
            // An explicit `: u32` annotation makes the binding wide even when the
            // initialiser is 16-bit (`let x: u32 = 5;`) — the value zero-extends.
            let ann = match &local.pat {
                syn::Pat::Type(t) => Some(ctx.width_of_type(&t.ty)),
                _ => None,
            };
            if ty == Width::DWord && matches!(ann, Some(w) if w != Width::DWord) {
                return Err(format!(
                    "cannot bind a u32 value to 16-bit `{name}` — narrow with `as u16`"
                ));
            }
            if ty == Width::DWord || ann == Some(Width::DWord) {
                let base = ctx.vars.declare(&name, 2, None, Width::DWord);
                body.push(Stmt::Assign32(base, coerce32(e, ty)));
            } else {
                let base = ctx.vars.declare(&name, 1, None, ty);
                body.push(Stmt::Assign(base, e));
            }
        }
    }
    Ok(())
}

/// Lower `let (a, b, …) = init`. The RHS is either a tuple literal (each component
/// assigned to its own slot) or a function call returning a tuple (one
/// [`Stmt::AssignTuple`] distributing `HL`/`DE`/`BC` into the slots).
fn lower_tuple_let(
    pt: &syn::PatTuple,
    init: &syn::Expr,
    ctx: &mut Ctx,
    body: &mut Vec<Stmt>,
) -> Result<(), String> {
    let names: Vec<String> = pt.elems.iter().map(pat_ident).collect::<Result<_, _>>()?;
    if names.len() > 3 {
        return Err("tuple bindings support up to 3 values".into());
    }
    match init {
        syn::Expr::Tuple(t) => {
            if t.elems.len() != names.len() {
                return Err("tuple binding has the wrong number of values".into());
            }
            // Evaluate all components before binding (Rust evaluates the RHS first).
            let vals: Vec<(Expr, Width)> = t
                .elems
                .iter()
                .map(|e| lower_expr(e, ctx))
                .collect::<Result<_, _>>()?;
            for (name, (v, ty)) in names.iter().zip(vals) {
                if ty == Width::DWord {
                    return Err(format!(
                        "u32 value in a 16-bit context (tuple binding `{name}`) — narrow with \
                         `as u16`"
                    ));
                }
                let base = ctx.vars.declare(name, 1, None, ty);
                body.push(Stmt::Assign(base, v));
            }
        }
        call => {
            let (e, _) = lower_expr(call, ctx)?;
            if !matches!(e, Expr::Call(..)) {
                return Err("a tuple binding needs a tuple literal or a function call".into());
            }
            let slots: Vec<usize> = names
                .iter()
                .map(|n| ctx.vars.declare(n, 1, None, Width::Word))
                .collect();
            body.push(Stmt::AssignTuple(slots, e));
        }
    }
    Ok(())
}

/// Lower `arr[i] = rhs`. For a struct-element array assigned a struct literal
/// (`a[i] = Cell { x, y }`), store each field at the element's computed address;
/// otherwise it's an ordinary scalar/field array store.
fn lower_index_assign(
    ix: &syn::ExprIndex,
    rhs: &syn::Expr,
    ctx: &mut Ctx,
    body: &mut Vec<Stmt>,
) -> Result<(), String> {
    // A struct-element array (local `a` or struct field `self.cells`) assigned a struct
    // literal: store each field at the element's computed address.
    if let Ok((base_addr, elem_struct)) = array_base(&ix.expr, ctx) {
        let syn::Expr::Struct(slit) = rhs else {
            return Err(format!(
                "assign a struct-array element with a struct literal (`{elem_struct} {{ … }}`)"
            ));
        };
        let efields = ctx
            .struct_fields(&elem_struct)
            .ok_or_else(|| format!("unknown struct {elem_struct}"))?;
        if efields.iter().any(|f| f.width == Width::DWord) {
            return Err(format!(
                "struct-array elements with u32 fields are not supported yet ({elem_struct})"
            ));
        }
        let stride = (struct_slots(&efields) * 2) as u16;
        let idx = lower_expr16(&ix.index, ctx, "array index")?;
        for fv in &slit.fields {
            let foff = field_offset(&efields, &member_name(&fv.member)?)?;
            let v = lower_expr16(&fv.expr, ctx, "struct field (u16 slots)")?;
            let elem = Expr::Bin(
                BinOp::Add,
                Box::new(base_addr.clone()),
                Box::new(Expr::MulConst(Box::new(idx.clone()), stride)),
                Width::Word,
            );
            let addr = if foff == 0 {
                elem
            } else {
                Expr::Bin(
                    BinOp::Add,
                    Box::new(elem),
                    Box::new(Expr::Lit((foff * 2) as u16)),
                    Width::Word,
                )
            };
            body.push(Stmt::StoreAt(addr, v, Width::Word));
        }
        return Ok(());
    }
    body.push(lower_index_store(ix, rhs, ctx)?);
    Ok(())
}

pub(crate) fn lower_stmt_expr(
    expr: &syn::Expr,
    ctx: &mut Ctx,
    body: &mut Vec<Stmt>,
) -> Result<(), String> {
    match expr {
        syn::Expr::Assign(a) => match &*a.left {
            syn::Expr::Index(ix) => lower_index_assign(ix, &a.right, ctx, body)?,
            syn::Expr::Field(f) => {
                let (val, vw) = lower_expr(&a.right, ctx)?;
                body.push(lower_field_store(f, val, vw, ctx)?);
            }
            _ => {
                let name = path_ident(&a.left)?;
                let slot = ctx.vars.base(&name);
                // `x = if c { a } else { b };` — a value conditional into x's slot.
                if matches!(&*a.right, syn::Expr::If(_) | syn::Expr::Match(_)) {
                    let dword = ctx.vars.ty(&name) == Width::DWord;
                    lower_value_into(slot, dword, &a.right, ctx, body)?;
                    return Ok(());
                }
                let (e, ew) = lower_expr(&a.right, ctx)?;
                if ctx.vars.ty(&name) == Width::DWord {
                    // A 16-bit value widens into a u32 var (`x = 5` on `x: u32`).
                    body.push(Stmt::Assign32(slot, coerce32(e, ew)));
                } else if ew == Width::DWord {
                    return Err(format!(
                        "cannot assign a u32 value to 16-bit `{name}` — narrow with `as u16`"
                    ));
                } else {
                    body.push(Stmt::Assign(slot, e));
                }
            }
        },
        syn::Expr::If(ifx) => {
            let cond = lower_cond(&ifx.cond, ctx)?;
            let then = lower_block(&ifx.then_branch, ctx)?;
            let els = match &ifx.else_branch {
                Some((_, e)) => lower_else(e, ctx)?,
                None => Vec::new(),
            };
            body.push(Stmt::If(cond, then, els));
        }
        syn::Expr::While(w) => {
            let cond = lower_cond(&w.cond, ctx)?;
            ctx.loop_depth += 1;
            let inner = lower_block(&w.body, ctx)?;
            ctx.loop_depth -= 1;
            body.push(Stmt::While(cond, inner));
        }
        // `match` lowers to an if-chain over a scrutinee temporary (no codegen change).
        syn::Expr::Match(m) => {
            let scrut = lower_expr16(&m.expr, ctx, "match scrutinee")?;
            let temp = ctx
                .vars
                .declare(&format!("__match{}", ctx.temp), 1, None, Width::Word);
            ctx.temp += 1;
            body.push(Stmt::Assign(temp, scrut));

            let mut default: Vec<Stmt> = Vec::new();
            let mut arms: Vec<(Vec<PatTest>, Vec<Stmt>)> = Vec::new();
            for arm in &m.arms {
                let arm_body = lower_arm_body(&arm.body, ctx)?;
                match pattern_tests(&arm.pat, ctx)? {
                    Some(t) => arms.push((t, arm_body)),
                    None => default = arm_body, // `_` wildcard
                }
            }
            let mut chain = default;
            for (tests, arm_body) in arms.into_iter().rev() {
                chain = vec![Stmt::If(arm_cond(temp, tests), arm_body, chain)];
            }
            body.extend(chain);
        }
        // A call as a statement: the `poke` intrinsic, or a void call (discarded).
        syn::Expr::Call(c) => {
            let name = path_ident(&c.func)?;
            if name == "poke" {
                let addr = c.args.first().ok_or("poke(addr, val) needs an address")?;
                let val = c.args.get(1).ok_or("poke(addr, val) needs a value")?;
                let addr = lower_expr16(addr, ctx, "poke address")?;
                let val = lower_expr16(val, ctx, "poke value")?;
                body.push(Stmt::Poke(addr, val));
            } else {
                body.push(Stmt::Eval(lower_expr(expr, ctx)?.0));
            }
        }
        // A method call as a statement (e.g. `self.move_head();`).
        syn::Expr::MethodCall(m) => {
            body.push(Stmt::Eval(lower_method_call(m, ctx)?.0));
        }
        // `for var in a..b { … }` — desugared to an init + a counted loop.
        syn::Expr::ForLoop(fl) => lower_for(fl, ctx, body)?,
        // `loop { … }` — an unconditional loop (exit via `break`/`return`).
        syn::Expr::Loop(l) => {
            if l.label.is_some() {
                return Err("loop labels are not supported".into());
            }
            ctx.loop_depth += 1;
            let inner = lower_block(&l.body, ctx)?;
            ctx.loop_depth -= 1;
            body.push(Stmt::Loop(inner));
        }
        syn::Expr::Break(b) => {
            if b.expr.is_some() {
                return Err("`break <value>` is not supported".into());
            }
            if b.label.is_some() {
                return Err("labeled `break` is not supported".into());
            }
            if ctx.loop_depth == 0 {
                return Err("`break` outside a loop".into());
            }
            body.push(Stmt::Break);
        }
        syn::Expr::Continue(c) => {
            if c.label.is_some() {
                return Err("labeled `continue` is not supported".into());
            }
            if ctx.loop_depth == 0 {
                return Err("`continue` outside a loop".into());
            }
            body.push(Stmt::Continue);
        }
        syn::Expr::Return(r) => {
            // `return if c { a } else { b };` — value conditional through a temp slot.
            if let Some(e) = r.expr.as_deref() {
                if matches!(e, syn::Expr::If(_) | syn::Expr::Match(_)) {
                    let w = value_width(e, ctx)?;
                    if w == Width::DWord {
                        return Err(
                            "u32 return values are not supported yet — narrow with `as u16`".into(),
                        );
                    }
                    let temp = ctx.vars.declare(&format!("__val{}", ctx.temp), 1, None, w);
                    ctx.temp += 1;
                    lower_value_into(temp, false, e, ctx, body)?;
                    body.push(Stmt::Return(Some(Expr::Var(temp))));
                    return Ok(());
                }
            }
            let val = match &r.expr {
                Some(e) => Some(lower_expr16(
                    e,
                    ctx,
                    "return value — u32 returns are not supported yet",
                )?),
                None => None,
            };
            body.push(Stmt::Return(val));
        }
        other => {
            return Err(format!(
                "unsupported statement: {} — statements are `let`, assignment, \
                 `if`/`while`/`for`/`loop`/`match`, `break`/`continue`/`return`, and \
                 calls; a bare value needs a `let` (or make it the final expression)",
                super::expr::describe_expr(other)
            ))
        }
    }
    Ok(())
}

/// Lower `for var in start..end { body }` to: assign the loop variable to `start`,
/// snapshot the (once-evaluated) `end` bound into a temp, and emit a [`Stmt::ForRange`]
/// whose step (`var += 1`) is the `continue` target. The loop variable's width is
/// inferred from the start bound.
fn lower_for(fl: &syn::ExprForLoop, ctx: &mut Ctx, body: &mut Vec<Stmt>) -> Result<(), String> {
    if fl.label.is_some() {
        return Err("loop labels are not supported".into());
    }
    // `for _ in …` still needs a counter slot — synthesise a hidden name for it.
    let var_name = match &*fl.pat {
        syn::Pat::Wild(_) => {
            let n = format!("__foridx{}", ctx.temp);
            ctx.temp += 1;
            n
        }
        p => pat_ident(p)?,
    };
    let syn::Expr::Range(range) = &*fl.expr else {
        return Err("`for` only supports integer ranges (`a..b` / `a..=b`)".into());
    };
    let start = range
        .start
        .as_ref()
        .ok_or("`for` range needs a start bound")?;
    let end_expr = range.end.as_ref().ok_or("`for` range needs an end bound")?;
    let inclusive = matches!(range.limits, syn::RangeLimits::Closed(_));

    // Evaluate both bounds before declaring the loop variable (they cannot see it).
    let (start_e, width) = lower_expr(start, ctx)?;
    let (end_e, end_w) = lower_expr(end_expr, ctx)?;
    if width == Width::DWord || end_w == Width::DWord {
        return Err("u32 `for` bounds are not supported yet — narrow with `as u16`".into());
    }
    let end_temp = ctx
        .vars
        .declare(&format!("__forend{}", ctx.temp), 1, None, width);
    ctx.temp += 1;
    let var = ctx.vars.declare(&var_name, 1, None, width);

    body.push(Stmt::Assign(var, start_e));
    body.push(Stmt::Assign(end_temp, end_e));

    ctx.loop_depth += 1;
    let inner = lower_block(&fl.body, ctx)?;
    ctx.loop_depth -= 1;

    body.push(Stmt::ForRange {
        var,
        end: Expr::Var(end_temp),
        inclusive,
        width,
        body: inner,
    });
    Ok(())
}

fn lower_else(e: &syn::Expr, ctx: &mut Ctx) -> Result<Vec<Stmt>, String> {
    match e {
        syn::Expr::Block(b) => lower_block(&b.block, ctx),
        syn::Expr::If(_) => {
            let mut v = Vec::new();
            lower_stmt_expr(e, ctx, &mut v)?;
            Ok(v)
        }
        other => Err(format!(
            "unsupported `else` branch: {} — write `else {{ … }}` or `else if … {{ … }}`",
            super::expr::describe_expr(other)
        )),
    }
}

fn lower_block(b: &syn::Block, ctx: &mut Ctx) -> Result<Vec<Stmt>, String> {
    let mut body = Vec::new();
    for st in &b.stmts {
        match st {
            syn::Stmt::Local(local) => lower_local(local, ctx, &mut body)?,
            syn::Stmt::Expr(expr, _) => lower_stmt_expr(expr, ctx, &mut body)?,
            other => {
                return Err(format!(
                    "unsupported statement: {}",
                    super::expr::describe_stmt(other)
                ))
            }
        }
    }
    Ok(body)
}

fn lower_cond(expr: &syn::Expr, ctx: &mut Ctx) -> Result<Cond, String> {
    // `!cond` negates the comparison in place (a tight conditional jump) rather than
    // materialising a `0`/`1` and testing it — `if !self.started`, `while !done`.
    if let syn::Expr::Unary(u) = expr {
        if matches!(u.op, syn::UnOp::Not(_)) {
            let inner = lower_cond(&u.expr, ctx)?;
            return Ok(Cond {
                cmp: negate_cmp(inner.cmp),
                lhs: inner.lhs,
                rhs: inner.rhs,
                signed: inner.signed,
            });
        }
    }
    // A comparison maps directly; any other bool expression means "is non-zero"
    // (e.g. `if input.held(Button::Left)`).
    if let syn::Expr::Binary(b) = expr {
        if let Some(cmp) = cmp_op(&b.op) {
            let (le, lw) = lower_expr(&b.left, ctx)?;
            let (re, rw) = lower_expr(&b.right, ctx)?;
            if lw == Width::DWord || rw == Width::DWord {
                return Err(
                    "u32 comparisons are not supported yet — compare the words (`as u16`, `>> 16`)"
                        .into(),
                );
            }
            return Ok(Cond {
                cmp,
                lhs: le,
                rhs: re,
                signed: lw == Width::SWord || rw == Width::SWord,
            });
        }
    }
    if let syn::Expr::Paren(p) = expr {
        return lower_cond(&p.expr, ctx);
    }
    let e = lower_expr16(expr, ctx, "condition")?;
    Ok(Cond {
        cmp: Cmp::Ne,
        lhs: e,
        rhs: Expr::Lit(0),
        signed: false,
    })
}

/// The logical negation of a comparison (for `!cond` / `if !x`).
fn negate_cmp(c: Cmp) -> Cmp {
    match c {
        Cmp::Eq => Cmp::Ne,
        Cmp::Ne => Cmp::Eq,
        Cmp::Lt => Cmp::Ge,
        Cmp::Ge => Cmp::Lt,
        Cmp::Le => Cmp::Gt,
        Cmp::Gt => Cmp::Le,
    }
}

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

/// A match arm body: a `{ block }` or a single expression-statement.
fn lower_arm_body(e: &syn::Expr, ctx: &mut Ctx) -> Result<Vec<Stmt>, String> {
    match e {
        syn::Expr::Block(b) => lower_block(&b.block, ctx),
        other => {
            let mut v = Vec::new();
            lower_stmt_expr(other, ctx, &mut v)?;
            Ok(v)
        }
    }
}

/// One test of a `match` arm against the scrutinee: equality with a value, or
/// membership in a (non-negative) literal range.
enum PatTest {
    Eq(Expr),
    Range { lo: Expr, hi: Expr, inclusive: bool },
}

/// A match arm's pattern as an or-list of tests, or `None` for the `_` wildcard.
/// Accepts integer/byte literals, enum variants, ranges (`0..=9`, `b'a'..=b'z'`),
/// and or-patterns (`1 | 2`) — including ranges inside or-patterns. Range bounds
/// are non-negative literals, so the unsigned comparison is exact for `i16`
/// scrutinees too (a negative value fails the upper bound either way).
fn pattern_tests(pat: &syn::Pat, ctx: &Ctx) -> Result<Option<Vec<PatTest>>, String> {
    let lit_value = |l: &syn::Lit| -> Result<Expr, String> {
        match l {
            syn::Lit::Int(i) => Ok(Expr::Lit(
                i.base10_parse::<u16>().map_err(|e| e.to_string())?,
            )),
            syn::Lit::Byte(b) => Ok(Expr::Lit(b.value() as u16)),
            other => Err(format!(
                "unsupported `match` pattern: {} — arms match integer/byte literals, \
                 ranges (`0..=9`), or-patterns (`1 | 2`), enum variants, or `_`",
                super::expr::describe_lit_kind(other)
            )),
        }
    };
    match pat {
        syn::Pat::Wild(_) => Ok(None),
        syn::Pat::Paren(p) => pattern_tests(&p.pat, ctx),
        syn::Pat::Lit(pl) => Ok(Some(vec![PatTest::Eq(lit_value(&pl.lit)?)])),
        syn::Pat::Path(pp) => resolve_enum_path(&pp.path, ctx.enums)
            .map(|v| Some(vec![PatTest::Eq(Expr::Lit(v))]))
            .ok_or_else(|| "unknown enum variant in pattern".into()),
        // `1 | 2 | Dir::Up` — flatten the cases into one or-list.
        syn::Pat::Or(o) => {
            let mut all = Vec::new();
            for p in &o.cases {
                match pattern_tests(p, ctx)? {
                    Some(mut t) => all.append(&mut t),
                    None => {
                        return Err("`_` inside an or-pattern already matches everything — \
                             use a bare `_` arm"
                            .into())
                    }
                }
            }
            Ok(Some(all))
        }
        // `lo..=hi` / `lo..hi` — both bounds required (no open ranges).
        syn::Pat::Range(r) => {
            let bound = |e: &Option<Box<syn::Expr>>| -> Result<Expr, String> {
                let e = e
                    .as_deref()
                    .ok_or("open-ended range patterns are not supported — give both bounds")?;
                match e {
                    syn::Expr::Lit(l) => lit_value(&l.lit),
                    _ => Err("range-pattern bounds must be integer/byte literals".into()),
                }
            };
            Ok(Some(vec![PatTest::Range {
                lo: bound(&r.start)?,
                hi: bound(&r.end)?,
                inclusive: matches!(r.limits, syn::RangeLimits::Closed(_)),
            }]))
        }
        _ => Err(
            "unsupported `match` pattern — arms match integer/byte literals \
                  (`0u16 => …`), ranges (`0..=9 => …`), or-patterns (`1 | 2 => …`), \
                  enum variants (`Dir::Up => …`), or `_` (no bindings or tuples)"
                .into(),
        ),
    }
}

/// The branch condition testing scrutinee `temp` against an arm's or-list. A single
/// equality stays a direct [`Cond`] comparison (the cheap common case); anything
/// compound materialises a `0`/`1` test expression and branches on `!= 0`.
fn arm_cond(temp: usize, tests: Vec<PatTest>) -> Cond {
    if let [PatTest::Eq(_)] = tests.as_slice() {
        let Some(PatTest::Eq(v)) = tests.into_iter().next() else {
            unreachable!()
        };
        return Cond {
            cmp: Cmp::Eq,
            lhs: Expr::Var(temp),
            rhs: v,
            signed: false,
        };
    }
    let cmp = |cmp, lhs: Expr, rhs: Expr| Expr::Cmp {
        cmp,
        lhs: Box::new(lhs),
        rhs: Box::new(rhs),
        signed: false,
    };
    let test_expr = tests
        .into_iter()
        .map(|t| match t {
            PatTest::Eq(v) => cmp(Cmp::Eq, Expr::Var(temp), v),
            PatTest::Range { lo, hi, inclusive } => Expr::Logic {
                and: true,
                lhs: Box::new(cmp(Cmp::Ge, Expr::Var(temp), lo)),
                rhs: Box::new(cmp(
                    if inclusive { Cmp::Le } else { Cmp::Lt },
                    Expr::Var(temp),
                    hi,
                )),
            },
        })
        .reduce(|acc, t| Expr::Logic {
            and: false,
            lhs: Box::new(acc),
            rhs: Box::new(t),
        })
        .expect("an or-list is never empty");
    Cond {
        cmp: Cmp::Ne,
        lhs: test_expr,
        rhs: Expr::Lit(0),
        signed: false,
    }
}

pub(crate) fn pat_ident(pat: &syn::Pat) -> Result<String, String> {
    match pat {
        syn::Pat::Ident(p) => Ok(p.ident.to_string()),
        syn::Pat::Type(t) => pat_ident(&t.pat),
        _ => Err(
            "unsupported `let` pattern — bind a plain name (`let x = …`) or a \
                  tuple of names (`let (a, b) = …`); no nested/struct patterns"
                .into(),
        ),
    }
}

// ── if/match as expressions ─────────────────────────────────────────────────────────
//
// `let x = if a { 1 } else { 2 };` is the single most idiomatic shape an LLM emits.
// A value-position `if`/`match` lowers to its statement form with every arm assigning
// into one destination slot — no codegen support needed. Arms recurse, so nested
// conditionals and `else if` chains land in the same slot.

/// Is this `if` a *value* (every path ends in a trailing expression) rather than a
/// statement? Mirrors rustc's typing for the dialect: a value-`if` needs an `else`,
/// and each branch must end with a no-semicolon expression.
pub(crate) fn if_is_value(ifx: &syn::ExprIf) -> bool {
    let Some((_, els)) = &ifx.else_branch else {
        return false;
    };
    let branch_is_value = |b: &syn::Block| matches!(b.stmts.last(), Some(syn::Stmt::Expr(e, None)) if expr_is_value(e));
    branch_is_value(&ifx.then_branch)
        && match &**els {
            syn::Expr::Block(b) => branch_is_value(&b.block),
            syn::Expr::If(nested) => if_is_value(nested),
            _ => false,
        }
}

/// Is this `match` a *value* — every arm body ends in (or is) a trailing expression?
pub(crate) fn match_is_value(m: &syn::ExprMatch) -> bool {
    !m.arms.is_empty()
        && m.arms.iter().all(|arm| match &*arm.body {
            syn::Expr::Block(b) => {
                matches!(b.block.stmts.last(), Some(syn::Stmt::Expr(e, None)) if expr_is_value(e))
            }
            e => expr_is_value(e),
        })
}

/// A trailing expression that plausibly *produces* a value (vs a void call / statement
/// shape). Conservative: conditionals recurse; calls count (non-void is the common cell
/// shape); loops/assignments don't.
fn expr_is_value(e: &syn::Expr) -> bool {
    match e {
        syn::Expr::If(ifx) => if_is_value(ifx),
        syn::Expr::Match(m) => match_is_value(m),
        syn::Expr::Paren(p) => expr_is_value(&p.expr),
        syn::Expr::Lit(_)
        | syn::Expr::Path(_)
        | syn::Expr::Binary(_)
        | syn::Expr::Unary(_)
        | syn::Expr::Call(_)
        | syn::Expr::MethodCall(_)
        | syn::Expr::Index(_)
        | syn::Expr::Field(_)
        | syn::Expr::Cast(_) => true,
        _ => false,
    }
}

/// The width a value-position expression will produce, probed from its first value
/// path (arms must agree — enforced when each arm actually lowers). Pure: `lower_expr`
/// emits no statements, and generic-instance requests dedup.
pub(crate) fn value_width(expr: &syn::Expr, ctx: &mut Ctx) -> Result<Width, String> {
    match expr {
        syn::Expr::If(ifx) => match ifx.then_branch.stmts.last() {
            Some(syn::Stmt::Expr(e, None)) => value_width(e, ctx),
            _ => Err(
                "an `if` used as a value must end each branch with the value \
                      (no trailing `;`)"
                    .into(),
            ),
        },
        syn::Expr::Match(m) => match m.arms.first() {
            Some(arm) => match &*arm.body {
                syn::Expr::Block(b) => match b.block.stmts.last() {
                    Some(syn::Stmt::Expr(e, None)) => value_width(e, ctx),
                    _ => Err("a `match` arm used as a value must end with the value \
                              (no trailing `;`)"
                        .into()),
                },
                e => value_width(e, ctx),
            },
            None => Err("a `match` used as a value needs at least one arm".into()),
        },
        syn::Expr::Paren(p) => value_width(&p.expr, ctx),
        other => Ok(lower_expr(other, ctx)?.1),
    }
}

/// Lower a value-position expression **into `slot`**, emitting statements. `dword`
/// selects the wide (`u32`, two-slot) destination. This is the temp-slot desugar for
/// `if`/`match` expressions; a plain expression is a straight assignment.
pub(crate) fn lower_value_into(
    slot: usize,
    dword: bool,
    expr: &syn::Expr,
    ctx: &mut Ctx,
    body: &mut Vec<Stmt>,
) -> Result<(), String> {
    match expr {
        syn::Expr::If(ifx) => {
            let Some((_, els)) = &ifx.else_branch else {
                return Err(
                    "an `if` used as a value needs an `else` branch — every path \
                            must produce the value (e.g. `let x = if c { 1u16 } else { 0u16 };`)"
                        .into(),
                );
            };
            let cond = lower_cond(&ifx.cond, ctx)?;
            let then = value_block_into(slot, dword, &ifx.then_branch, ctx)?;
            let els_stmts = match &**els {
                syn::Expr::Block(b) => value_block_into(slot, dword, &b.block, ctx)?,
                nested @ syn::Expr::If(_) => {
                    // `else if …` chain — recurse into the same slot.
                    let mut v = Vec::new();
                    lower_value_into(slot, dword, nested, ctx, &mut v)?;
                    v
                }
                other => {
                    return Err(format!(
                        "unsupported `else` branch for an `if` value: {}",
                        super::expr::describe_expr(other)
                    ))
                }
            };
            body.push(Stmt::If(cond, then, els_stmts));
        }
        syn::Expr::Match(m) => {
            // Scrutinee temp + an if-chain whose arms assign into `slot` (the same
            // desugar as statement-`match`).
            let scrut = lower_expr16(&m.expr, ctx, "match scrutinee")?;
            let temp = ctx
                .vars
                .declare(&format!("__match{}", ctx.temp), 1, None, Width::Word);
            ctx.temp += 1;
            body.push(Stmt::Assign(temp, scrut));

            let mut default: Option<Vec<Stmt>> = None;
            let mut arms: Vec<(Vec<PatTest>, Vec<Stmt>)> = Vec::new();
            for arm in &m.arms {
                let mut ab = Vec::new();
                match &*arm.body {
                    syn::Expr::Block(b) => {
                        ab = value_block_into(slot, dword, &b.block, ctx)?;
                    }
                    e => lower_value_into(slot, dword, e, ctx, &mut ab)?,
                }
                match pattern_tests(&arm.pat, ctx)? {
                    Some(t) => arms.push((t, ab)),
                    None => default = Some(ab),
                }
            }
            let Some(default) = default else {
                return Err(
                    "a `match` used as a value needs a `_` arm — every input must \
                            produce the value (add `_ => <default>`)"
                        .into(),
                );
            };
            let mut chain = default;
            for (tests, ab) in arms.into_iter().rev() {
                chain = vec![Stmt::If(arm_cond(temp, tests), ab, chain)];
            }
            body.extend(chain);
        }
        syn::Expr::Paren(p) => lower_value_into(slot, dword, &p.expr, ctx, body)?,
        other => {
            let (e, w) = lower_expr(other, ctx)?;
            if dword {
                body.push(Stmt::Assign32(slot, coerce32(e, w)));
            } else if w == Width::DWord {
                return Err(
                    "this branch produces a u32 but the destination is 16-bit — narrow \
                     with `as u16` (or make every branch u32)"
                        .into(),
                );
            } else {
                body.push(Stmt::Assign(slot, e));
            }
        }
    }
    Ok(())
}

/// Lower a value-`if`/`match` **branch block**: leading statements as usual, the
/// trailing expression into `slot`.
fn value_block_into(
    slot: usize,
    dword: bool,
    block: &syn::Block,
    ctx: &mut Ctx,
) -> Result<Vec<Stmt>, String> {
    let mut body = Vec::new();
    let n = block.stmts.len();
    if n == 0 {
        return Err("an empty branch can't produce a value — end it with the value".into());
    }
    for (i, st) in block.stmts.iter().enumerate() {
        let last = i + 1 == n;
        match st {
            syn::Stmt::Expr(e, None) if last => lower_value_into(slot, dword, e, ctx, &mut body)?,
            _ if last => {
                return Err(
                    "the branch of an `if`/`match` value must end with the value \
                            (drop the trailing `;`)"
                        .into(),
                )
            }
            syn::Stmt::Local(local) => lower_local(local, ctx, &mut body)?,
            syn::Stmt::Expr(expr, _) => lower_stmt_expr(expr, ctx, &mut body)?,
            other => {
                return Err(format!(
                    "unsupported statement in a value branch: {}",
                    super::expr::describe_stmt(other)
                ))
            }
        }
    }
    Ok(body)
}
