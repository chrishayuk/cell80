//! MSL codegen (Phase 6 WS-E/E1): lowering of the `cell80-core` typed IR to
//! Metal Shading Language for **straight-line** integer cells — loop-free bodies
//! (`if` allowed; `while`/`loop`/`for` are E2 territory and refuse with a typed
//! error). One thread per input triple: the batch layout E3 grows into.
//!
//! **The data model is the interpreter's.** All IR pointer values are 16-bit
//! addresses into the family window: consts at `0x8000`, the slot file at
//! `0x9000` (slot `i` of a function at `0x9000 + (base + i)·2`, little-endian —
//! the family-wide 2-byte slot ABI, docs 13 §2.2.1). On the GPU the window is
//! emulated per thread: consts ride a shared read-only buffer, the slot file is
//! a thread-local `ushort` array, and byte-routed `rd8`/`wr8` helpers preserve
//! wrapping/aliasing exactly. *Pre-registered weakening (docs 14, E1):* the full
//! 64 KiB image is a per-target choice, not an IR requirement — a read outside
//! the mapped regions returns `0` (the interpreter's untouched-memory value) and
//! a write outside them is a **trap**, never a silent drop.
//!
//! **Width discipline:** 16-bit values (u8/u16/i16) live zero-extended in `uint`
//! temps, re-masked after every op (the interpreter's `mask`); signed 16-bit
//! compare/divide/`>>` sign-extend via bit-cast (`sx16`) at the operation. Wide
//! (u32/i32) values are native `uint`; signed-32 div/rem guard the `MIN/-1` wrap
//! explicitly — C++ would overflow where the IR (rustc `wrapping_*`) defines it.
//! Shift-by-≥-width corners (risk R1) are decided at codegen for literal counts
//! and guarded with `min`/select for runtime counts, matching `interp.rs` arm by
//! arm. Divide-by-zero and `halt(code)` are per-thread traps mirroring the
//! interpreter's refusals — a status word per thread, never a poisoned value.

use cell80_core::ir::*;
use std::collections::HashMap;
use std::fmt::Write as _;

/// Where const data lays in the window (the interpreter's convention).
pub const CONST_BASE: u16 = 0x8000;
/// The slot file's window base (the family scratch region).
pub const SCRATCH: u16 = 0x9000;

/// Per-thread run status: clean.
pub const STATUS_OK: u16 = 0;
/// Per-thread run status: divide by zero (the interpreter's refusal, not a value).
pub const STATUS_DIV0: u16 = 1;
/// Per-thread run status: `halt(code)` — the code rides result register 0.
pub const STATUS_HALT: u16 = 2;
/// Per-thread run status: a write outside the mapped window regions.
pub const STATUS_OOW: u16 = 3;

/// The emitted kernel's function name.
pub const KERNEL_NAME: &str = "cell_main";

/// Inputs consumed per thread (the `HL`/`DE`/`BC` register-arg convention).
pub const IN_STRIDE: usize = 3;
/// Outputs produced per thread: `r0 r1 r2 status`.
pub const OUT_STRIDE: usize = 4;

/// A compiled MSL module: the translation unit (one kernel, [`KERNEL_NAME`]),
/// the const blob to bind read-only at `buffer(2)`, and the entry's shape.
#[derive(Clone, Debug)]
pub struct MslModule {
    pub source: String,
    pub consts: Vec<u8>,
    /// The IR entry function this kernel wraps.
    pub entry: String,
    /// Entry parameter slots consumed from the input triple (≤ 3).
    pub params: usize,
    /// Result registers the entry produces (a wide return is 2: low, high).
    pub ret_regs: usize,
    pub wide_ret: bool,
}

struct FrameInfo {
    base: u16,
    wide_ret: bool,
    ret_len: usize,
}

struct Gen<'a> {
    frames: HashMap<&'a str, (FrameInfo, &'a Func)>,
    consts: HashMap<&'a str, u16>,
    const_len: usize,
    total_slots: u16,
    tmp: usize,
    out: String,
}

/// Compile lowered functions + const data to an MSL module wrapping `entry`.
/// Refuses (with a typed error) anything outside the E1 fragment: loops,
/// recursion, ports, f32.
pub fn compile(
    funcs: &[(String, Func)],
    consts: &[(String, Vec<u8>)],
    entry: &str,
) -> Result<MslModule, String> {
    if let Some(cycle) = cell80_core::dce::find_recursion(funcs) {
        return Err(format!("msl: recursion is not lowered: {cycle}"));
    }
    // Frames laid in `funcs` order with a running base — the interpreter's (and
    // every sibling backend's) slot-assignment rule, so addresses agree.
    let mut frames = HashMap::new();
    let mut base = 0u16;
    for (name, f) in funcs {
        frames.insert(
            name.as_str(),
            (
                FrameInfo {
                    base,
                    wide_ret: f.wide_ret,
                    ret_len: f.ret.len(),
                },
                f,
            ),
        );
        base = base.wrapping_add(f.n_locals as u16);
    }
    let total_slots = base;
    let mut blob = Vec::new();
    let mut const_map = HashMap::new();
    let mut at = CONST_BASE;
    for (name, bytes) in consts {
        const_map.insert(name.as_str(), at);
        blob.extend_from_slice(bytes);
        at = at.wrapping_add(bytes.len() as u16);
    }
    if CONST_BASE as usize + blob.len() > SCRATCH as usize {
        return Err("msl: const data overflows into the slot file".into());
    }
    let (entry_info, entry_fn) = frames
        .get(entry)
        .map(|(i, f)| (i.base, *f))
        .ok_or_else(|| format!("msl: unknown entry `{entry}`"))?;

    let mut g = Gen {
        frames,
        consts: const_map,
        const_len: blob.len(),
        total_slots,
        tmp: 0,
        out: String::new(),
    };
    g.prelude();
    for (name, _) in funcs {
        let _ = writeln!(g.out, "static void {}(thread Ctx& c);", mangle(name));
    }
    g.out.push('\n');
    for (name, f) in funcs {
        g.gen_fn(name, f)?;
    }
    g.kernel(entry, entry_info, entry_fn);

    Ok(MslModule {
        source: g.out,
        consts: blob,
        entry: entry.to_string(),
        params: entry_fn.params.min(IN_STRIDE),
        ret_regs: if entry_fn.wide_ret {
            2
        } else {
            entry_fn.ret.len()
        },
        wide_ret: entry_fn.wide_ret,
    })
}

/// A function's MSL name (`f_` + the IR name, non-identifier chars folded to `_`).
fn mangle(name: &str) -> String {
    let body: String = name
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect();
    format!("f_{body}")
}

impl<'a> Gen<'a> {
    fn prelude(&mut self) {
        let slot_bytes = (self.total_slots as u32) * 2;
        let const_len = self.const_len as u32;
        let o = &mut self.out;
        let _ = writeln!(
            o,
            "// generated by rustmsl — do not edit; semantics are the cell80-core interpreter's\n\
             #include <metal_stdlib>\n\
             using namespace metal;\n\
             \n\
             struct Ctx {{\n\
             \x20   thread ushort* slots;\n\
             \x20   device const uchar* cst;\n\
             \x20   uint trap;\n\
             \x20   uint halt;\n\
             \x20   uint r0; uint r1; uint r2;\n\
             \x20   uint rw;\n\
             \x20   uint rn;\n\
             }};\n\
             \n\
             // Sign-extend a masked 16-bit lane / bit-cast a 32-bit lane to signed.\n\
             static int sx16(uint x) {{ return (int)as_type<short>((ushort)(x)); }}\n\
             static int sx32(uint x) {{ return as_type<int>(x); }}\n\
             \n\
             // Byte-routed window emulation: consts (read-only), the slot file, else\n\
             // zero on read and a trap on write (the pre-registered E1 weakening).\n\
             static uint rd8(thread Ctx& c, uint a) {{\n\
             \x20   a &= 0xFFFFu;\n\
             \x20   if (a >= 0x{cb:X}u && a < 0x{cb:X}u + {const_len}u) return (uint)c.cst[a - 0x{cb:X}u];\n\
             \x20   if (a >= 0x{sc:X}u && a < 0x{sc:X}u + {slot_bytes}u) {{\n\
             \x20       uint o = a - 0x{sc:X}u;\n\
             \x20       return ((uint)c.slots[o >> 1] >> ((o & 1u) * 8u)) & 0xFFu;\n\
             \x20   }}\n\
             \x20   return 0u;\n\
             }}\n\
             static void wr8(thread Ctx& c, uint a, uint v) {{\n\
             \x20   a &= 0xFFFFu;\n\
             \x20   if (a >= 0x{sc:X}u && a < 0x{sc:X}u + {slot_bytes}u) {{\n\
             \x20       uint o = a - 0x{sc:X}u;\n\
             \x20       uint i = o >> 1;\n\
             \x20       uint sh = (o & 1u) * 8u;\n\
             \x20       c.slots[i] = (ushort)(((uint)c.slots[i] & ~(0xFFu << sh)) | ((v & 0xFFu) << sh));\n\
             \x20       return;\n\
             \x20   }}\n\
             \x20   c.trap = {oow}u;\n\
             }}\n\
             static uint rd16(thread Ctx& c, uint a) {{ return rd8(c, a) | (rd8(c, a + 1u) << 8u); }}\n\
             static void wr16(thread Ctx& c, uint a, uint v) {{ wr8(c, a, v); wr8(c, a + 1u, v >> 8u); }}\n\
             static uint rd32(thread Ctx& c, uint a) {{ return rd16(c, a) | (rd16(c, a + 2u) << 16u); }}\n\
             static void wr32(thread Ctx& c, uint a, uint v) {{ wr16(c, a, v); wr16(c, a + 2u, v >> 16u); }}\n",
            cb = CONST_BASE,
            sc = SCRATCH,
            oow = STATUS_OOW,
        );
    }

    fn temp(&mut self) -> String {
        self.tmp += 1;
        format!("t{}", self.tmp)
    }

    fn line(&mut self, ind: usize, s: &str) {
        for _ in 0..ind {
            self.out.push_str("    ");
        }
        self.out.push_str(s);
        self.out.push('\n');
    }

    /// Emit `uint NAME = EXPR;` and return the temp's name — every value is
    /// materialised at its evaluation point, so side effects (calls, traps)
    /// sequence exactly as the interpreter's left-to-right order.
    fn bind(&mut self, ind: usize, expr: &str) -> String {
        let t = self.temp();
        self.line(ind, &format!("uint {t} = {expr};"));
        t
    }

    fn slot_addr(&self, fr: &FrameInfo, slot: usize) -> u16 {
        SCRATCH.wrapping_add(fr.base.wrapping_add(slot as u16).wrapping_mul(2))
    }

    /// Absolute slot-file index of a frame slot (the direct-array fast path —
    /// in bounds by construction: frames partition `0..total_slots`).
    fn slot_idx(&self, fr: &FrameInfo, slot: usize) -> u16 {
        fr.base.wrapping_add(slot as u16)
    }

    fn width_mask(w: Width) -> &'static str {
        if w == Width::Byte {
            "0xFFu"
        } else {
            "0xFFFFu"
        }
    }

    // ── 16-bit expressions (the interpreter's `eval16`, arm by arm) ────────────

    fn e16(&mut self, fr: &FrameInfo, e: &Expr, ind: usize) -> Result<String, String> {
        match e {
            Expr::Lit(n) => Ok(format!("{n}u")),
            Expr::Var(slot) => {
                let i = self.slot_idx(fr, *slot);
                Ok(self.bind(ind, &format!("(uint)c.slots[{i}u]")))
            }
            Expr::Bin(op, l, r, w) => self.bin16(fr, *op, l, r, *w, ind),
            Expr::Index(base, index, w) => {
                let ti = self.e16(fr, index, ind)?;
                let sa = self.slot_addr(fr, *base);
                let addr = format!("{sa}u + (({ti} * 2u) & 0xFFFFu)");
                Ok(match w {
                    Width::Byte => self.bind(ind, &format!("rd8(c, {addr})")),
                    _ => self.bind(ind, &format!("rd16(c, {addr})")),
                })
            }
            Expr::Call(name, args) => {
                let out = self.call(fr, name, args, ind)?;
                Ok(match out {
                    CallVal::Operand(s) => self.bind(ind, &s),
                    // A wide-returning call in 16-bit position leaves its low word.
                    CallVal::Wide => self.bind(ind, "c.rw & 0xFFFFu"),
                    // `.first().unwrap_or(0)` — `rn` guards a void path whose
                    // shared return regs were clobbered by an inner call.
                    CallVal::Narrow => self.bind(ind, "(c.rn > 0u) ? c.r0 : 0u"),
                })
            }
            Expr::Trunc(e) => {
                let t = self.e16(fr, e, ind)?;
                Ok(self.bind(ind, &format!("{t} & 0xFFu")))
            }
            Expr::Peek(addr) => {
                let t = self.e16(fr, addr, ind)?;
                Ok(self.bind(ind, &format!("rd8(c, {t})")))
            }
            // The harness bus answers every port with 0xFF; the port expression
            // still evaluates (for its ordering/traps), as in the interpreter.
            Expr::InPort(port) => {
                let t = self.e16(fr, port, ind)?;
                self.line(ind, &format!("(void){t};"));
                Ok("0xFFu".into())
            }
            Expr::AddrOf(slot) => Ok(format!("{}u", self.slot_addr(fr, *slot))),
            Expr::ConstAddr(name) => self
                .consts
                .get(name.as_str())
                .map(|a| format!("{a}u"))
                .ok_or_else(|| format!("msl: unknown const `{name}`")),
            Expr::Deref(ptr, off) => {
                let tp = self.e16(fr, ptr, ind)?;
                Ok(self.bind(ind, &format!("rd16(c, {tp} + {off}u)")))
            }
            Expr::PtrIndex { ptr, off, index } => {
                let tp = self.e16(fr, ptr, ind)?;
                let ti = self.e16(fr, index, ind)?;
                Ok(self.bind(
                    ind,
                    &format!("rd16(c, {tp} + {off}u + (({ti} * 2u) & 0xFFFFu))"),
                ))
            }
            Expr::MulConst(e, k) => {
                let t = self.e16(fr, e, ind)?;
                Ok(self.bind(ind, &format!("({t} * {k}u) & 0xFFFFu")))
            }
            Expr::LoadAt(addr, w) => {
                let t = self.e16(fr, addr, ind)?;
                Ok(match w {
                    Width::Byte => self.bind(ind, &format!("rd8(c, {t})")),
                    _ => self.bind(ind, &format!("rd16(c, {t})")),
                })
            }
            Expr::Cmp {
                cmp,
                lhs,
                rhs,
                signed,
            } => {
                let tl = self.e16(fr, lhs, ind)?;
                let tr = self.e16(fr, rhs, ind)?;
                let cond = cmp16_text(*cmp, &tl, &tr, *signed);
                Ok(self.bind(ind, &format!("({cond}) ? 1u : 0u")))
            }
            Expr::Logic { and, lhs, rhs } => {
                let tl = self.e16(fr, lhs, ind)?;
                let t = self.temp();
                self.line(ind, &format!("uint {t} = {tl};"));
                // Short-circuit: the left value *is* the result when it decides;
                // the right operand (and any trap inside it) only runs otherwise.
                let need_rhs = if *and {
                    format!("{t} != 0u")
                } else {
                    format!("{t} == 0u")
                };
                self.line(ind, &format!("if ({need_rhs}) {{"));
                let tr = self.e16(fr, rhs, ind + 1)?;
                self.line(ind + 1, &format!("{t} = {tr};"));
                self.line(ind, "}");
                Ok(t)
            }
            Expr::Cmp32 {
                cmp,
                lhs,
                rhs,
                signed,
            } => {
                let tl = self.e32(fr, lhs, ind)?;
                let tr = self.e32(fr, rhs, ind)?;
                let cond = cmp32_text(*cmp, &tl, &tr, *signed);
                Ok(self.bind(ind, &format!("({cond}) ? 1u : 0u")))
            }
            Expr::ShiftVar { left, e, amount, w } => {
                let tv = self.e16(fr, e, ind)?;
                let ta = self.e16(fr, amount, ind)?;
                let tc = self.bind(ind, &format!("{ta} & 0xFFu"));
                let m = Self::width_mask(*w);
                // Per-step semantics folded: n wrapping shifts of 1 ≡ one shift
                // by n with the count guarded past the value's width.
                let expr = if *left {
                    format!("({tc} > 31u) ? 0u : (({tv} << {tc}) & {m})")
                } else if *w == Width::SWord {
                    format!("((uint)(sx16({tv}) >> (int)min({tc}, 15u))) & 0xFFFFu")
                } else {
                    format!("({tc} > 15u) ? 0u : (({tv} >> {tc}) & {m})")
                };
                Ok(self.bind(ind, &expr))
            }
            Expr::Trunc32(e) => {
                let t = self.e32(fr, e, ind)?;
                Ok(self.bind(ind, &format!("{t} & 0xFFFFu")))
            }
            Expr::Halt(code) => {
                let t = self.e16(fr, code, ind)?;
                self.line(ind, &format!("c.trap = {}u;", STATUS_HALT));
                self.line(ind, &format!("c.halt = {t};"));
                self.line(ind, "return;");
                Ok("0u".into())
            }
            Expr::Lit32(_)
            | Expr::Var32(_)
            | Expr::Deref32(..)
            | Expr::Bin32(..)
            | Expr::Shift32 { .. }
            | Expr::Widen(..)
            | Expr::SignExtend(..) => Err("msl: u32 node in a 16-bit context".into()),
        }
    }

    fn bin16(
        &mut self,
        fr: &FrameInfo,
        op: BinOp,
        l: &Expr,
        r: &Expr,
        w: Width,
        ind: usize,
    ) -> Result<String, String> {
        if w == Width::F32 {
            return Err("msl: f32 arithmetic is E4 territory, not lowered yet".into());
        }
        let m = Self::width_mask(w);
        let tl = self.e16(fr, l, ind)?;
        Ok(match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Or | BinOp::And | BinOp::Xor => {
                let tr = self.e16(fr, r, ind)?;
                let sym = match op {
                    BinOp::Add => "+",
                    BinOp::Sub => "-",
                    BinOp::Mul => "*",
                    BinOp::Or => "|",
                    BinOp::And => "&",
                    BinOp::Xor => "^",
                    _ => unreachable!(),
                };
                self.bind(ind, &format!("({tl} {sym} {tr}) & {m}"))
            }
            BinOp::Div | BinOp::Rem => {
                let tr = self.e16(fr, r, ind)?;
                self.line(
                    ind,
                    &format!("if ({tr} == 0u) {{ c.trap = {}u; return; }}", STATUS_DIV0),
                );
                let sym = if matches!(op, BinOp::Div) { "/" } else { "%" };
                if w == Width::SWord {
                    // Truncate toward zero, remainder takes the dividend's sign;
                    // MIN/-1 wraps — safe here because the int32 quotient 32768
                    // re-masks to 0x8000 (rustc's `wrapping_*`).
                    self.bind(
                        ind,
                        &format!("((uint)(sx16({tl}) {sym} sx16({tr}))) & 0xFFFFu"),
                    )
                } else {
                    self.bind(ind, &format!("({tl} {sym} {tr}) & {m}"))
                }
            }
            BinOp::Shl => {
                let k = lit_shift(r)?;
                if k >= 16 {
                    "0u".into()
                } else {
                    self.bind(ind, &format!("({tl} << {k}u) & {m}"))
                }
            }
            BinOp::Shr if w == Width::SWord => {
                // Arithmetic: per-step SRA saturates at the sign fill.
                let k = lit_shift(r)?.min(15);
                self.bind(ind, &format!("((uint)(sx16({tl}) >> {k})) & 0xFFFFu"))
            }
            BinOp::Shr => {
                let k = lit_shift(r)?;
                if k >= 16 {
                    "0u".into()
                } else {
                    self.bind(ind, &format!("({tl} >> {k}u) & {m}"))
                }
            }
        })
    }

    // ── 32-bit expressions (the interpreter's `eval32`) ────────────────────────

    fn e32(&mut self, fr: &FrameInfo, e: &Expr, ind: usize) -> Result<String, String> {
        match e {
            Expr::Lit32(n) => Ok(format!("{n}u")),
            Expr::Var32(slot) => {
                let i = self.slot_idx(fr, *slot);
                let j = i.wrapping_add(1);
                Ok(self.bind(
                    ind,
                    &format!("(uint)c.slots[{i}u] | ((uint)c.slots[{j}u] << 16u)"),
                ))
            }
            Expr::Deref32(ptr, off) => {
                let tp = self.e16(fr, ptr, ind)?;
                Ok(self.bind(ind, &format!("rd32(c, {tp} + {off}u)")))
            }
            // Identity in wide position — the low word is the 16-bit consumer's.
            Expr::Trunc32(e) => self.e32(fr, e, ind),
            Expr::Call(name, args) => match self.call(fr, name, args, ind)? {
                CallVal::Wide => Ok(self.bind(ind, "c.rw")),
                _ => Err("msl: narrow call in a u32 context".into()),
            },
            Expr::Widen(inner) => self.e16(fr, inner, ind),
            Expr::SignExtend(inner) => {
                let t = self.e16(fr, inner, ind)?;
                Ok(self.bind(ind, &format!("(uint)sx16({t})")))
            }
            Expr::Bin32(op, l, r, signed) => {
                let tl = self.e32(fr, l, ind)?;
                let tr = self.e32(fr, r, ind)?;
                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Or | BinOp::And | BinOp::Xor => {
                        let sym = match op {
                            BinOp::Add => "+",
                            BinOp::Sub => "-",
                            BinOp::Mul => "*",
                            BinOp::Or => "|",
                            BinOp::And => "&",
                            BinOp::Xor => "^",
                            _ => unreachable!(),
                        };
                        Ok(self.bind(ind, &format!("{tl} {sym} {tr}")))
                    }
                    BinOp::Div | BinOp::Rem => {
                        self.line(
                            ind,
                            &format!("if ({tr} == 0u) {{ c.trap = {}u; return; }}", STATUS_DIV0),
                        );
                        if *signed {
                            // rustc `wrapping_div`/`wrapping_rem`: MIN/-1 is MIN
                            // (rem 0) — C++ overflows there, so select it out.
                            let sym = if matches!(op, BinOp::Div) { "/" } else { "%" };
                            let min_wrap = if matches!(op, BinOp::Div) {
                                tl.clone()
                            } else {
                                "0u".into()
                            };
                            Ok(self.bind(
                                ind,
                                &format!(
                                    "({tl} == 0x80000000u && {tr} == 0xFFFFFFFFu) ? {min_wrap} \
                                     : (uint)(sx32({tl}) {sym} sx32({tr}))"
                                ),
                            ))
                        } else {
                            let sym = if matches!(op, BinOp::Div) { "/" } else { "%" };
                            Ok(self.bind(ind, &format!("{tl} {sym} {tr}")))
                        }
                    }
                    BinOp::Shl | BinOp::Shr => Err("msl: u32 shifts lower to Shift32".into()),
                }
            }
            Expr::Shift32 { left, e, k, signed } => {
                let tv = self.e32(fr, e, ind)?;
                if *signed && !*left {
                    // Arithmetic: a count ≥ 32 saturates at the sign fill.
                    let k = (*k).min(31);
                    Ok(self.bind(ind, &format!("(uint)(sx32({tv}) >> {k})")))
                } else if *k >= 32 {
                    Ok("0u".into())
                } else if *left {
                    Ok(self.bind(ind, &format!("{tv} << {k}u")))
                } else {
                    Ok(self.bind(ind, &format!("{tv} >> {k}u")))
                }
            }
            _ => Err("msl: not a u32 expression".into()),
        }
    }

    // ── calls ──────────────────────────────────────────────────────────────────

    fn call(
        &mut self,
        fr: &FrameInfo,
        name: &str,
        args: &[Expr],
        ind: usize,
    ) -> Result<CallVal, String> {
        // The bit-method kernels are reserved names lowered as calls — Metal has
        // them native; the u16 semantics ride explicit masks/guards.
        if let Some(text) = match name {
            "__bits_count_ones" => Some("popcount(ARG)".to_string()),
            "__bits_leading_zeros" => Some("(ARG == 0u) ? 16u : (clz(ARG) - 16u)".to_string()),
            "__bits_trailing_zeros" => Some("(ARG == 0u) ? 16u : ctz(ARG)".to_string()),
            _ => None,
        } {
            let t = self.e16(fr, &args[0], ind)?;
            return Ok(CallVal::Operand(text.replace("ARG", &t)));
        }
        let (info, f) = self
            .frames
            .get(name)
            .map(|(i, f)| ((i.base, i.wide_ret), *f))
            .ok_or_else(|| format!("msl: call to unknown fn `{name}`"))?;
        let (callee_base, callee_wide) = info;
        let (wide_param, wide_second) = (f.wide_param, f.wide_second);
        // Evaluate every argument *before* writing any callee slot — an argument
        // that aliases the callee's slots through a pointer reads pre-call values.
        let mut writes: Vec<(u16, bool, String)> = Vec::new(); // (abs slot, wide, temp)
        if wide_second {
            let a0 = self.e32(fr, &args[0], ind)?;
            let a1 = self.e32(fr, &args[1], ind)?;
            let a2 = match args.get(2) {
                Some(e) => Some(self.e16(fr, e, ind)?),
                None => None,
            };
            writes.push((callee_base, true, a0));
            writes.push((callee_base.wrapping_add(2), true, a1));
            if let Some(v) = a2 {
                writes.push((callee_base.wrapping_add(4), false, v));
            }
        } else if wide_param {
            let a0 = self.e32(fr, &args[0], ind)?;
            let a1 = match args.get(1) {
                Some(e) => Some(self.e16(fr, e, ind)?),
                None => None,
            };
            writes.push((callee_base, true, a0));
            if let Some(v) = a1 {
                writes.push((callee_base.wrapping_add(2), false, v));
            }
        } else {
            for (i, e) in args.iter().enumerate() {
                let t = self.e16(fr, e, ind)?;
                writes.push((callee_base.wrapping_add(i as u16), false, t));
            }
        }
        for (slot, wide, t) in writes {
            if wide {
                self.line(ind, &format!("c.slots[{slot}u] = (ushort)({t} & 0xFFFFu);"));
                self.line(
                    ind,
                    &format!("c.slots[{}u] = (ushort)({t} >> 16u);", slot.wrapping_add(1)),
                );
            } else {
                self.line(ind, &format!("c.slots[{slot}u] = (ushort)({t});"));
            }
        }
        self.line(ind, &format!("{}(c);", mangle(name)));
        self.line(ind, "if (c.trap != 0u) { return; }");
        Ok(if callee_wide {
            CallVal::Wide
        } else {
            CallVal::Narrow
        })
    }

    // ── statements (the interpreter's `exec_stmt`) ─────────────────────────────

    fn stmts(&mut self, fr: &FrameInfo, body: &[Stmt], ind: usize) -> Result<(), String> {
        for s in body {
            self.stmt(fr, s, ind)?;
        }
        Ok(())
    }

    fn stmt(&mut self, fr: &FrameInfo, s: &Stmt, ind: usize) -> Result<(), String> {
        match s {
            Stmt::Assign(slot, e) => {
                let t = self.e16(fr, e, ind)?;
                let i = self.slot_idx(fr, *slot);
                self.line(ind, &format!("c.slots[{i}u] = (ushort)({t});"));
            }
            // The low byte always, the high byte only for `Word` — mirrored.
            Stmt::StoreIndex(base, index, value, w) => {
                let tv = self.e16(fr, value, ind)?;
                let ti = self.e16(fr, index, ind)?;
                let sa = self.slot_addr(fr, *base);
                let ta = self.bind(ind, &format!("{sa}u + (({ti} * 2u) & 0xFFFFu)"));
                self.line(ind, &format!("wr8(c, {ta}, {tv});"));
                if *w == Width::Word {
                    self.line(ind, &format!("wr8(c, {ta} + 1u, {tv} >> 8u);"));
                }
            }
            Stmt::Poke(addr, value) => {
                let tv = self.e16(fr, value, ind)?;
                let ta = self.e16(fr, addr, ind)?;
                self.line(ind, &format!("wr8(c, {ta}, {tv});"));
            }
            Stmt::Store(ptr, off, value) => {
                let tv = self.e16(fr, value, ind)?;
                let tp = self.e16(fr, ptr, ind)?;
                self.line(ind, &format!("wr16(c, {tp} + {off}u, {tv});"));
            }
            Stmt::PtrStoreIndex {
                ptr,
                off,
                index,
                value,
            } => {
                let tv = self.e16(fr, value, ind)?;
                let ti = self.e16(fr, index, ind)?;
                let tp = self.e16(fr, ptr, ind)?;
                self.line(
                    ind,
                    &format!("wr16(c, {tp} + {off}u + (({ti} * 2u) & 0xFFFFu), {tv});"),
                );
            }
            Stmt::StoreAt(addr, value, w) => {
                let tv = self.e16(fr, value, ind)?;
                let ta = self.e16(fr, addr, ind)?;
                self.line(ind, &format!("wr8(c, {ta}, {tv});"));
                if *w == Width::Word {
                    self.line(ind, &format!("wr8(c, {ta} + 1u, {tv} >> 8u);"));
                }
            }
            Stmt::Assign32(slot, e) => {
                let t = self.e32(fr, e, ind)?;
                let i = self.slot_idx(fr, *slot);
                let j = i.wrapping_add(1);
                self.line(ind, &format!("c.slots[{i}u] = (ushort)({t} & 0xFFFFu);"));
                self.line(ind, &format!("c.slots[{j}u] = (ushort)({t} >> 16u);"));
            }
            Stmt::Store32(ptr, off, value) => {
                let tv = self.e32(fr, value, ind)?;
                let tp = self.e16(fr, ptr, ind)?;
                self.line(ind, &format!("wr32(c, {tp} + {off}u, {tv});"));
            }
            Stmt::Fill { base, count, value } => {
                if *count > 0 {
                    let tv = self.e16(fr, value, ind)?;
                    let sa = self.slot_addr(fr, *base);
                    let i = self.temp();
                    self.line(
                        ind,
                        &format!("for (uint {i} = 0u; {i} < {count}u; {i}++) {{"),
                    );
                    self.line(
                        ind + 1,
                        &format!("wr16(c, {sa}u + (({i} * 2u) & 0xFFFFu), {tv});"),
                    );
                    self.line(ind, "}");
                }
            }
            Stmt::Eval(e) => {
                let t = self.e16(fr, e, ind)?;
                self.line(ind, &format!("(void){t};"));
            }
            Stmt::AssignTuple(slots, call) => {
                let Expr::Call(name, args) = call else {
                    return Err("msl: AssignTuple of a non-call".into());
                };
                match self.call(fr, name, args, ind)? {
                    CallVal::Wide => return Err("msl: AssignTuple of a wide return".into()),
                    CallVal::Operand(_) => {
                        return Err("msl: AssignTuple of a builtin".into());
                    }
                    CallVal::Narrow => {}
                }
                // zip(slots, produced values): `rn` guards short actual returns.
                for (i, slot) in slots.iter().enumerate().take(3) {
                    let si = self.slot_idx(fr, *slot);
                    self.line(
                        ind,
                        &format!("if (c.rn > {i}u) {{ c.slots[{si}u] = (ushort)c.r{i}; }}"),
                    );
                }
            }
            Stmt::If(cond, then, els) => {
                let tl = self.e16(fr, &cond.lhs, ind)?;
                let tr = self.e16(fr, &cond.rhs, ind)?;
                let c = cmp16_text(cond.cmp, &tl, &tr, cond.signed);
                self.line(ind, &format!("if ({c}) {{"));
                self.stmts(fr, then, ind + 1)?;
                if els.is_empty() {
                    self.line(ind, "}");
                } else {
                    self.line(ind, "} else {");
                    self.stmts(fr, els, ind + 1)?;
                    self.line(ind, "}");
                }
            }
            Stmt::While(..) | Stmt::Loop(..) | Stmt::ForRange { .. } => {
                return Err(
                    "msl: not straight-line — loops (`while`/`loop`/`for`) are E2, \
                     not lowered yet"
                        .into(),
                );
            }
            Stmt::Break | Stmt::Continue => {
                return Err("msl: break/continue outside a loop (E2, not lowered yet)".into());
            }
            Stmt::Return(val) => {
                match val {
                    None => {
                        self.line(ind, "c.rn = 0u;");
                    }
                    Some(e) if fr.wide_ret => {
                        let t = self.e32(fr, e, ind)?;
                        self.line(ind, &format!("c.rw = {t};"));
                    }
                    Some(e) => {
                        let t = self.e16(fr, e, ind)?;
                        self.line(ind, &format!("c.r0 = {t};"));
                        self.line(ind, "c.rn = 1u;");
                    }
                }
                self.line(ind, "return;");
            }
        }
        Ok(())
    }

    // ── functions and the kernel ───────────────────────────────────────────────

    fn gen_fn(&mut self, name: &'a str, f: &'a Func) -> Result<(), String> {
        let info = {
            let (i, _) = &self.frames[name];
            FrameInfo {
                base: i.base,
                wide_ret: i.wide_ret,
                ret_len: i.ret_len,
            }
        };
        let m = mangle(name);
        self.line(0, &format!("static void {m}(thread Ctx& c) {{"));
        // The shared return regs zero at entry so a void path reads as absent.
        self.line(1, "c.r0 = 0u; c.r1 = 0u; c.r2 = 0u; c.rw = 0u; c.rn = 0u;");
        self.stmts(&info, &f.body, 1)
            .map_err(|e| format!("{e} (in fn `{name}`)"))?;
        // Fall-through: the `ret` expressions, evaluated then latched.
        if info.wide_ret {
            let t = self
                .e32(&info, &f.ret[0], 1)
                .map_err(|e| format!("{e} (in fn `{name}`)"))?;
            self.line(1, &format!("c.rw = {t};"));
        } else {
            let mut temps = Vec::new();
            for e in &f.ret {
                temps.push(
                    self.e16(&info, e, 1)
                        .map_err(|e| format!("{e} (in fn `{name}`)"))?,
                );
            }
            for (i, t) in temps.iter().enumerate() {
                self.line(1, &format!("c.r{i} = {t};"));
            }
            self.line(1, &format!("c.rn = {}u;", f.ret.len()));
        }
        self.line(0, "}");
        self.out.push('\n');
        Ok(())
    }

    fn kernel(&mut self, entry: &str, entry_base: u16, entry_fn: &Func) {
        let n_slots = self.total_slots.max(1);
        let m = mangle(entry);
        let o = &mut self.out;
        let _ = writeln!(
            o,
            "kernel void {KERNEL_NAME}(\n\
             \x20   device const ushort* inp [[buffer(0)]],\n\
             \x20   device ushort* outp [[buffer(1)]],\n\
             \x20   device const uchar* cst [[buffer(2)]],\n\
             \x20   uint tid [[thread_position_in_grid]])\n\
             {{\n\
             \x20   ushort slots[{n_slots}] = {{}};\n\
             \x20   Ctx c;\n\
             \x20   c.slots = slots;\n\
             \x20   c.cst = cst;\n\
             \x20   c.trap = 0u; c.halt = 0u;\n\
             \x20   c.r0 = 0u; c.r1 = 0u; c.r2 = 0u; c.rw = 0u; c.rn = 0u;"
        );
        for i in 0..entry_fn.params.min(IN_STRIDE) {
            let slot = entry_base.wrapping_add(i as u16);
            let _ = writeln!(o, "    slots[{slot}u] = inp[tid * {IN_STRIDE}u + {i}u];");
        }
        let _ = writeln!(o, "    {m}(c);");
        if entry_fn.wide_ret {
            let _ = writeln!(
                o,
                "    uint r0 = c.rw & 0xFFFFu;\n\
                 \x20   uint r1 = c.rw >> 16u;\n\
                 \x20   uint r2 = 0u;"
            );
        } else {
            let _ = writeln!(
                o,
                "    uint r0 = (c.rn > 0u) ? c.r0 : 0u;\n\
                 \x20   uint r1 = (c.rn > 1u) ? c.r1 : 0u;\n\
                 \x20   uint r2 = (c.rn > 2u) ? c.r2 : 0u;"
            );
        }
        let _ = writeln!(
            o,
            "    if (c.trap != 0u) {{\n\
             \x20       r0 = (c.trap == {halt}u) ? (c.halt & 0xFFFFu) : 0u;\n\
             \x20       r1 = 0u;\n\
             \x20       r2 = 0u;\n\
             \x20   }}\n\
             \x20   outp[tid * {OUT_STRIDE}u + 0u] = (ushort)r0;\n\
             \x20   outp[tid * {OUT_STRIDE}u + 1u] = (ushort)r1;\n\
             \x20   outp[tid * {OUT_STRIDE}u + 2u] = (ushort)r2;\n\
             \x20   outp[tid * {OUT_STRIDE}u + 3u] = (ushort)c.trap;\n\
             }}",
            halt = STATUS_HALT,
        );
    }
}

/// A call's value shape at the use site.
enum CallVal {
    /// A builtin folded to an operand expression.
    Operand(String),
    /// A narrow (≤3 scalar regs) return in `c.r0..r2` guarded by `c.rn`.
    Narrow,
    /// A wide return in `c.rw`.
    Wide,
}

/// A shift amount operand — literal by construction (the interpreter's rule).
fn lit_shift(e: &Expr) -> Result<u32, String> {
    match e {
        Expr::Lit(k) => Ok(*k as u32),
        _ => Err("msl: shift amount must be a constant".into()),
    }
}

fn cmp16_text(cmp: Cmp, tl: &str, tr: &str, signed: bool) -> String {
    let sym = cmp_sym(cmp);
    if signed && !matches!(cmp, Cmp::Eq | Cmp::Ne) {
        format!("sx16({tl}) {sym} sx16({tr})")
    } else {
        format!("{tl} {sym} {tr}")
    }
}

fn cmp32_text(cmp: Cmp, tl: &str, tr: &str, signed: bool) -> String {
    let sym = cmp_sym(cmp);
    if signed {
        format!("sx32({tl}) {sym} sx32({tr})")
    } else {
        format!("{tl} {sym} {tr}")
    }
}

fn cmp_sym(cmp: Cmp) -> &'static str {
    match cmp {
        Cmp::Lt => "<",
        Cmp::Le => "<=",
        Cmp::Gt => ">",
        Cmp::Ge => ">=",
        Cmp::Eq => "==",
        Cmp::Ne => "!=",
    }
}
