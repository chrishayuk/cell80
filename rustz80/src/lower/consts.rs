//! Program `const` items — the **const-data section** (roadmap: `&CONST → addr`).
//!
//! Two kinds of constant, split by how they reach the code:
//!
//! - **Scalar consts** (`const SPEED: u16 = 3;`) are compile-time values,
//!   substituted as literals at every use site — they never occupy image bytes.
//! - **Data consts** (`const TILE: [u8; 8] = […];`, `const MSG: &str = "hi";`,
//!   `const HERO: Tile = Tile { rows: […] };`, `const SHEET: [Tile; 4] = […];`)
//!   are **byte-packed into the image** after the code, and `&TILE` / `TILE[i]`
//!   resolve against that address — so a prelude routine can receive a pointer to
//!   real bitmap/tile/string data instead of a colour by value.
//!
//! Data layout is *packed, little-endian, declaration order* (`u8` → 1 byte,
//! `u16`/`i16` → 2 bytes LE, arrays/structs concatenate). This is **not** the
//! 2-byte-slot layout of *state* structs: const data is read through `peek` /
//! `&[T; N]` reference parameters (byte-addressed), never through the slot-based
//! field access of a `self` receiver. Strings are length-prefixed: a
//! **little-endian `u16` length**, then the bytes — the Phase S wire format
//! (`docs/11-machine-text.md` §1): length low byte at `peek(s)`, high at
//! `peek(s + 1)`, byte `i` at `peek(s + 2 + i)`. Byte strings (`b"…"`,
//! `const B: &[u8; N] = b"…";`) are **raw** bytes with no prefix — their
//! `[u8; N]` type carries the length.

use crate::ir::Width;
use std::collections::HashMap;

/// One data const, laid into the image as `bytes` at a symbol named `name`.
#[derive(Debug, Clone)]
pub(crate) struct DataConst {
    pub(crate) name: String,
    pub(crate) bytes: Vec<u8>,
    /// The element's value width when the const is directly indexable
    /// (`[u8; N]` / `[u16; N]` / `[i16; N]`); `None` for strings, struct consts,
    /// and struct arrays (addressed, not element-read).
    pub(crate) elem_width: Option<Width>,
    /// Element byte stride — `1`/`2` for scalar arrays, the packed struct size for
    /// `[Struct; N]` (drives `&NAME[i]` address math). `0` marks a non-array.
    pub(crate) stride: u16,
    /// Element count (arrays) — for the out-of-range diagnostics.
    pub(crate) len: u16,
    /// The const's Rust type is a reference (a `&str` const / interned string
    /// literal, or a `&[u8; N]` const / interned byte-string) — so its bare name
    /// *is* the address, and `&NAME` would be a reference to a reference.
    pub(crate) is_ref: bool,
}

/// Every `const` in the program: scalars by value, data consts by (eventual) address.
/// Also the string-literal intern pool — a `"…"` argument becomes an anonymous data
/// const, deduplicated by content.
#[derive(Debug, Default)]
pub(crate) struct ConstTable {
    pub(crate) scalars: HashMap<String, (u16, Width)>,
    pub(crate) data: Vec<DataConst>,
    /// Interned string content → its data-const name (`__str0`, `__str1`, …).
    strings: HashMap<String, String>,
    /// Interned byte-string content → its data-const name (`__bytes0`, …).
    byte_strings: HashMap<Vec<u8>, String>,
}

impl ConstTable {
    pub(crate) fn get(&self, name: &str) -> Option<&DataConst> {
        self.data.iter().find(|d| d.name == name)
    }

    /// Intern a string literal as a length-prefixed data const, deduplicated by
    /// content. Returns the const's symbol name.
    pub(crate) fn intern_str(&mut self, s: &str) -> Result<String, String> {
        if let Some(name) = self.strings.get(s) {
            return Ok(name.clone());
        }
        if s.len() > 1024 {
            return Err(format!(
                "string literal is {} bytes — const data caps at 1024",
                s.len()
            ));
        }
        let name = format!("__str{}", self.strings.len());
        let mut bytes = Vec::with_capacity(s.len() + 2);
        bytes.extend_from_slice(&(s.len() as u16).to_le_bytes());
        bytes.extend_from_slice(s.as_bytes());
        self.data.push(DataConst {
            name: name.clone(),
            bytes,
            elem_width: None,
            stride: 0,
            len: s.len() as u16,
            is_ref: true,
        });
        self.strings.insert(s.to_string(), name.clone());
        Ok(name)
    }

    /// Intern a byte-string literal (`b"…"`) as a **raw** (unprefixed) data const,
    /// deduplicated by content — its `[u8; N]` type carries the length, so unlike
    /// a string there is no length byte. Returns the const's symbol name.
    pub(crate) fn intern_bytes(&mut self, b: &[u8]) -> Result<String, String> {
        if let Some(name) = self.byte_strings.get(b) {
            return Ok(name.clone());
        }
        if b.len() > 1024 {
            return Err(format!(
                "byte-string literal is {} bytes — const data caps at 1024",
                b.len()
            ));
        }
        let name = format!("__bytes{}", self.byte_strings.len());
        self.data.push(DataConst {
            name: name.clone(),
            bytes: b.to_vec(),
            elem_width: Some(Width::Byte),
            stride: 1,
            len: b.len() as u16,
            is_ref: true,
        });
        self.byte_strings.insert(b.to_vec(), name.clone());
        Ok(name)
    }
}

/// The packed byte layout of a struct used as *const data*: each field in
/// declaration order, `u8` → 1 byte, `u16`/`i16` → 2 LE, arrays and nested structs
/// concatenated. (Distinct from the 2-byte-slot layout of state structs.)
struct PackedField {
    name: String,
    ty: syn::Type,
}
type PackedStructs = HashMap<String, Vec<PackedField>>;

/// Collect every top-level `const` item into a [`ConstTable`]. Struct-typed consts
/// pack against the file's struct definitions; scalar consts collected earlier in
/// the file may size/fill later ones.
pub(crate) fn collect_consts(file: &syn::File) -> Result<ConstTable, String> {
    let mut structs = PackedStructs::new();
    for item in &file.items {
        if let syn::Item::Struct(s) = item {
            if let syn::Fields::Named(named) = &s.fields {
                structs.insert(
                    s.ident.to_string(),
                    named
                        .named
                        .iter()
                        .map(|f| PackedField {
                            name: f.ident.as_ref().unwrap().to_string(),
                            ty: f.ty.clone(),
                        })
                        .collect(),
                );
            }
        }
    }

    let mut table = ConstTable::default();
    for item in &file.items {
        let syn::Item::Const(c) = item else { continue };
        let name = c.ident.to_string();
        collect_one(&name, &c.ty, &c.expr, &structs, &mut table)?;
    }
    Ok(table)
}

fn collect_one(
    name: &str,
    ty: &syn::Type,
    expr: &syn::Expr,
    structs: &PackedStructs,
    table: &mut ConstTable,
) -> Result<(), String> {
    match ty {
        // Scalar value consts — substituted at use sites, no image bytes.
        syn::Type::Path(p) if scalar_width(&p.path).is_some() => {
            let w = scalar_width(&p.path).unwrap();
            let v = eval_scalar(expr, table).map_err(|e| format!("const `{name}`: {e}"))?;
            table.scalars.insert(name.to_string(), (v, w));
            Ok(())
        }
        // `&str` — length-prefixed bytes; the name aliases the interned data.
        syn::Type::Reference(r) if is_str(&r.elem) => {
            let syn::Expr::Lit(l) = expr else {
                return Err(format!(
                    "const `{name}`: a &str const must be a string literal"
                ));
            };
            let syn::Lit::Str(s) = &l.lit else {
                return Err(format!(
                    "const `{name}`: a &str const must be a string literal"
                ));
            };
            let interned = table.intern_str(&s.value())?;
            let d = table.get(&interned).unwrap().clone();
            table.data.push(DataConst {
                name: name.to_string(),
                ..d
            });
            Ok(())
        }
        // `&[u8; N]` — a byte-string const (`const CRLF: &[u8; 2] = b"\r\n";`): raw
        // packed bytes, the bare name is the address, elements index like any
        // array const (`CRLF[i]`).
        syn::Type::Reference(r) if byte_array_len(&r.elem).is_some() => {
            let n = byte_array_len(&r.elem).unwrap()?;
            let bytes = match expr {
                syn::Expr::Lit(l) => match &l.lit {
                    syn::Lit::ByteStr(bs) => bs.value(),
                    _ => {
                        return Err(format!(
                            "const `{name}`: a `&[u8; N]` const must be a byte-string \
                             literal (`b\"…\"`)"
                        ))
                    }
                },
                _ => {
                    return Err(format!(
                        "const `{name}`: a `&[u8; N]` const must be a byte-string \
                         literal (`b\"…\"`)"
                    ))
                }
            };
            if bytes.len() != n {
                return Err(format!(
                    "const `{name}`: the byte-string is {} bytes for a `&[u8; {n}]`",
                    bytes.len()
                ));
            }
            table.data.push(DataConst {
                name: name.to_string(),
                bytes,
                elem_width: Some(Width::Byte),
                stride: 1,
                len: n as u16,
                is_ref: true,
            });
            Ok(())
        }
        // Scalar arrays `[u8; N]` / `[u16; N]` / `[i16; N]` — indexable data.
        syn::Type::Array(arr) if scalar_elem(&arr.elem).is_some() => {
            let w = scalar_elem(&arr.elem).unwrap();
            let n = super::layout::lit_len(&arr.len).map_err(|e| format!("const `{name}`: {e}"))?;
            let mut bytes = Vec::new();
            pack_scalar_array(expr, w, n, table, &mut bytes)
                .map_err(|e| format!("const `{name}`: {e}"))?;
            table.data.push(DataConst {
                name: name.to_string(),
                bytes,
                elem_width: Some(w),
                stride: scalar_bytes(w),
                len: n as u16,
                is_ref: false,
            });
            Ok(())
        }
        // `[Struct; N]` — packed elements; `&NAME[i]` addresses element `i`.
        syn::Type::Array(arr) => {
            if type_ident(&arr.elem).is_none() {
                return Err(format!(
                    "const `{name}`: array element must be a scalar or a named struct"
                ));
            }
            let n = super::layout::lit_len(&arr.len).map_err(|e| format!("const `{name}`: {e}"))?;
            let syn::Expr::Array(a) = expr else {
                return Err(format!("const `{name}`: expected an array literal"));
            };
            if a.elems.len() != n {
                return Err(format!(
                    "const `{name}`: {} elements for a length-{n} array",
                    a.elems.len()
                ));
            }
            let mut bytes = Vec::new();
            for e in &a.elems {
                pack_value(e, &arr.elem, structs, table, &mut bytes)
                    .map_err(|e| format!("const `{name}`: {e}"))?;
            }
            let stride = bytes.len().checked_div(n).unwrap_or(0) as u16;
            table.data.push(DataConst {
                name: name.to_string(),
                bytes,
                elem_width: None,
                stride,
                len: n as u16,
                is_ref: false,
            });
            Ok(())
        }
        // A named struct const (`const HERO: Tile = Tile { … };`) — packed bytes.
        syn::Type::Path(p) => {
            let sname = p
                .path
                .get_ident()
                .map(|i| i.to_string())
                .ok_or_else(|| format!("const `{name}`: unsupported const type"))?;
            if !structs.contains_key(&sname) {
                return Err(format!(
                    "const `{name}`: unknown type `{sname}` — const data can be a scalar, \
                     `[u8/u16/i16; N]`, `&str`, or a struct defined in this program"
                ));
            }
            let mut bytes = Vec::new();
            pack_value(expr, ty, structs, table, &mut bytes)
                .map_err(|e| format!("const `{name}`: {e}"))?;
            table.data.push(DataConst {
                name: name.to_string(),
                bytes,
                elem_width: None,
                stride: 0,
                len: 1,
                is_ref: false,
            });
            Ok(())
        }
        _ => Err(format!(
            "const `{name}`: unsupported const type — scalars, `[u8/u16/i16; N]`, `&str`, \
             `&[u8; N]`, structs, and `[Struct; N]` are supported"
        )),
    }
}

/// Pack one value of type `ty` (packed layout) onto `out`.
fn pack_value(
    expr: &syn::Expr,
    ty: &syn::Type,
    structs: &PackedStructs,
    table: &ConstTable,
    out: &mut Vec<u8>,
) -> Result<(), String> {
    match ty {
        syn::Type::Path(p) if scalar_width(&p.path).is_some() => {
            let w = scalar_width(&p.path).unwrap();
            let v = eval_scalar(expr, table)?;
            push_scalar(v, w, out)
        }
        syn::Type::Path(p) => {
            let sname = p
                .path
                .get_ident()
                .map(|i| i.to_string())
                .ok_or("unsupported field type")?;
            let fields = structs
                .get(&sname)
                .ok_or_else(|| format!("unknown struct `{sname}`"))?;
            let syn::Expr::Struct(s) = expr else {
                return Err(format!("expected a `{sname} {{ … }}` struct literal"));
            };
            // Pack in *declaration* order, matching by field name.
            for f in fields {
                let fv = s
                    .fields
                    .iter()
                    .find(|fx| {
                        super::layout::member_name(&fx.member)
                            .map(|n| n == f.name)
                            .unwrap_or(false)
                    })
                    .ok_or_else(|| format!("struct literal is missing field `{}`", f.name))?;
                pack_value(&fv.expr, &f.ty, structs, table, out)?;
            }
            Ok(())
        }
        syn::Type::Array(arr) => {
            if let Some(w) = scalar_elem(&arr.elem) {
                let n = super::layout::lit_len(&arr.len)?;
                return pack_scalar_array(expr, w, n, table, out);
            }
            let n = super::layout::lit_len(&arr.len)?;
            let syn::Expr::Array(a) = expr else {
                return Err("expected an array literal".into());
            };
            if a.elems.len() != n {
                return Err(format!("{} elements for a length-{n} array", a.elems.len()));
            }
            for e in &a.elems {
                pack_value(e, &arr.elem, structs, table, out)?;
            }
            Ok(())
        }
        _ => Err("unsupported const field type".into()),
    }
}

/// Pack `[T; N]` of scalars: an `[a, b, …]` literal or an `[v; N]` repeat.
fn pack_scalar_array(
    expr: &syn::Expr,
    w: Width,
    n: usize,
    table: &ConstTable,
    out: &mut Vec<u8>,
) -> Result<(), String> {
    match expr {
        syn::Expr::Array(a) => {
            if a.elems.len() != n {
                return Err(format!("{} elements for a length-{n} array", a.elems.len()));
            }
            for e in &a.elems {
                push_scalar(eval_scalar(e, table)?, w, out)?;
            }
            Ok(())
        }
        syn::Expr::Repeat(r) => {
            let v = eval_scalar(&r.expr, table)?;
            for _ in 0..n {
                push_scalar(v, w, out)?;
            }
            Ok(())
        }
        _ => Err("expected an array literal (`[…]` or `[v; N]`)".into()),
    }
}

fn push_scalar(v: u16, w: Width, out: &mut Vec<u8>) -> Result<(), String> {
    match w {
        Width::Byte => {
            if v > 0xFF {
                return Err(format!("`{v}` is out of range for u8"));
            }
            out.push(v as u8);
        }
        Width::Word | Width::SWord => {
            out.push(v as u8);
            out.push((v >> 8) as u8);
        }
        Width::DWord => return Err("u32 const data is not supported yet".into()),
    }
    Ok(())
}

/// Evaluate a const initialiser scalar: an integer/bool literal, a negated `i16`
/// literal, or a reference to an *earlier* scalar const.
fn eval_scalar(expr: &syn::Expr, table: &ConstTable) -> Result<u16, String> {
    match expr {
        syn::Expr::Lit(l) => match &l.lit {
            syn::Lit::Int(i) => i
                .base10_parse::<u16>()
                .map_err(|_| format!("`{}` is out of the 16-bit range", i.base10_digits())),
            syn::Lit::Bool(b) => Ok(b.value as u16),
            syn::Lit::Byte(b) => Ok(b.value() as u16),
            other => Err(format!(
                "unsupported const value: {}",
                super::expr::describe_lit(other)
            )),
        },
        syn::Expr::Unary(u) if matches!(u.op, syn::UnOp::Neg(_)) => {
            Ok(eval_scalar(&u.expr, table)?.wrapping_neg())
        }
        syn::Expr::Path(p) => {
            let name = p
                .path
                .get_ident()
                .map(|i| i.to_string())
                .ok_or("expected a const name")?;
            table
                .scalars
                .get(&name)
                .map(|(v, _)| *v)
                .ok_or_else(|| format!("`{name}` is not a (previously declared) scalar const"))
        }
        _ => Err("const values must be integer/bool literals or scalar const names".into()),
    }
}

fn is_str(ty: &syn::Type) -> bool {
    matches!(ty, syn::Type::Path(p) if p.path.is_ident("str"))
}

/// `Some(N)` when `ty` is `[u8; N]` (the referent of a `&[u8; N]` const); the inner
/// `Result` reports a bad length expression.
fn byte_array_len(ty: &syn::Type) -> Option<Result<usize, String>> {
    let syn::Type::Array(arr) = ty else {
        return None;
    };
    // Precisely `u8` — `bool` also lowers to a byte but isn't byte-string material.
    if !matches!(&*arr.elem, syn::Type::Path(p) if p.path.is_ident("u8")) {
        return None;
    }
    Some(super::layout::lit_len(&arr.len))
}

fn scalar_width(p: &syn::Path) -> Option<Width> {
    if p.is_ident("u8") {
        Some(Width::Byte)
    } else if p.is_ident("u16") || p.is_ident("usize") {
        Some(Width::Word)
    } else if p.is_ident("i16") {
        Some(Width::SWord)
    } else if p.is_ident("bool") {
        Some(Width::Byte)
    } else {
        None
    }
}

fn scalar_elem(ty: &syn::Type) -> Option<Width> {
    match ty {
        syn::Type::Path(p) => scalar_width(&p.path),
        _ => None,
    }
}

fn scalar_bytes(w: Width) -> u16 {
    match w {
        Width::Byte => 1,
        _ => 2,
    }
}

fn type_ident(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(p) => p.path.get_ident().map(|i| i.to_string()),
        _ => None,
    }
}
