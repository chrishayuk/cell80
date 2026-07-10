//! Field / index / const-data access: address computation and load/store lowering.

use super::super::layout::{field_offset, member_name, struct_slots};
use super::super::Ctx;
use super::*;
use crate::ir::*;

/// If `f`'s receiver chain bottoms out at an indexed struct-array element
/// (`a[i].x`, `a[i].pos.x`, any depth), return the index expression and the member
/// chain from the element **outward to the leaf** (`[pos, x]`). `None` for any other
/// receiver (a plain local/`self` field access goes through [`field_target`]).
fn indexed_field_chain(f: &syn::ExprField) -> Option<(&syn::ExprIndex, Vec<&syn::Member>)> {
    let mut members = vec![&f.member];
    let mut base = &*f.base;
    loop {
        match base {
            syn::Expr::Index(ix) => {
                members.reverse(); // collected leaf-first; want element-first
                return Some((ix, members));
            }
            syn::Expr::Field(inner) => {
                members.push(&inner.member);
                base = &*inner.base;
            }
            _ => return None,
        }
    }
}

/// The byte address (and leaf width) of a field *chain* off a struct-array element:
/// `a[i].field`, `a[i].pos.x`, any depth. `&a + index*elem_stride + Σ field_offsets`
/// (all in bytes); intermediate members must be nested structs, the leaf a scalar.
pub(crate) fn elem_field_chain_addr(
    ix: &syn::ExprIndex,
    members: &[&syn::Member],
    ctx: &mut Ctx,
) -> Result<(Expr, Width), String> {
    let (base_addr, elem_struct) = array_base(&ix.expr, ctx)?;
    let mut fields = ctx
        .struct_fields(&elem_struct)
        .ok_or_else(|| format!("unknown struct {elem_struct}"))?;
    let stride = (struct_slots(&fields) * 2) as u16; // stride of the outermost element
    let mut foff = 0usize; // accumulated slot offset down the chain
    let mut width = Width::Word;
    for (mi, member) in members.iter().enumerate() {
        let fname = member_name(member)?;
        foff += field_offset(&fields, &fname)?;
        let fd = fields
            .iter()
            .find(|f| f.name == fname)
            .expect("field_offset just matched it");
        let (fd_struct, fd_width, fd_is_array) = (
            fd.struct_ty.clone(),
            fd.width,
            fd.elem_struct.is_some() || fd.packed_len.is_some() || fd.wide_len.is_some(),
        );
        if mi == members.len() - 1 {
            // The leaf must be a scalar — not a nested struct, array, or `u32`.
            if let Some(s) = fd_struct {
                return Err(format!(
                    "`{fname}` of a struct-array element is a `{s}` struct — read a scalar field of it"
                ));
            }
            if fd_is_array {
                return Err(format!(
                    "`{fname}` of a struct-array element is an array field — not supported here"
                ));
            }
            if fd_width == Width::DWord {
                return Err(format!(
                    "u32 field `{fname}` of a struct-array element is not supported yet"
                ));
            }
            width = fd_width;
        } else {
            // An intermediate member must be a nested struct to descend into.
            let sub = fd_struct.ok_or_else(|| {
                format!("`{fname}` of a struct-array element is not a nested struct — cannot index into it")
            })?;
            fields = ctx
                .struct_fields(&sub)
                .ok_or_else(|| format!("unknown struct {sub}"))?;
        }
    }
    let idx = lower_expr16(&ix.index, ctx, "array index")?;
    // base + index*stride (+ Σ field_offsets)
    let elem = Expr::Bin(
        BinOp::Add,
        Box::new(base_addr),
        Box::new(Expr::MulConst(Box::new(idx), stride)),
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
    Ok((addr, width))
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
    /// `Some(N)` — a byte-packed `[u8; N]` field: element `i` is the byte at
    /// `field_base + i` (not a 2-byte slot). See `FieldDef::packed_len`.
    packed_len: Option<usize>,
    /// `Some(N)` — a `[u32; N]` field: element `i` is the 4-byte value at
    /// `field_base + i*4`. See `FieldDef::wide_len`.
    wide_len: Option<usize>,
    /// `Some(name)` — a **nested struct** field (`sprite: Sprite`): `s.sprite.x`
    /// drills into `name`'s layout. A `FieldRef` whose `field_struct` is set points at
    /// a whole sub-struct, not a scalar — the read/store paths reject that and ask for
    /// a leaf field. See `FieldDef::struct_ty`.
    field_struct: Option<String>,
}

/// Resolve a field access: `obj.field`, a tuple element (`obj.field.N`), or a field of
/// a **nested struct** field (`obj.sprite.x`, any depth). Offsets sum down the chain;
/// `base`/`is_ptr` come from the outermost path receiver, the leaf's `width`/`slots`
/// from the innermost field.
fn field_target(f: &syn::ExprField, ctx: &mut Ctx) -> Result<FieldRef, String> {
    // A nested base (`obj.field.…`): resolve the inner field, then step into it.
    if let syn::Expr::Field(inner) = &*f.base {
        let r = field_target(inner, ctx)?;
        return match &f.member {
            // `obj.field.N` — a tuple element (one slot) at the field's offset + N.
            syn::Member::Unnamed(idx) => Ok(FieldRef {
                off: r.off + idx.index as usize,
                slots: 1,
                width: Width::Word,
                elem_struct: None,
                packed_len: None,
                wide_len: None,
                field_struct: None,
                ..r
            }),
            // `obj.sprite.x` — a field of a nested struct field: drill into the
            // sub-struct's layout. `base`/`is_ptr` stay the outermost receiver's.
            syn::Member::Named(id) => {
                let sub = r.field_struct.as_ref().ok_or_else(|| {
                    format!(
                        "`.{id}` has no such field — the value before it is not a nested struct"
                    )
                })?;
                let fields = ctx
                    .struct_fields(sub)
                    .ok_or_else(|| format!("unknown struct {sub}"))?;
                let name = id.to_string();
                let foff = field_offset(&fields, &name)?;
                let fd = fields.iter().find(|d| d.name == name);
                Ok(FieldRef {
                    off: r.off + foff,
                    slots: fd.map_or(1, |d| d.slots),
                    width: fd.map_or(Width::Word, |d| d.width),
                    elem_struct: fd.and_then(|d| d.elem_struct.clone()),
                    packed_len: fd.and_then(|d| d.packed_len),
                    wide_len: fd.and_then(|d| d.wide_len),
                    field_struct: fd.and_then(|d| d.struct_ty.clone()),
                    ..r
                })
            }
        };
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
        packed_len: fd.and_then(|d| d.packed_len),
        wide_len: fd.and_then(|d| d.wide_len),
        field_struct: fd.and_then(|d| d.struct_ty.clone()),
    })
}

/// The byte address of element `i` of a **byte-packed** `[u8; N]` field: the field's
/// base byte address (through the pointer for `self`, [`Expr::AddrOf`] for a by-value
/// local) plus the index. No bounds check — same as every array access.
fn packed_elem_addr(r: &FieldRef, idx: Expr) -> Expr {
    let field_base = if r.is_ptr {
        Expr::Bin(
            BinOp::Add,
            Box::new(Expr::Var(r.base)),
            Box::new(Expr::Lit((r.off * 2) as u16)),
            Width::Word,
        )
    } else {
        Expr::AddrOf(r.base + r.off)
    };
    Expr::Bin(BinOp::Add, Box::new(field_base), Box::new(idx), Width::Word)
}

/// The byte address of element `i` of a `[u32; N]` field: the field's base byte
/// address plus `i * 4`. No bounds check — same as every array access.
fn wide_elem_addr(r: &FieldRef, idx: Expr) -> Expr {
    let field_base = if r.is_ptr {
        Expr::Bin(
            BinOp::Add,
            Box::new(Expr::Var(r.base)),
            Box::new(Expr::Lit((r.off * 2) as u16)),
            Width::Word,
        )
    } else {
        Expr::AddrOf(r.base + r.off)
    };
    Expr::Bin(
        BinOp::Add,
        Box::new(field_base),
        Box::new(scaled(idx, 4)),
        Width::Word,
    )
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
pub(super) fn lower_const_ref(
    referent: &syn::Expr,
    ctx: &mut Ctx,
) -> Result<(Expr, Width), String> {
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
pub(super) fn lower_index_read(
    ix: &syn::ExprIndex,
    ctx: &mut Ctx,
) -> Result<(Expr, Width), String> {
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
        // A `[u32; N]` field: element `i` is the 4-byte value at `field + i*4`.
        if r.wide_len.is_some() {
            let addr = wide_elem_addr(&r, idx);
            return Ok((Expr::Deref32(Box::new(addr), 0), Width::DWord));
        }
        // A byte-packed `[u8; N]` field: element `i` is the byte at `field + i`.
        if r.packed_len.is_some() {
            let addr = packed_elem_addr(&r, idx);
            return Ok((Expr::LoadAt(Box::new(addr), Width::Byte), Width::Byte));
        }
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
    // A local `[u32; N]` array: a wide load at `&base + i*4`.
    if ctx.vars.wide_array(&arr) {
        let base = ctx.vars.base(&arr);
        let idx = lower_expr16(&ix.index, ctx, "array index")?;
        let addr = Expr::Bin(
            BinOp::Add,
            Box::new(Expr::AddrOf(base)),
            Box::new(scaled(idx, 4)),
            Width::Word,
        );
        return Ok((Expr::Deref32(Box::new(addr), 0), Width::DWord));
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
        // A `[u32; N]` field: a wide store at `field + i*4`.
        if r.wide_len.is_some() {
            let (v, vw) = lower_expr(rhs, ctx)?;
            let addr = wide_elem_addr(&r, idx);
            return Ok(Stmt::Store32(addr, 0, coerce32(v, vw)));
        }
        let val = lower_expr16(rhs, ctx, "array element (u16 slots)")?;
        // A byte-packed `[u8; N]` field: store the low byte at `field + i`.
        if r.packed_len.is_some() {
            let addr = packed_elem_addr(&r, idx);
            return Ok(Stmt::StoreAt(addr, val, Width::Byte));
        }
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
    // A local `[u32; N]` array: a wide store at `&base + i*4`.
    if ctx.vars.wide_array(&arr) {
        let base = ctx.vars.base(&arr);
        let idx = lower_expr16(&ix.index, ctx, "array index")?;
        let (v, vw) = lower_expr(rhs, ctx)?;
        let addr = Expr::Bin(
            BinOp::Add,
            Box::new(Expr::AddrOf(base)),
            Box::new(scaled(idx, 4)),
            Width::Word,
        );
        return Ok(Stmt::Store32(addr, 0, coerce32(v, vw)));
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
pub(super) fn lower_field_read(f: &syn::ExprField, ctx: &mut Ctx) -> Result<(Expr, Width), String> {
    // `a[i].field` (or `a[i].pos.x`, any depth) — a field chain of a struct-array
    // element at a computed address, read at the leaf's width.
    if let Some((ix, members)) = indexed_field_chain(f) {
        let (addr, w) = elem_field_chain_addr(ix, &members, ctx)?;
        return Ok((Expr::LoadAt(Box::new(addr), w), w));
    }
    let r = field_target(f, ctx)?;
    if let Some(sub) = &r.field_struct {
        return Err(format!(
            "a `{sub}` struct field isn't a scalar — read one of its fields (`s.field.x`)"
        ));
    }
    if r.wide_len.is_some() {
        return Err("a `[u32; N]` field is not a scalar — index it (`s.field[i]`)".into());
    }
    if r.width.is_wide() {
        // A wide field reads at its declared representation — an `f32` field's
        // value is f32-typed, never bare bits.
        return Ok(if r.is_ptr {
            (
                Expr::Deref32(Box::new(Expr::Var(r.base)), r.off * 2),
                r.width,
            )
        } else {
            (Expr::Var32(r.base + r.off), r.width)
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
    // `a[i].field = v` (or `a[i].pos.x = v`) — store a field chain of a struct-array
    // element at a computed address, at the leaf's width.
    if let Some((ix, members)) = indexed_field_chain(f) {
        let (addr, w) = elem_field_chain_addr(ix, &members, ctx)?;
        if vw == Width::DWord {
            return Err(
                "u32 value in a 16-bit context (struct-array element field) — narrow with \
                 `as u16`"
                    .into(),
            );
        }
        return Ok(Stmt::StoreAt(addr, val, w));
    }
    let r = field_target(f, ctx)?;
    if let Some(sub) = &r.field_struct {
        return Err(format!(
            "a `{sub}` struct field isn't a scalar — assign one of its fields (`s.field.x = v`)"
        ));
    }
    if r.wide_len.is_some() {
        return Err("a `[u32; N]` field is not a scalar — index it (`s.field[i] = v`)".into());
    }
    if r.width.is_wide() {
        // The representation must agree: an f32 field takes f32 values, a u32 field
        // takes integers (16-bit widens); bits never cross silently.
        if (r.width == Width::F32) != (vw == Width::F32) {
            return Err(format!(
                "cannot assign this value — the field is {} and the value is {}; \
                 conversions are explicit (`int_to_f32`/`f32_to_int_trunc`)",
                if r.width == Width::F32 { "f32" } else { "u32" },
                if vw == Width::F32 {
                    "f32"
                } else {
                    "an integer"
                }
            ));
        }
        let val = coerce32(val, vw);
        return Ok(if r.is_ptr {
            Stmt::Store32(Expr::Var(r.base), r.off * 2, val)
        } else {
            Stmt::Assign32(r.base + r.off, val)
        });
    }
    if vw.is_wide() {
        return Err(
            "cannot assign a u32/f32 value to a 16-bit field — narrow with `as u16`".into(),
        );
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
