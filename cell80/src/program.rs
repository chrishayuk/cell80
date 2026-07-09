//! The compiled, cacheable cell artifact — `CellProgram` + its image format.
use super::config::check_caps;
use super::report::sorted_symbols;
use super::*;
use rustz80::Program;
use std::collections::HashMap;

/// The shared **kernel prelude** appended to every cell before compile. Cells call these
/// instead of re-implementing them; rooting DCE at the cell's entry then keeps only the
/// kernels that entry reaches, so a cartridge carries only what it uses (a cell that calls no
/// kernel is byte-identical — every kernel here is pruned). Appended *after* the cell source
/// so a parse error keeps the cell's own line numbers, and call resolution is order-independent
/// (linked by name).
pub(super) const CELL_PRELUDE: &str = "\
fn gcd(a: u16, b: u16) -> u16 { let mut x = a; let mut y = b; while y != 0u16 { let t = x % y; x = y; y = t; } x }\n\
fn imin(a: u16, b: u16) -> u16 { let mut m = a; if b < a { m = b; } m }\n\
fn imax(a: u16, b: u16) -> u16 { let mut m = a; if b > a { m = b; } m }\n\
fn iabs_diff(a: u16, b: u16) -> u16 { let mut d = 0u16; if a > b { d = a - b; } else { d = b - a; } d }\n\
fn isqrt(n: u16) -> u16 { let mut r = 0u16; while r < 255u16 && (r + 1u16) * (r + 1u16) <= n { r = r + 1u16; } r }\n\
fn clamp_to(x: u16, lo: u16, hi: u16) -> u16 { let mut r = x; if x < lo { r = lo; } if x > hi { r = hi; } r }\n\
fn gcd_u32(a: u32, b: u32) -> u32 { let mut x = a; let mut y = b; while y != 0u32 { let t = y; y = x % y; x = t; } x }\n\
fn mul_checked_u32(a: u32, b: u32) -> u32 { let p = a.wrapping_mul(b); if a != 0u32 && p / a != b { halt(0xFF05u16); } p }\n\
fn add_checked_u32(a: u32, b: u32) -> u32 { let s = a.wrapping_add(b); if s < a { halt(0xFF05u16); } s }\n\
fn sub_checked_u32(a: u32, b: u32) -> u32 { if a < b { halt(0xFF05u16); } a - b }\n\
fn imax_u32(a: u32, b: u32) -> u32 { let mut m = a; if b > a { m = b; } m }\n\
fn imin_u32(a: u32, b: u32) -> u32 { let mut m = a; if b < a { m = b; } m }\n\
fn iabs_diff_u32(a: u32, b: u32) -> u32 { let mut d = 0u32; if a > b { d = a - b; } if b > a { d = b - a; } d }\n";

/// The DCE roots for a cell: its entry functions — every free `fn run`/`fn main` and every
/// `impl` method named `run`/`main` (`Type::run`), matching the cartridge's entry convention.
/// DCE keeps only what these reach, so the appended prelude (and any dead code) is pruned to
/// what the cell actually uses. No entry found → every function the *cell itself* defines
/// becomes a root (they stay compiled and resolvable), while the appended prelude still
/// prunes — with the f32 kernel family aboard, keep-all would overrun the code window.
fn entry_roots(file: &syn::File) -> Vec<String> {
    let is_entry = |id: &syn::Ident| id == "run" || id == "main";
    let mut roots = Vec::new();
    let mut cell_fns = Vec::new();
    for item in &file.items {
        match item {
            syn::Item::Fn(f) => {
                let name = f.sig.ident.to_string();
                if is_entry(&f.sig.ident) {
                    roots.push(name);
                } else if !prelude_fn_names().contains(&name) {
                    cell_fns.push(name);
                }
            }
            syn::Item::Impl(imp) => {
                if let syn::Type::Path(p) = &*imp.self_ty {
                    if let Some(ty) = p.path.segments.last() {
                        for it in &imp.items {
                            if let syn::ImplItem::Fn(m) = it {
                                let name = format!("{}::{}", ty.ident, m.sig.ident);
                                if is_entry(&m.sig.ident) {
                                    roots.push(name);
                                } else {
                                    cell_fns.push(name);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    if roots.is_empty() {
        cell_fns
    } else {
        roots
    }
}

/// The fn names the shared prelude defines (the classic set + the f32 kernel family),
/// parsed once — the no-entry root fallback excludes them so DCE still prunes kernels.
fn prelude_fn_names() -> &'static std::collections::HashSet<String> {
    use std::sync::OnceLock;
    static NAMES: OnceLock<std::collections::HashSet<String>> = OnceLock::new();
    NAMES.get_or_init(|| {
        let text = format!("{CELL_PRELUDE}{}", rustz80::F32_KERNELS);
        let file: syn::File = syn::parse_str(&text).expect("prelude parses");
        file.items
            .iter()
            .filter_map(|item| match item {
                syn::Item::Fn(f) => Some(f.sig.ident.to_string()),
                _ => None,
            })
            .collect()
    })
}

/// A **compiled** cell: the result of parse + lower + codegen under a policy. Cheap to
/// clone and cache (e.g. by source hash) — re-running a known snippet then skips the
/// (syn-parse-dominated, ~16 µs) compile. Turn one into a runnable machine with
/// [`Runner::new`].
#[derive(Clone)]
pub struct CellProgram {
    pub(super) prog: Program,
    pub(super) cfg: CellConfig,
    /// Compiled against the **resident kernel bank**: the image's `CALL`s into
    /// [`rustz80::BANK_ORG`] assume the bank is loaded — [`crate::Runner::new`]
    /// places it (outside touch-tracking, like the code itself).
    pub(super) bank: bool,
}

impl CellProgram {
    /// The safety policy this program was compiled under (capabilities, ceilings,
    /// divide-by-zero behaviour) — it rides in the image.
    pub fn cfg(&self) -> &CellConfig {
        &self.cfg
    }

    /// Compile `src` with the **permissive** policy (raw memory + ports allowed, no
    /// ceilings) — for trusted/game code.
    pub fn compile(src: &str) -> Result<Self, String> {
        Self::compile_with_config(src, CellConfig::permissive())
    }

    /// Compile `src` under `cfg`: enforce its capability gates (`poke`/`peek`/`inport`)
    /// and `max_code_bytes`. Parses once (shared by the cap scan and the compile).
    pub fn compile_with_config(src: &str, cfg: CellConfig) -> Result<Self, String> {
        // Append the shared kernel prelude (plus the owned-softfloat family, which lives
        // in rustz80 so its differential bank tests the same text), then DCE down to what
        // the cell's entry reaches — so a cell calls `gcd`/`iabs_diff`/`fadd`/… without
        // re-implementing them, and the kernels it doesn't use are pruned away.
        let combined = format!("{src}\n{CELL_PRELUDE}{}", rustz80::F32_KERNELS);
        let file: syn::File = syn::parse_str(&combined).map_err(|e| format!("parse error: {e}"))?;
        check_caps(&file, &cfg)?;
        let roots = entry_roots(&file);
        let root_refs: Vec<&str> = roots.iter().map(String::as_str).collect();
        // The cell runs in Cell80 mode: `*`/`/`/`%` lower to `ED FE` host traps that the
        // bus services natively (no software mul/div runtime appended).
        let prog = rustz80::compile_file_pruned(&file, rustz80::Target::Cell, &root_refs)?;
        if let Some(max) = cfg.max_code_bytes {
            if prog.code.len() > max {
                return Err(format!(
                    "code is {} bytes, over the {max}-byte limit",
                    prog.code.len()
                ));
            }
        }
        Ok(CellProgram {
            prog,
            cfg,
            bank: false,
        })
    }

    /// [`compile_with_config`](Self::compile_with_config) against the **resident
    /// kernel bank**: the softfloat family resolves to [`rustz80::BANK_ORG`]
    /// instead of being appended per cell — an f32 cell's image carries only its
    /// own logic (a banked `norm2` is ~100 bytes, not ~5,700). The classic
    /// prelude (`gcd`, the checked family, …) still appends and prunes as ever.
    pub fn compile_with_config_banked(src: &str, cfg: CellConfig) -> Result<Self, String> {
        let combined = format!("{src}\n{CELL_PRELUDE}");
        let file: syn::File = syn::parse_str(&combined).map_err(|e| format!("parse error: {e}"))?;
        check_caps(&file, &cfg)?;
        let roots = entry_roots(&file);
        let root_refs: Vec<&str> = roots.iter().map(String::as_str).collect();
        let prog = rustz80::compile_file_pruned_banked(&file, &root_refs)?;
        if let Some(max) = cfg.max_code_bytes {
            if prog.code.len() > max {
                return Err(format!(
                    "code is {} bytes, over the {max}-byte limit",
                    prog.code.len()
                ));
            }
        }
        Ok(CellProgram {
            prog,
            cfg,
            bank: true,
        })
    }

    /// Whether this program calls into the resident kernel bank (the runner must
    /// load it).
    pub fn uses_kernel_bank(&self) -> bool {
        self.bank
    }

    /// The underlying program (symbol map, code).
    pub fn program(&self) -> &Program {
        &self.prog
    }

    /// Serialize to a compact, self-contained **image** — code + symbols + policy, no syn,
    /// no source. Cache it (by hash), ship it, retrieve it; [`from_bytes`](Self::from_bytes)
    /// reloads it in ~µs, skipping the parse-dominated (~16 µs) compile. The cell
    /// "cartridge": a few dozen bytes you can hash, index, and hand around cheaply.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(IMAGE_MAGIC);
        b.push(IMAGE_VER);
        b.extend_from_slice(&(self.prog.code.len() as u16).to_le_bytes());
        b.extend_from_slice(&self.prog.code);
        let syms = sorted_symbols(&self.prog.symbols); // deterministic → stable hash
        b.extend_from_slice(&(syms.len() as u16).to_le_bytes());
        for (name, addr) in &syms {
            b.push(name.len() as u8);
            b.extend_from_slice(name.as_bytes());
            b.extend_from_slice(&addr.to_le_bytes());
        }
        let c = &self.cfg;
        let flags = (c.allow_raw_memory as u8)
            | (c.allow_ports as u8) << 1
            | (c.max_code_bytes.is_some() as u8) << 2
            | (c.max_touched.is_some() as u8) << 3
            // Bit 4 = the legacy saturate opt-in; absent (0) = halt on divide-by-zero,
            // so pre-policy images load with the safe default.
            | ((c.div_by_zero == DivByZero::Saturate) as u8) << 4
            // Bit 5 = compiled against the resident kernel bank (image v2).
            | (self.bank as u8) << 5;
        b.push(flags);
        b.extend_from_slice(&(c.max_code_bytes.unwrap_or(0) as u32).to_le_bytes());
        b.extend_from_slice(&(c.max_touched.unwrap_or(0) as u32).to_le_bytes());
        b
    }

    /// Reload an image written by [`to_bytes`](Self::to_bytes) — no parse, no compile.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut r = ImageReader { b: bytes, i: 0 };
        if r.take(4)? != IMAGE_MAGIC {
            return Err("not a CZ80 cell image".into());
        }
        let ver = r.u8()?;
        if ver != 1 && ver != IMAGE_VER {
            return Err(format!("unsupported cell-image version {ver}"));
        }
        let code_len = r.u16()? as usize;
        let code = r.take(code_len)?.to_vec();
        let nsym = r.u16()?;
        let mut symbols = HashMap::with_capacity(nsym as usize);
        for _ in 0..nsym {
            let nlen = r.u8()? as usize;
            let name = std::str::from_utf8(r.take(nlen)?)
                .map_err(|_| "bad symbol name in image")?
                .to_string();
            symbols.insert(name, r.u16()?);
        }
        let flags = r.u8()?;
        let max_code = r.u32()? as usize;
        let max_touched = r.u32()? as usize;
        Ok(CellProgram {
            bank: flags & 32 != 0,
            prog: Program { code, symbols },
            cfg: CellConfig {
                allow_raw_memory: flags & 1 != 0,
                allow_ports: flags & 2 != 0,
                max_code_bytes: (flags & 4 != 0).then_some(max_code),
                max_touched: (flags & 8 != 0).then_some(max_touched),
                div_by_zero: if flags & 16 != 0 {
                    DivByZero::Saturate
                } else {
                    DivByZero::Halt
                },
            },
        })
    }
}

const IMAGE_MAGIC: &[u8; 4] = b"CZ80";

// v2 adds the kernel-bank flag (bit 5 of the policy byte): a banked image's
// `CALL`s assume the resident bank at `BANK_ORG`, so a pre-bank host must reject
// it rather than run it bankless into garbage. v1 images load unchanged.
const IMAGE_VER: u8 = 2;

/// A tiny bounds-checked byte cursor — shared by [`CellProgram::from_bytes`] and the
/// `.cell` cartridge reader ([`super::cartridge`]).
pub(super) struct ImageReader<'a> {
    pub(super) b: &'a [u8],
    pub(super) i: usize,
}
impl<'a> ImageReader<'a> {
    pub(super) fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.i.checked_add(n).ok_or("cell image truncated")?;
        let s = self.b.get(self.i..end).ok_or("cell image truncated")?;
        self.i = end;
        Ok(s)
    }
    pub(super) fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    pub(super) fn u16(&mut self) -> Result<u16, String> {
        let s = self.take(2)?;
        Ok(u16::from_le_bytes([s[0], s[1]]))
    }
    pub(super) fn u32(&mut self) -> Result<u32, String> {
        let s = self.take(4)?;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
    pub(super) fn u64(&mut self) -> Result<u64, String> {
        let s = self.take(8)?;
        Ok(u64::from_le_bytes(s.try_into().unwrap()))
    }
    /// A `u16`-length-prefixed UTF-8 string.
    pub(super) fn string(&mut self) -> Result<String, String> {
        let n = self.u16()? as usize;
        Ok(std::str::from_utf8(self.take(n)?)
            .map_err(|_| "bad utf-8 in cell image")?
            .to_string())
    }
}

/// Write a `u16`-length-prefixed UTF-8 string into `b` (mirror of [`ImageReader::string`]).
pub(super) fn put_string(b: &mut Vec<u8>, s: &str) {
    b.extend_from_slice(&(s.len() as u16).to_le_bytes());
    b.extend_from_slice(s.as_bytes());
}
