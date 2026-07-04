//! Lower a `syn` file (the accepted subset) to the IR. Unsupported nodes become
//! errors — the "not supported on Z80 / host-only" signal.
//!
//! This module owns the lowering [`Ctx`] and the function-level orchestration
//! (`lower_program`, generic instantiation, parameters, the function body). The work
//! is split across submodules by concern:
//!
//! - [`vars`] — the per-function register file (named locals → slots);
//! - [`layout`] — struct/enum layout + the syntactic parse helpers;
//! - [`prelude`] — handle routing ([`PreludeConfig`]), the generic-compiler hook;
//! - [`generics`] — monomorphization of generic free functions;
//! - [`expr`] — expression lowering (and field/index/method access);
//! - [`stmt`] — statements and control flow (`if`/`while`/`for`/`loop`/`match`).

pub(crate) mod consts;
mod expr;
mod generics;
pub(crate) mod layout;
mod prelude;
mod stmt;
mod vars;

pub use prelude::PreludeConfig;

use crate::ir::*;
use expr::lower_expr;
use generics::{
    collect_generic_fns, collect_generic_methods, collect_generic_structs,
    impl_is_for_generic_struct, instance_name, is_generic_fn, is_generic_sig, GArg, Mono,
};
use layout::{collect_enums, collect_structs, type_name, Enums, FieldDef, Structs};
use prelude::handle_type;
use std::cell::RefCell;
use std::collections::HashMap;
use stmt::{
    if_is_value, lower_local, lower_stmt_expr, lower_value_into, match_is_value, pat_ident,
    value_width,
};
use vars::Vars;

/// Per-function lowering context: locals + the program's struct/enum layouts + the
/// caller's handle-routing config + the (shared) monomorphization state.
pub(crate) struct Ctx<'a> {
    pub(crate) vars: Vars,
    pub(crate) structs: &'a Structs,
    pub(crate) enums: &'a Enums,
    pub(crate) prelude: &'a PreludeConfig,
    /// Counter for synthesised `match`/`for` temporaries.
    pub(crate) temp: usize,
    /// Nesting depth of enclosing loops — so a `break`/`continue` outside any loop is
    /// rejected cleanly rather than producing dangling jumps.
    pub(crate) loop_depth: usize,
    /// Type-parameter → concrete width for the instance being lowered (empty for a
    /// non-generic function).
    pub(crate) type_args: &'a HashMap<String, Width>,
    /// Const-parameter → concrete value for the instance being lowered (used as array
    /// lengths and as plain values; empty for a non-generic function).
    pub(crate) const_args: &'a HashMap<String, u16>,
    /// Shared monomorphization registry/worklist (calls register instances here).
    pub(crate) mono: &'a RefCell<Mono>,
    /// The program's `const` items (scalars substituted, data consts addressed) —
    /// shared mutably so string literals intern into the pool during lowering.
    pub(crate) consts: &'a RefCell<consts::ConstTable>,
}

impl Ctx<'_> {
    /// The width of a type annotation, resolving a generic parameter to its concrete
    /// width for this instantiation (`u8` → byte; a type-param → its bound width;
    /// anything else → word).
    pub(crate) fn width_of_type(&self, t: &syn::Type) -> Width {
        if let syn::Type::Path(p) = t {
            if let Some(id) = p.path.get_ident() {
                let s = id.to_string();
                if let Some(w) = self.type_args.get(&s) {
                    return *w;
                }
                if s == "u8" {
                    return Width::Byte;
                }
                if s == "i16" {
                    return Width::SWord;
                }
                if s == "u32" {
                    return Width::DWord;
                }
            }
        }
        Width::Word
    }

    /// A struct's field layout — a regular struct from the eager map, or a const-/
    /// generic struct *instance* (`Buf$8`) registered on demand at construction.
    pub(crate) fn struct_fields(&self, name: &str) -> Option<Vec<FieldDef>> {
        if let Some(f) = self.structs.get(name) {
            return Some(f.clone());
        }
        self.mono.borrow().struct_instances.get(name).cloned()
    }

    /// Evaluate an array length to a value — an integer literal, or a const-generic
    /// parameter resolved to this instance's value.
    pub(crate) fn eval_len(&self, e: &syn::Expr) -> Result<u16, String> {
        if let syn::Expr::Path(p) = e {
            if let Some(id) = p.path.get_ident() {
                if let Some(n) = self.const_args.get(&id.to_string()) {
                    return Ok(*n);
                }
            }
        }
        if let syn::Expr::Lit(l) = e {
            if let syn::Lit::Int(i) = &l.lit {
                return i.base10_parse::<u16>().map_err(|e| e.to_string());
            }
        }
        Err("array length must be an integer literal or a const-generic parameter".into())
    }
}

/// A lowered program: the functions plus its **const-data pool** (`const` bytes to
/// lay into the image after the code, which `Expr::ConstAddr` references resolve
/// against). Produced by [`lower_program_full`]; consumed by the codegen entries
/// that emit a data section (`codegen_loop_full`, and the internal compile paths).
pub struct Lowered {
    pub funcs: Vec<(String, Func)>,
    pub(crate) consts: consts::ConstTable,
}

/// Lower every `fn` in a file to `(name, Func)`, using the file's struct layouts and
/// the caller's handle-routing config (empty for plain generic compilation).
///
/// Convenience over [`lower_program_full`] for programs without const *data* — any
/// data const referenced by the code will surface as an unknown symbol at encode
/// (pass the [`Lowered`] to a `*_full` codegen entry to lay the data section).
pub fn lower_program(
    file: &syn::File,
    prelude: &PreludeConfig,
) -> Result<Vec<(String, Func)>, String> {
    Ok(lower_program_full(file, prelude)?.funcs)
}

/// [`lower_program`] carrying the program's const-data pool ([`Lowered`]) — the
/// `&CONST → addr` feature: `const` bytes (tiles, strings, tables) lay into the
/// image and `&TILE` / `TILE[i]` / `"text"` resolve to addresses in it.
pub fn lower_program_full(file: &syn::File, prelude: &PreludeConfig) -> Result<Lowered, String> {
    let structs = collect_structs(file)?;
    let enums = collect_enums(file)?;
    let generic_structs = collect_generic_structs(file)?;
    let mut generic_fns = collect_generic_fns(file)?;
    collect_generic_methods(file, &generic_structs, &mut generic_fns)?;
    let mut mono_state = Mono::new(generic_fns);
    mono_state.generic_structs = generic_structs;
    let mono = RefCell::new(mono_state);
    let consts_cell = RefCell::new(consts::collect_consts(file)?);
    let no_args = HashMap::new();
    let no_const = HashMap::new();
    let mut out = Vec::new();
    for item in &file.items {
        match item {
            // `poke`/`peek` are host-only prelude intrinsics — skip their bodies.
            syn::Item::Fn(f) if is_intrinsic(&f.sig.ident.to_string()) => {}
            // Generic functions are lowered on demand, once per instantiation (below).
            syn::Item::Fn(f) if is_generic_fn(f) => {}
            syn::Item::Fn(f) => out.push((
                f.sig.ident.to_string(),
                lower_with(
                    f,
                    &structs,
                    &enums,
                    prelude,
                    &mono,
                    &no_args,
                    &no_const,
                    None,
                    &consts_cell,
                )?,
            )),
            // `impl T { fn m(&mut self, …) }` — each method becomes a `T::m` function
            // taking `self` as a leading pointer argument.
            syn::Item::Impl(imp)
                if impl_is_for_generic_struct(imp, &mono.borrow().generic_structs) =>
            {
                // A const-generic struct's methods are instantiated per struct instance
                // (the worklist), not lowered here.
            }
            syn::Item::Impl(imp) => {
                let self_ty = type_name(&imp.self_ty)?;
                for it in &imp.items {
                    let syn::ImplItem::Fn(m) = it else {
                        return Err("only methods are supported in impl blocks".into());
                    };
                    if is_generic_sig(&m.sig) {
                        return Err("generic methods are not supported (use a free fn)".into());
                    }
                    let name = format!("{self_ty}::{}", m.sig.ident);
                    out.push((
                        name,
                        lower_method(
                            m,
                            &self_ty,
                            &structs,
                            &enums,
                            prelude,
                            &mono,
                            &no_args,
                            &no_const,
                            &consts_cell,
                        )?,
                    ));
                }
            }
            syn::Item::Struct(_) | syn::Item::Enum(_) => {} // already collected
            syn::Item::Const(_) => {}                       // collected into the const table above
            syn::Item::Use(_) => {} // host-only imports — rustz80 has its own prelude
            other => {
                return Err(format!(
                    "only `fn`/`struct`/`enum`/`impl` items are supported: {other:?}"
                ))
            }
        }
    }

    // Drain the instantiation worklist: lowering each instance may request more
    // (a generic fn calling another), so loop until the queue is empty.
    loop {
        let inst = {
            let mut m = mono.borrow_mut();
            m.queue.pop()
        };
        let Some(inst) = inst else { break };
        let gf = mono.borrow().generics[&inst.generic].clone();
        // Split the instance's arguments into type-widths and const-values by the
        // matching parameter's kind.
        let mut type_args: HashMap<String, Width> = HashMap::new();
        let mut const_args: HashMap<String, u16> = HashMap::new();
        for (p, a) in gf.params.iter().zip(&inst.args) {
            match a {
                GArg::Width(w) => {
                    type_args.insert(p.name.clone(), *w);
                }
                GArg::Const(n) => {
                    const_args.insert(p.name.clone(), *n);
                }
            }
        }
        // A generic-struct method lowers with `self` typed as the matching struct
        // instance (`Buf$8`); a free fn has no `self`.
        let self_ty = gf
            .self_ty
            .as_ref()
            .map(|base| instance_name(base, &inst.args));
        let func = lower_with(
            &gf.item,
            &structs,
            &enums,
            prelude,
            &mono,
            &type_args,
            &const_args,
            self_ty.as_deref(),
            &consts_cell,
        )?;
        out.push((inst.name, func));
    }

    if out.is_empty() {
        return Err("no functions found".into());
    }
    // Stage 1 gives every function static local slots — a recursive call clobbers the
    // caller's frame, so any cycle in the call graph would compile to silently wrong
    // values. Reject it here, with the cycle named.
    if let Some(cycle) = crate::dce::find_recursion(&out) {
        return Err(format!(
            "recursion is not supported (Stage 1: static locals) — rewrite as a loop \
             (cycle: {cycle})"
        ));
    }
    Ok(Lowered {
        funcs: out,
        consts: consts_cell.into_inner(),
    })
}

/// Lower a standalone function (no struct/enum context — used by `compile_fn`).
pub fn lower(item: &syn::ItemFn) -> Result<Func, String> {
    let mono = RefCell::new(Mono::default());
    let consts_cell = RefCell::new(consts::ConstTable::default());
    let no_args = HashMap::new();
    let no_const = HashMap::new();
    lower_with(
        item,
        &Structs::new(),
        &Enums::new(),
        &PreludeConfig::default(),
        &mono,
        &no_args,
        &no_const,
        None,
        &consts_cell,
    )
}

/// Lower an `impl` method. The receiver (`&self`/`&mut self`) becomes a leading
/// pointer parameter; `self.field` reads/writes through it.
#[allow(clippy::too_many_arguments)]
fn lower_method<'a>(
    m: &syn::ImplItemFn,
    self_ty: &str,
    structs: &'a Structs,
    enums: &'a Enums,
    prelude: &'a PreludeConfig,
    mono: &'a RefCell<Mono>,
    type_args: &'a HashMap<String, Width>,
    const_args: &'a HashMap<String, u16>,
    consts: &'a RefCell<consts::ConstTable>,
) -> Result<Func, String> {
    let mut ctx = new_ctx(structs, enums, prelude, mono, type_args, const_args, consts);
    let params = lower_inputs(&m.sig.inputs, &mut ctx, Some(self_ty))?;
    let (body, ret) = lower_fn_block(&m.block, &mut ctx)?;
    Ok(Func {
        params,
        n_locals: ctx.vars.next,
        body,
        ret,
    })
}

#[allow(clippy::too_many_arguments)]
fn lower_with<'a>(
    item: &syn::ItemFn,
    structs: &'a Structs,
    enums: &'a Enums,
    prelude: &'a PreludeConfig,
    mono: &'a RefCell<Mono>,
    type_args: &'a HashMap<String, Width>,
    const_args: &'a HashMap<String, u16>,
    self_ty: Option<&str>,
    consts: &'a RefCell<consts::ConstTable>,
) -> Result<Func, String> {
    let mut ctx = new_ctx(structs, enums, prelude, mono, type_args, const_args, consts);
    let params = lower_inputs(&item.sig.inputs, &mut ctx, self_ty)?;
    let (body, ret) = lower_fn_block(&item.block, &mut ctx)?;
    Ok(Func {
        params,
        n_locals: ctx.vars.next,
        body,
        ret,
    })
}

#[allow(clippy::too_many_arguments)]
fn new_ctx<'a>(
    structs: &'a Structs,
    enums: &'a Enums,
    prelude: &'a PreludeConfig,
    mono: &'a RefCell<Mono>,
    type_args: &'a HashMap<String, Width>,
    const_args: &'a HashMap<String, u16>,
    consts: &'a RefCell<consts::ConstTable>,
) -> Ctx<'a> {
    Ctx {
        vars: Vars::default(),
        structs,
        enums,
        prelude,
        temp: 0,
        loop_depth: 0,
        type_args,
        const_args,
        mono,
        consts,
    }
}

/// Names the compiler handles itself (their host definitions are prelude-only).
fn is_intrinsic(name: &str) -> bool {
    matches!(name, "poke" | "peek" | "inport")
}

/// A `&[u8/u16/i16; N]` (immutable) parameter type → its `(element width, byte
/// stride)`. These are read-only pointers into **packed** data (const tiles/tables);
/// a `&mut` reference or any other referent is not one.
fn ref_array_param(t: &syn::Type) -> Option<(Width, u16)> {
    let syn::Type::Reference(r) = t else {
        return None;
    };
    if r.mutability.is_some() {
        return None;
    }
    let syn::Type::Array(arr) = &*r.elem else {
        return None;
    };
    if let syn::Type::Path(p) = &*arr.elem {
        if p.path.is_ident("u8") {
            return Some((Width::Byte, 1));
        }
        if p.path.is_ident("u16") {
            return Some((Width::Word, 2));
        }
        if p.path.is_ident("i16") {
            return Some((Width::SWord, 2));
        }
    }
    None
}

/// A `s: &str` (immutable) parameter — one register holding the address of a
/// length-prefixed buffer (u16 LE length at `s`, bytes at `s + 2` — the Phase S
/// wire format, `docs/11-machine-text.md` §2.1).
fn is_str_param(t: &syn::Type) -> bool {
    let syn::Type::Reference(r) = t else {
        return false;
    };
    r.mutability.is_none() && matches!(&*r.elem, syn::Type::Path(p) if p.path.is_ident("str"))
}

/// Declare a function's parameters, returning the count. `self_ty` is `Some` for
/// methods — then a leading `&self`/`&mut self` receiver is a pointer parameter.
fn lower_inputs(
    inputs: &syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>,
    ctx: &mut Ctx,
    self_ty: Option<&str>,
) -> Result<usize, String> {
    let mut params = 0;
    for (i, arg) in inputs.iter().enumerate() {
        match arg {
            syn::FnArg::Receiver(_) => {
                if i != 0 {
                    return Err("`self` must be the first parameter".into());
                }
                let sty = self_ty.ok_or("`self` outside an impl block")?;
                ctx.vars.declare_ptr("self", sty);
            }
            syn::FnArg::Typed(pt) => {
                let name = pat_ident(&pt.pat)?;
                match handle_type(&pt.ty, ctx.prelude) {
                    Some(h) => ctx.vars.declare_handle(&name, &h),
                    // `t: &[u8; N]` / `&[u16; N]` — a read-only pointer to *packed*
                    // element data (a const tile/table passed by address); `t[i]`
                    // loads through it. Mirrors real Rust: `f(&TILE)` where
                    // `TILE: [u8; 8]`.
                    None => match ref_array_param(&pt.ty) {
                        Some((w, stride)) => ctx.vars.declare_elem_ptr(&name, w, stride),
                        // `s: &str` — the runtime-length sibling of `&[u8; N]`:
                        // one register, the address of a length-prefixed buffer;
                        // reads go through `s.len()` / `s.as_bytes()[i]`.
                        None if is_str_param(&pt.ty) => ctx.vars.declare_str(&name),
                        None => {
                            let w = ctx.width_of_type(&pt.ty);
                            if w == Width::DWord {
                                return Err(format!(
                                    "u32 parameter `{name}` is not supported yet (params pass \
                                     in 16-bit registers) — pass the words and widen with \
                                     `as u32`"
                                ));
                            }
                            ctx.vars.declare(&name, 1, None, w)
                        }
                    },
                };
            }
        }
        params += 1;
    }
    if params > 3 {
        return Err("more than 3 parameters not supported yet (no stack args)".into());
    }
    Ok(params)
}

/// Lower a function body: statements + an optional tail expression. The tail may be
/// a tuple `(a, b)` — a multi-value return placed in `HL`/`DE`/`BC`.
fn lower_fn_block(block: &syn::Block, ctx: &mut Ctx) -> Result<(Vec<Stmt>, Vec<Expr>), String> {
    let mut body = Vec::new();
    let mut ret = Vec::new();
    let stmts = &block.stmts;
    for (i, st) in stmts.iter().enumerate() {
        let last = i + 1 == stmts.len();
        match st {
            syn::Stmt::Local(local) => lower_local(local, ctx, &mut body)?,
            syn::Stmt::Expr(expr, semi) if last && semi.is_none() => match expr {
                syn::Expr::Tuple(t) => {
                    if t.elems.len() > 3 {
                        return Err("tuple returns support up to 3 values".into());
                    }
                    for e in &t.elems {
                        let (le, w) = lower_expr(e, ctx)?;
                        if w == Width::DWord {
                            return Err("u32 return values are not supported yet — narrow with \
                                        `as u16`"
                                .into());
                        }
                        ret.push(le);
                    }
                }
                // A tail `if`/`match` whose branches all end in a value — the
                // idiomatic `fn f() -> u16 { if c { 1 } else { 2 } }` — lowers through
                // a temp slot. (A tail conditional with statement branches stays a
                // statement: void fns legitimately end with `if`.)
                syn::Expr::If(ifx) if if_is_value(ifx) => {
                    ret.push(lower_value_tail(expr, ctx, &mut body)?);
                }
                syn::Expr::Match(m) if match_is_value(m) => {
                    ret.push(lower_value_tail(expr, ctx, &mut body)?);
                }
                _ if is_value_expr(expr) => {
                    let (le, w) = lower_expr(expr, ctx)?;
                    if w == Width::DWord {
                        return Err(
                            "u32 return values are not supported yet — narrow with `as u16`".into(),
                        );
                    }
                    ret.push(le);
                }
                _ => lower_stmt_expr(expr, ctx, &mut body)?,
            },
            syn::Stmt::Expr(expr, _) => lower_stmt_expr(expr, ctx, &mut body)?,
            other => {
                return Err(format!(
                    "unsupported statement: {}",
                    expr::describe_stmt(other)
                ))
            }
        }
    }
    Ok((body, ret))
}

/// Lower a tail-position value `if`/`match` through a hidden temp slot, returning the
/// slot read the epilogue evaluates.
fn lower_value_tail(expr: &syn::Expr, ctx: &mut Ctx, body: &mut Vec<Stmt>) -> Result<Expr, String> {
    let w = value_width(expr, ctx)?;
    if w == Width::DWord {
        return Err("u32 return values are not supported yet — narrow with `as u16`".into());
    }
    let temp = ctx.vars.declare(&format!("__val{}", ctx.temp), 1, None, w);
    ctx.temp += 1;
    lower_value_into(temp, false, expr, ctx, body)?;
    Ok(Expr::Var(temp))
}

fn is_value_expr(e: &syn::Expr) -> bool {
    matches!(
        e,
        syn::Expr::Lit(_)
            | syn::Expr::Path(_)
            | syn::Expr::Binary(_)
            | syn::Expr::Unary(_)
            | syn::Expr::Paren(_)
            | syn::Expr::Call(_)
            | syn::Expr::Index(_)
            | syn::Expr::Cast(_)
            | syn::Expr::Field(_)
            | syn::Expr::MethodCall(_)
    )
}
