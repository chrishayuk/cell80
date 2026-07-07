//! `rustz80` — a **generic** restricted-Rust → Z80 compiler
//! ([spec 07](../docs/07-rust-z80-compiler-spec.md)).
//!
//! Parse a Rust source with [`syn`], lower a bounded subset to a tiny typed IR, and
//! emit Z80 machine code — `u16`/`u8`, arithmetic, `if`/`while`, `for` ranges,
//! `loop`/`break`/`continue`, early `return`, comparisons, arrays, `struct`/`enum`,
//! functions, methods, and `poke`/`peek`/`inport` intrinsics. The accepted subset is
//! *also real Rust*, so the same source runs under rustc and compiles here, checked
//! against each other by differential testing on the emulator (`tests/diff.rs`).
//!
//! This crate knows nothing about "games" or any particular SDK. Method calls on a
//! *handle* parameter route to free prelude functions via a caller-supplied
//! [`PreludeConfig`]; the game/SDK layer lives above (`chuk-speccy-sdk`).
//!
//! Not an LLVM backend, no real `core`: codegen uses `HL` as the accumulator, `DE`
//! as secondary, and a fixed RAM scratch region as the "register file".

mod canon;
mod codegen;
mod dce;
pub mod diag;
mod inline;
mod ir;
mod lower;
mod softfloat;
mod tap;

pub use canon::{
    canonical_unit, canonicalize_source, CanonMode, CanonOptions, CanonOutput, Rename, UnitHint,
    UNIT_TABLE_VERSION,
};
pub use codegen::{codegen_loop, Target};
pub use diag::{classify_error, Diag, DiagCode, Repair};
pub use ir::Func;
pub use lower::{lower_program, lower_program_full, Lowered, PreludeConfig};
pub use softfloat::F32_KERNELS;
pub use tap::to_tap;

/// [`codegen_loop`] over a [`Lowered`] program — lays the **const-data section**
/// (tiles, strings, tables) into the image after the code, so `&CONST` / string
/// literals resolve to real addresses. The pairing for [`lower_program_full`].
pub fn codegen_loop_full(
    lowered: &Lowered,
    org: u16,
    entry: &str,
    state_base: u16,
    state_bytes: u16,
) -> Result<Vec<u8>, String> {
    codegen::codegen_loop_c(
        &lowered.funcs,
        &lowered.consts.data,
        org,
        entry,
        state_base,
        state_bytes,
    )
}

use std::collections::HashMap;

/// Where compiled code is laid out (absolute jump targets are resolved against it).
pub const ORG: u16 = 0x8000;

/// A compiled program: the machine code (loaded at [`ORG`]) and the absolute
/// address of each function by name.
#[derive(Clone)]
pub struct Program {
    pub code: Vec<u8>,
    pub symbols: HashMap<String, u16>,
}

/// One entry in a [`Program::size_report`]: a named function (or appended runtime routine, or
/// a monomorphized instance) and the byte span it occupies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnSize {
    pub name: String,
    pub addr: u16,
    pub size: u16,
    /// A monomorphized generic instance (its mangled name carries a `$`).
    pub instance: bool,
}

impl Program {
    /// Per-symbol code sizes, in layout order. Each function's size is the gap to the
    /// next symbol's address (the last reaches the end of `code`) — so it accounts for
    /// every monomorphized instance and the appended `__mul16`/`__divmod16` runtime.
    /// Lets you *see* what generics/tuples cost in bytes ([`FnSize::instance`] flags
    /// the instances). Sizes sum to `code.len()` for a [`compile_program`] image.
    pub fn size_report(&self) -> Vec<FnSize> {
        let mut syms: Vec<(&String, u16)> = self.symbols.iter().map(|(n, &a)| (n, a)).collect();
        syms.sort_by_key(|&(_, a)| a);
        let end = ORG.wrapping_add(self.code.len() as u16);
        syms.iter()
            .enumerate()
            .map(|(i, &(name, addr))| {
                let next = syms.get(i + 1).map(|&(_, a)| a).unwrap_or(end);
                FnSize {
                    name: name.clone(),
                    addr,
                    size: next.wrapping_sub(addr),
                    instance: name.contains('$'),
                }
            })
            .collect()
    }
}

/// Compile a multi-`fn` program. Functions are laid out in source order from
/// [`ORG`]; calls resolve by name; the mul/div micro-runtime is appended if used.
pub fn compile_program(src: &str) -> Result<Program, String> {
    compile_program_for(src, Target::Spectrum48)
}

/// [`compile_program`] with an explicit [`Target`] — so the differential harness can
/// exercise the Cell target's `ED FE` trap path against the same rustc oracle.
pub fn compile_program_for(src: &str, target: Target) -> Result<Program, String> {
    let file: syn::File = syn::parse_str(src).map_err(|e| format!("parse error: {e}"))?;
    compile_file(&file, target)
}

/// Compile an already-parsed file for `target` — lets a caller that has parsed the source
/// (e.g. the `cell80` crate's capability scan) avoid a second parse, and pick the backend.
pub fn compile_file(file: &syn::File, target: Target) -> Result<Program, String> {
    let lowered = lower_program_full(file, &PreludeConfig::default())?;
    let funcs = inline::inline(lowered.funcs, &[]);
    let (code, symbols) =
        codegen::codegen_program_c(&funcs, &lowered.consts.data, ORG, None, target)?;
    Ok(Program { code, symbols })
}

/// [`compile_file_pruned`] from source text — the cell layer's path (inline, then prune the
/// shared-kernel prelude to what the entry reaches), exposed for callers holding a `&str`.
pub fn compile_program_pruned(src: &str, roots: &[&str]) -> Result<Program, String> {
    let file: syn::File = syn::parse_str(src).map_err(|e| format!("parse error: {e}"))?;
    compile_file_pruned(&file, Target::Spectrum48, roots)
}

/// Like [`compile_file`], but **dead-code-eliminates** down to the functions reachable from
/// the entry `roots`. Whole-program compilers (games) don't know the entry until runtime, so
/// they call [`compile_file`] (keep everything); the cell layer knows a cell's entry
/// (`run`/`main`/`Type::run`) and passes it here, so a prepended shared-kernel prelude — and
/// any other dead code — is pruned to exactly what that entry reaches. An empty `roots` is a
/// no-op (identical to [`compile_file`]).
pub fn compile_file_pruned(
    file: &syn::File,
    target: Target,
    roots: &[&str],
) -> Result<Program, String> {
    let lowered = lower_program_full(file, &PreludeConfig::default())?;
    let funcs = inline::inline(lowered.funcs, roots);
    let funcs = dce::prune(funcs, roots);
    let (code, symbols) =
        codegen::codegen_program_c(&funcs, &lowered.consts.data, ORG, None, target)?;
    Ok(Program { code, symbols })
}

/// One field of a [`struct_layout`]: its name, slot offset, and slot count (1 for a
/// scalar, `N` for `[T; N]` / a tuple, `N × sizeof(elem)` for `[Cell; N]`). Each slot is
/// a 2-byte `u16` cell, so a field's byte address (relative to a struct base) is
/// `offset * 2`. This is the typed-state ABI: the layout a caller writes inputs into and
/// reads outputs out of (`rustz80-cell`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldLayout {
    pub name: String,
    /// Slot offset from the struct's base.
    pub offset: u16,
    /// Slot count (each slot is one 2-byte `u16`).
    pub slots: u16,
    /// `true` for a `u32` field — two consecutive slots holding one little-endian
    /// 4-byte value (vs. `slots == 2` meaning a 2-element array).
    pub dword: bool,
    /// `Some(N)` for a **byte-packed** `[u8; N]` field (Phase S §2.3): `N` raw bytes
    /// at `base + offset * 2`, occupying `ceil(N / 2)` slots. `None` for everything
    /// else — including `slots == 1` scalars, so a `[u8; 1]`/`[u8; 2]` field is never
    /// mistaken for a `u16`.
    pub bytes: Option<u16>,
}

/// The field layout of a (non-generic) named struct in `src` — `(name, slot offset, slot
/// count)` in declaration order. Field byte addresses are `base + offset * 2`. Used to
/// place typed inputs / read typed state for a cell whose state lives at a known base.
pub fn struct_layout(src: &str, name: &str) -> Result<Vec<FieldLayout>, String> {
    let file: syn::File = syn::parse_str(src).map_err(|e| format!("parse error: {e}"))?;
    let structs = lower::layout::collect_structs(&file)?;
    let fields = structs
        .get(name)
        .ok_or_else(|| format!("no struct `{name}`"))?;
    let mut out = Vec::with_capacity(fields.len());
    let mut offset = 0u16;
    for f in fields {
        out.push(FieldLayout {
            name: f.name.clone(),
            offset,
            slots: f.slots as u16,
            // A `[u32; N]` field is wide-*elemented*, not a wide scalar — `dword`
            // must not fire for it (the cell layer would misread it as one u32).
            dword: f.width == ir::Width::DWord && f.wide_len.is_none(),
            bytes: f.packed_len.map(|n| n as u16),
        });
        offset += f.slots as u16;
    }
    Ok(out)
}

/// The typed I/O signature of a cell entry — what a caller/registry needs to map named
/// JSON to args/state without re-parsing the source. For a free `fn run(a, b) -> R`,
/// `params` are the args; for a `&mut self` method, `params` is empty and `state` carries
/// the owning struct's fields (the named typed state).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Signature {
    /// Value parameters `(name, type)` in order.
    pub params: Vec<(String, String)>,
    /// Return type (`"()"` if none).
    pub ret: String,
    /// The receiver struct's fields `(name, type)`, for a `&mut self` method entry.
    pub state: Vec<(String, String)>,
}

impl Signature {
    /// A one-line declaration, e.g. `run(a: u16, b: u16) -> u16`.
    pub fn to_decl(&self, entry: &str) -> String {
        let ps: Vec<String> = self
            .params
            .iter()
            .map(|(n, t)| format!("{n}: {t}"))
            .collect();
        format!("{entry}({}) -> {}", ps.join(", "), self.ret)
    }
}

/// Stringify a dialect type (`u16`, `[u16; 4]`, `(u16, u16)`, `&mut State`, …).
fn type_str(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(p) => p
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default(),
        syn::Type::Array(a) => {
            let len = match &a.len {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(i),
                    ..
                }) => i.base10_digits().to_string(),
                syn::Expr::Path(p) => p
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_default(),
                _ => "?".into(),
            };
            format!("[{}; {}]", type_str(&a.elem), len)
        }
        syn::Type::Tuple(t) => format!(
            "({})",
            t.elems.iter().map(type_str).collect::<Vec<_>>().join(", ")
        ),
        syn::Type::Reference(r) => format!(
            "&{}{}",
            if r.mutability.is_some() { "mut " } else { "" },
            type_str(&r.elem)
        ),
        _ => "?".into(),
    }
}

/// Extract the typed I/O [`Signature`] of `entry` (`"run"` or `"State::run"`) from `src`.
pub fn entry_signature(src: &str, entry: &str) -> Result<Signature, String> {
    let file: syn::File = syn::parse_str(src).map_err(|e| format!("parse error: {e}"))?;
    let (ty_name, fn_name) = match entry.split_once("::") {
        Some((t, m)) => (Some(t), m),
        None => (None, entry),
    };
    // Find the entry's signature — a free `fn`, or a method in `impl Type`.
    let sig = file.items.iter().find_map(|item| match item {
        syn::Item::Fn(f) if ty_name.is_none() && f.sig.ident == fn_name => Some(&f.sig),
        syn::Item::Impl(im) if ty_name.is_some() => {
            let is_target = matches!(&*im.self_ty, syn::Type::Path(p)
                if p.path.segments.last().map(|s| s.ident.to_string()).as_deref() == ty_name);
            is_target.then(|| {
                im.items.iter().find_map(|it| match it {
                    syn::ImplItem::Fn(m) if m.sig.ident == fn_name => Some(&m.sig),
                    _ => None,
                })
            })?
        }
        _ => None,
    });
    let sig = sig.ok_or_else(|| format!("no entry `{entry}`"))?;

    let mut params = Vec::new();
    let mut has_self = false;
    for arg in &sig.inputs {
        match arg {
            syn::FnArg::Receiver(_) => has_self = true,
            syn::FnArg::Typed(pt) => {
                let name = match &*pt.pat {
                    syn::Pat::Ident(i) => i.ident.to_string(),
                    _ => "_".into(),
                };
                params.push((name, type_str(&pt.ty)));
            }
        }
    }
    let ret = match &sig.output {
        syn::ReturnType::Default => "()".to_string(),
        syn::ReturnType::Type(_, t) => type_str(t),
    };
    // For a `&mut self` method, the receiver struct's fields are the named typed state.
    let state = match (has_self, ty_name) {
        (true, Some(tn)) => file
            .items
            .iter()
            .find_map(|item| match item {
                syn::Item::Struct(s) if s.ident == tn => match &s.fields {
                    syn::Fields::Named(named) => Some(
                        named
                            .named
                            .iter()
                            .map(|f| (f.ident.as_ref().unwrap().to_string(), type_str(&f.ty)))
                            .collect(),
                    ),
                    _ => None,
                },
                _ => None,
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    Ok(Signature { params, ret, state })
}

/// Compile a program and wrap it as a bootable `.tap` that runs from `entry`
/// (a function name, default `"main"`). The autoloader `CLEAR`s below [`ORG`],
/// `LOAD`s the code there, and `RANDOMIZE USR`s the entry.
///
/// **Dead-code-eliminated**, rooted at `entry`: a tape carries only the functions the game
/// actually reaches (and so only the `__mul16`/`__divmod16` runtime it actually needs) — which
/// matters on a 48 K Spectrum, where carrying an SDK's unused helpers may not fit.
pub fn compile_to_tap(src: &str, entry: &str, name: &str) -> Result<Vec<u8>, String> {
    let file: syn::File = syn::parse_str(src).map_err(|e| format!("parse error: {e}"))?;
    let lowered = lower_program_full(&file, &PreludeConfig::default())?;
    if !lowered.funcs.iter().any(|(n, _)| n == entry) {
        return Err(format!("no `{entry}` function"));
    }
    let funcs = inline::inline(lowered.funcs, &[entry]);
    let funcs = dce::prune(funcs, &[entry]);
    // Emit a DI/EI trampoline at ORG and boot into it (`USR ORG`).
    let (code, _) = codegen::codegen_program_c(
        &funcs,
        &lowered.consts.data,
        ORG,
        Some(entry),
        Target::Spectrum48,
    )?;
    Ok(to_tap(&code, ORG, ORG, name))
}

/// Compile a single Rust `fn` to Z80 machine code with its entry at [`ORG`]
/// (result in `HL`, then `RET`). Convenience over [`compile_program`].
pub fn compile_fn(src: &str) -> Result<Vec<u8>, String> {
    compile_fn_for(src, Target::Spectrum48)
}

/// [`compile_fn`] with an explicit [`Target`] — so the differential harness can exercise
/// the Cell target's `ED FE` trap path against the same rustc oracle.
pub fn compile_fn_for(src: &str, target: Target) -> Result<Vec<u8>, String> {
    let item: syn::ItemFn = syn::parse_str(src).map_err(|e| {
        // The classic misuse: handing a whole program to the single-`fn` entry. Point
        // at the right API instead of surfacing a bare parse error.
        if let Ok(file) = syn::parse_str::<syn::File>(src) {
            if file.items.len() > 1 {
                return format!(
                    "this source has {} items but `compile_fn` compiles exactly one `fn` — \
                     use `compile_program` for multi-function sources",
                    file.items.len()
                );
            }
            if !matches!(file.items.first(), Some(syn::Item::Fn(_))) {
                return "this source's top-level item is not a `fn` — `compile_fn` takes a \
                        single function; use `compile_program` for structs/impls"
                    .to_string();
            }
        }
        format!("parse error: {e}")
    })?;
    let name = item.sig.ident.to_string();
    let func = lower::lower(&item)?;
    let funcs = [(name, func)];
    // A single fn can still self-recurse — same static-locals hazard as the program path.
    if let Some(cycle) = dce::find_recursion(&funcs) {
        return Err(format!(
            "recursion is not supported (Stage 1: static locals) — rewrite as a loop \
             (cycle: {cycle})"
        ));
    }
    let (code, _) = codegen::codegen_program(&funcs, ORG, None, target)?;
    Ok(code)
}
