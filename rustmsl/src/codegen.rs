//! MSL codegen (Phase 6 WS-E, E1+E2): lowering of the `cell80-core` typed IR to
//! Metal Shading Language — straight-line integer cells (E1) plus loops and
//! branches (E2: `while`/`loop`/`for`, `break`/`continue`), budget-bounded by a
//! per-thread fuel counter that mirrors the interpreter's `tick()` placement
//! **exactly** (one step per statement, per expression node, per loop
//! iteration). Each thread reports its step count, so batteries assert
//! IR-step parity — the canonical family cost (docs 14, Q2) — alongside value
//! parity, and a runaway loop is a counted trap ([`STATUS_FUEL`]), never a hung
//! dispatch.
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
//!
//! **The batch layouts (E3)** share one kernel shape: the grid is
//! `n_cells × n_inputs`, a thread routes `tid → (cell, input)`, and each cell's
//! window (its const-blob slice, its slot-file length) is selected per thread.
//! [`compile`] emits a one-cell module (the fuzzing/reward layout);
//! [`compile_library`] fuses many cells into one translation unit (the
//! library × probe-set layout — retrieval by execution's substrate, WS-F).
//!
//! `continue` inside a `for` must reach the induction step (the interpreter's
//! `Flow::Continue` lands on the step, and MSL has no `goto`) — so a `for`
//! body is wrapped in `do { … } while(false)`: C's `continue` exits the wrapper
//! into the step, and `break` sets a flag the wrapper checks to leave the loop.

use cell80_core::ir::*;
use std::collections::HashMap;
use std::fmt::Write as _;

/// Where const data lays in the window (the interpreter's convention).
pub const CONST_BASE: u16 = 0x8000;
/// The slot file's window base (the family scratch region).
pub const SCRATCH: u16 = 0x9000;
/// Where a state cell's struct lays in the window (`cell80::STATE_BASE`) —
/// per thread, loaded from `buffer(4)` and written back to `buffer(5)`.
pub const STATE_BASE: u16 = 0xB000;

/// Per-thread run status: clean.
pub const STATUS_OK: u16 = 0;
/// Per-thread run status: divide by zero (the interpreter's refusal, not a value).
pub const STATUS_DIV0: u16 = 1;
/// Per-thread run status: `halt(code)` — the code rides result register 0.
pub const STATUS_HALT: u16 = 2;
/// Per-thread run status: a write outside the mapped window regions.
pub const STATUS_OOW: u16 = 3;
/// Per-thread run status: fuel exhausted (the interpreter's runaway-loop guard,
/// same budget, same tick placement).
pub const STATUS_FUEL: u16 = 4;

/// The execution budget — the interpreter's `FUEL`, per thread.
pub const FUEL: u32 = 100_000_000;

/// The emitted kernel's function name.
pub const KERNEL_NAME: &str = "cell_main";

/// Inputs consumed per thread (the `HL`/`DE`/`BC` register-arg convention).
pub const IN_STRIDE: usize = 3;
/// Outputs produced per thread: `r0 r1 r2 status steps_lo steps_hi`.
pub const OUT_STRIDE: usize = 6;

/// One cell's shape inside a compiled module.
#[derive(Clone, Debug)]
pub struct CellMeta {
    /// The IR entry function this cell's case wraps.
    pub entry: String,
    /// Entry parameter slots consumed from the input triple (≤ 3). For a
    /// state cell, param 0 is the `&mut self` pointer ([`STATE_BASE`], not an
    /// input word) and the input triple feeds params 1…
    pub params: usize,
    /// Result registers the entry produces (a wide return is 2: low, high).
    pub ret_regs: usize,
    pub wide_ret: bool,
    /// The state struct's byte length at [`STATE_BASE`] (0 for a value cell).
    /// Each thread gets a private state window loaded from `buffer(4)` and
    /// written back to `buffer(5)` — the typed-state I/O surface.
    pub state_len: usize,
}

/// A compiled GPU module: one translation unit (one kernel, [`KERNEL_NAME`])
/// in the requested [`Dialect`] over one or more cells, the concatenated
/// const blob to bind read-only (MSL `buffer(2)` / the CUDA `cst` param),
/// and each cell's shape. The output grid is cell-major: thread
/// `cell · n_inputs + input` writes quad `[r0, r1, r2, status, steps_lo,
/// steps_hi]`. The struct is dialect-neutral — only `source`'s text differs.
#[derive(Clone, Debug)]
pub struct GpuModule {
    /// Which surface syntax `source` is in — executors refuse a module
    /// compiled for the other runtime with a typed error, not a compiler one.
    pub dialect: Dialect,
    pub source: String,
    pub consts: Vec<u8>,
    pub cells: Vec<CellMeta>,
}

/// One cell's compile input for [`compile_library`]: lowered+pruned functions,
/// its const pool, its entry name, and (for a state cell) the state struct's
/// byte length at [`STATE_BASE`] — 0 for a value cell.
pub struct LibraryCell<'a> {
    pub funcs: &'a [(String, Func)],
    pub consts: &'a [(String, Vec<u8>)],
    pub entry: &'a str,
    pub state_len: usize,
}

/// The emitted GPU dialect. One walker emits both: every tick placement,
/// mask, guard, and trap is shared text; only surface syntax (headers,
/// address-space qualifiers, kernel signatures, intrinsic spellings)
/// dialects. The set is closed by design — WGSL/ROCm are out of scope.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dialect {
    Msl,
    Cuda,
}

/// Compile one **value** cell (the one-cell × N-inputs layout). Refuses (with
/// a typed error) anything outside the E1+E2 fragment: recursion,
/// ports-by-policy, f32 (E4). State cells go through [`compile_library`] with
/// their `state_len`.
pub fn compile(
    funcs: &[(String, Func)],
    consts: &[(String, Vec<u8>)],
    entry: &str,
) -> Result<GpuModule, String> {
    compile_library(&[LibraryCell {
        funcs,
        consts,
        entry,
        state_len: 0,
    }])
}

/// Fuse many cells into one translation unit — the library × probe-set layout
/// (E3): one launch runs every cell against every input triple.
pub fn compile_library(cells: &[LibraryCell]) -> Result<GpuModule, String> {
    compile_library_dialect(Dialect::Msl, cells)
}

/// [`compile`] in the CUDA dialect — same walker, same tick placement, CUDA
/// surface syntax. Run the result with `CudaBatch` (the `cuda` feature).
pub fn compile_cuda(
    funcs: &[(String, Func)],
    consts: &[(String, Vec<u8>)],
    entry: &str,
) -> Result<GpuModule, String> {
    compile_library_cuda(&[LibraryCell {
        funcs,
        consts,
        entry,
        state_len: 0,
    }])
}

/// [`compile_library`] in the CUDA dialect.
pub fn compile_library_cuda(cells: &[LibraryCell]) -> Result<GpuModule, String> {
    compile_library_dialect(Dialect::Cuda, cells)
}

fn compile_library_dialect(dialect: Dialect, cells: &[LibraryCell]) -> Result<GpuModule, String> {
    if cells.is_empty() {
        return Err("msl: empty library".into());
    }
    let mut g = Gen {
        dialect,
        frames: HashMap::new(),
        consts: HashMap::new(),
        prefix: String::new(),
        tmp: 0,
        loops: Vec::new(),
        out: String::new(),
    };

    let mut blob = Vec::new();
    let mut metas = Vec::new();
    let mut max_slots = 1u16;
    let mut max_state = 0usize;
    // Per-cell: (const buffer offset, const len, slot bytes, entry base,
    // cumulative state bytes before this cell, meta).
    let mut cases: Vec<(usize, usize, u32, u16, usize, CellMeta)> = Vec::new();
    let mut state_cum = 0usize;

    for (ci, cell) in cells.iter().enumerate() {
        let tag = |e: String| {
            if cells.len() == 1 {
                e
            } else {
                format!("{e} (cell `{}`)", cell.entry)
            }
        };
        if let Some(cycle) = cell80_core::dce::find_recursion(cell.funcs) {
            return Err(tag(format!("msl: recursion is not lowered: {cycle}")));
        }
        // Frames laid in `funcs` order with a running base — the interpreter's
        // (and every sibling backend's) slot-assignment rule, so addresses agree.
        g.frames.clear();
        g.consts.clear();
        g.prefix = format!("c{ci}_");
        let mut base = 0u16;
        for (name, f) in cell.funcs {
            g.frames.insert(
                name.clone(),
                FrameInfo {
                    base,
                    wide_ret: f.wide_ret,
                    wide_param: f.wide_param,
                    wide_second: f.wide_second,
                },
            );
            base = base.wrapping_add(f.n_locals as u16);
        }
        max_slots = max_slots.max(base);
        let cst_off = blob.len();
        let mut at = CONST_BASE;
        for (name, bytes) in cell.consts {
            g.consts.insert(name.clone(), at);
            blob.extend_from_slice(bytes);
            at = at.wrapping_add(bytes.len() as u16);
        }
        let cst_len = blob.len() - cst_off;
        if CONST_BASE as usize + cst_len > SCRATCH as usize {
            return Err(tag("msl: const data overflows into the slot file".into()));
        }
        let entry_fn = cell
            .funcs
            .iter()
            .find(|(n, _)| n == cell.entry)
            .map(|(_, f)| f)
            .ok_or_else(|| tag(format!("msl: unknown entry `{}`", cell.entry)))?;
        let entry_base = g.frames[cell.entry].base;

        for (name, _) in cell.funcs {
            let tref = g.dialect.thread_ref();
            let _ = writeln!(g.out, "static CELLFN void {}({tref} c);", g.mangle(name));
        }
        for (name, f) in cell.funcs {
            g.gen_fn(name, f).map_err(&tag)?;
        }
        if STATE_BASE as usize + cell.state_len > 0x1_0000 {
            return Err(tag("msl: state struct overflows the window".into()));
        }
        max_state = max_state.max(cell.state_len);
        let meta = CellMeta {
            entry: cell.entry.to_string(),
            // A state cell's param 0 is the `&mut self` pointer; the input
            // triple only feeds what follows it.
            params: entry_fn
                .params
                .min(IN_STRIDE + usize::from(cell.state_len > 0)),
            ret_regs: if entry_fn.wide_ret {
                2
            } else {
                entry_fn.ret.len()
            },
            wide_ret: entry_fn.wide_ret,
            state_len: cell.state_len,
        };
        cases.push((
            cst_off,
            cst_len,
            base as u32 * 2,
            entry_base,
            state_cum,
            meta.clone(),
        ));
        state_cum += cell.state_len;
        metas.push(meta);
    }
    // The prelude needs the fused slot-file and state-window sizes (the Ctx
    // array members), so it prepends after every cell has been walked.
    let funcs_text = std::mem::take(&mut g.out);
    g.prelude(max_slots, max_state);
    g.out.push_str(&funcs_text);
    g.kernel(&cases);
    Ok(GpuModule {
        dialect,
        source: g.out,
        consts: blob,
        cells: metas,
    })
}

struct FrameInfo {
    base: u16,
    wide_ret: bool,
    wide_param: bool,
    wide_second: bool,
}

/// The dialect points — every place the two outputs differ. Everything the
/// walker emits between these (ticks, masks, guards, traps, the whole kernel
/// body) is shared text, so a semantics fix lands in both dialects by
/// construction.
impl Dialect {
    /// Translation-unit header: includes/`using` for MSL; scalar typedefs
    /// (the walker's `uint`/`ushort`/`uchar` vocabulary) for CUDA — NVRTC
    /// includes nothing by default. CUDA's `min` is a macro: a function named
    /// `min` could collide with a builtin, a macro can neither collide nor be
    /// missing (the runtime-shift clamp is its only use, side-effect-free
    /// temps both times).
    fn header(self) -> &'static str {
        match self {
            Dialect::Msl => "#include <metal_stdlib>\nusing namespace metal;",
            Dialect::Cuda => {
                "typedef unsigned int uint;\n\
                 typedef unsigned short ushort;\n\
                 typedef unsigned char uchar;\n\
                 #define min(a, b) (((a) < (b)) ? (a) : (b))"
            }
        }
    }

    /// The read-only const-blob pointer's type inside `Ctx`.
    fn dev_const_ptr(self) -> &'static str {
        match self {
            Dialect::Msl => "device const uchar*",
            Dialect::Cuda => "const uchar*",
        }
    }

    /// The attribute `CELLFN` expands to — cell functions are pinned noinline
    /// on both targets (the shipped configuration is the battery-validated
    /// one, not an inliner heuristic's; on CUDA it also caps NVRTC's appetite
    /// on the fused megakernel).
    fn cellfn_attr(self) -> &'static str {
        match self {
            Dialect::Msl => "__attribute__((noinline))",
            Dialect::Cuda => "__device__ __noinline__",
        }
    }

    /// The two sign-extension helpers (`sx16`/`sx32`), whole lines — MSL
    /// bit-casts via `as_type`, CUDA via value-preserving two's-complement
    /// integer conversions (defined behavior under nvcc/NVRTC).
    fn sx_helpers(self) -> &'static str {
        match self {
            Dialect::Msl => {
                "static int sx16(uint x) { return (int)as_type<short>((ushort)(x)); }\n\
                 static int sx32(uint x) { return as_type<int>(x); }"
            }
            Dialect::Cuda => {
                "static __device__ int sx16(uint x) { return (int)(short)(ushort)(x); }\n\
                 static __device__ int sx32(uint x) { return (int)(x); }"
            }
        }
    }

    /// Prefix for the opaque noinline division helpers. The Metal
    /// branch-inversion miscompile is not assumed to transfer to CUDA — the
    /// helpers stay noinline there for uniformity (the shipped configuration
    /// is the battery-validated one on both targets), and any CUDA-specific
    /// quirk the cloud battery finds lands in this dialect's arms only.
    fn noinline_static(self) -> &'static str {
        match self {
            Dialect::Msl => "static __attribute__((noinline))",
            Dialect::Cuda => "static __device__ __noinline__",
        }
    }

    /// Prefix for the plain (inlinable) window helpers `rd8`/`wr8`/….
    fn static_fn(self) -> &'static str {
        match self {
            Dialect::Msl => "static",
            Dialect::Cuda => "static __device__",
        }
    }

    /// The by-reference `Ctx` parameter type.
    fn thread_ref(self) -> &'static str {
        match self {
            Dialect::Msl => "thread Ctx&",
            Dialect::Cuda => "Ctx&",
        }
    }

    /// A `__bits_*` intrinsic's expression template over `ARG` (a zero-
    /// extended u16 in a `uint` temp — so CUDA's 32-bit `__clz` minus 16 is
    /// exactly MSL's per-width `clz`). The zero guards pin the 16-bit answer
    /// on both targets; `__ffs` is 1-based with `__ffs(0) == 0`, but the zero
    /// case is guarded, so `- 1` is exact.
    fn bits_intrinsic(self, name: &str) -> Option<&'static str> {
        match self {
            Dialect::Msl => match name {
                "__bits_count_ones" => Some("popcount(ARG)"),
                "__bits_leading_zeros" => Some("(ARG == 0u) ? 16u : (clz(ARG) - 16u)"),
                "__bits_trailing_zeros" => Some("(ARG == 0u) ? 16u : ctz(ARG)"),
                _ => None,
            },
            Dialect::Cuda => match name {
                "__bits_count_ones" => Some("(uint)__popc((int)(ARG))"),
                "__bits_leading_zeros" => {
                    Some("(ARG == 0u) ? 16u : (uint)(__clz((int)(ARG)) - 16)")
                }
                "__bits_trailing_zeros" => {
                    Some("(ARG == 0u) ? 16u : (uint)(__ffs((int)(ARG)) - 1)")
                }
                _ => None,
            },
        }
    }
}

struct Gen {
    dialect: Dialect,
    frames: HashMap<String, FrameInfo>,
    consts: HashMap<String, u16>,
    /// Per-cell function-name prefix (`c0_`, `c1_`, …) — cells share one
    /// translation unit, and every cell defines `run`.
    prefix: String,
    tmp: usize,
    /// The enclosing-loop stack: `None` for `while`/`loop` (C `break`/
    /// `continue` bind directly), `Some(flag)` for a `for` body's
    /// do-while(false) wrapper (`break` sets the flag; `continue` falls to the
    /// induction step).
    loops: Vec<Option<String>>,
    out: String,
}

impl Gen {
    fn prelude(&mut self, n_slots: u16, n_state: usize) {
        let n_slots = n_slots.max(1);
        let n_state = n_state.max(1);
        let header = self.dialect.header();
        let devc = self.dialect.dev_const_ptr();
        let cellfn = self.dialect.cellfn_attr();
        let sx = self.dialect.sx_helpers();
        let ni = self.dialect.noinline_static();
        let sfn = self.dialect.static_fn();
        let tref = self.dialect.thread_ref();
        let o = &mut self.out;
        // The slot file is an array *member*, not a thread pointer: passing a
        // struct holding a `thread ushort*` into non-inlined functions
        // miscompiled on Metal (branch inversion once the fused unit grew past
        // the inliner's appetite — caught by the E3 megakernel battery). A
        // member array keeps the provenance visible and is correct whether or
        // not the compiler inlines.
        let _ = writeln!(
            o,
            "// generated by rustmsl — do not edit; semantics are the cell80-core interpreter's\n\
             {header}\n\
             \n\
             struct Ctx {{\n\
             \x20   ushort slots[{n_slots}];\n\
             \x20   uchar state[{n_state}];\n\
             \x20   {devc} cst;\n\
             \x20   uint cst_len;\n\
             \x20   uint slot_bytes;\n\
             \x20   uint state_len;\n\
             \x20   uint trap;\n\
             \x20   uint halt;\n\
             \x20   uint fuel;\n\
             \x20   uint r0; uint r1; uint r2;\n\
             \x20   uint rw;\n\
             \x20   uint rn;\n\
             }};\n\
             \n\
             // The interpreter's tick(): one unit per statement, per expression\n\
             // node, per loop iteration; exhaustion is a trap, never a hang.\n\
             #define TICK if (--c.fuel == 0u) {{ c.trap = {fuel_trap}u; return; }}\n\
             // Cell functions are pinned noinline: the shipped configuration is\n\
             // exactly the battery-validated one, not an inliner heuristic's.\n\
             #define CELLFN {cellfn}\n\
             \n\
             // Sign-extend a masked 16-bit lane / bit-cast a 32-bit lane to signed.\n\
             {sx}\n\
             \n\
             // Division rides opaque value-taking helpers: Metal's backend\n\
             // miscompiles a divide feeding a branch that guards stores through\n\
             // a thread-reference param (branch polarity inverts — caught by the\n\
             // E3 megakernel battery on `mul_sat`, minimised to a 10-line repro).\n\
             // The call boundary blocks the faulty fusion; zero-checks stay at\n\
             // the call site (the trap), and the signed MIN/-1 wrap lives here.\n\
             {ni} uint udiv(uint a, uint b) {{ return a / b; }}\n\
             {ni} uint urem(uint a, uint b) {{ return a % b; }}\n\
             {ni} uint sdiv16(uint a, uint b) {{ return ((uint)(sx16(a) / sx16(b))) & 0xFFFFu; }}\n\
             {ni} uint srem16(uint a, uint b) {{ return ((uint)(sx16(a) % sx16(b))) & 0xFFFFu; }}\n\
             {ni} uint sdiv32(uint a, uint b) {{ return (a == 0x80000000u && b == 0xFFFFFFFFu) ? a : (uint)(sx32(a) / sx32(b)); }}\n\
             {ni} uint srem32(uint a, uint b) {{ return (a == 0x80000000u && b == 0xFFFFFFFFu) ? 0u : (uint)(sx32(a) % sx32(b)); }}\n\
             \n\
             // Byte-routed window emulation: consts (read-only), the slot file,\n\
             // the state struct, else zero on read and a trap on write (the\n\
             // pre-registered E1 weakening).\n\
             {sfn} uint rd8({tref} c, uint a) {{\n\
             \x20   a &= 0xFFFFu;\n\
             \x20   if (a >= 0x{cb:X}u && a < 0x{cb:X}u + c.cst_len) return (uint)c.cst[a - 0x{cb:X}u];\n\
             \x20   if (a >= 0x{sc:X}u && a < 0x{sc:X}u + c.slot_bytes) {{\n\
             \x20       uint o = a - 0x{sc:X}u;\n\
             \x20       return ((uint)c.slots[o >> 1] >> ((o & 1u) * 8u)) & 0xFFu;\n\
             \x20   }}\n\
             \x20   if (a >= 0x{sb:X}u && a < 0x{sb:X}u + c.state_len) return (uint)c.state[a - 0x{sb:X}u];\n\
             \x20   return 0u;\n\
             }}\n\
             {sfn} void wr8({tref} c, uint a, uint v) {{\n\
             \x20   a &= 0xFFFFu;\n\
             \x20   if (a >= 0x{sc:X}u && a < 0x{sc:X}u + c.slot_bytes) {{\n\
             \x20       uint o = a - 0x{sc:X}u;\n\
             \x20       uint i = o >> 1;\n\
             \x20       uint sh = (o & 1u) * 8u;\n\
             \x20       c.slots[i] = (ushort)(((uint)c.slots[i] & ~(0xFFu << sh)) | ((v & 0xFFu) << sh));\n\
             \x20       return;\n\
             \x20   }}\n\
             \x20   if (a >= 0x{sb:X}u && a < 0x{sb:X}u + c.state_len) {{\n\
             \x20       c.state[a - 0x{sb:X}u] = (uchar)(v & 0xFFu);\n\
             \x20       return;\n\
             \x20   }}\n\
             \x20   c.trap = {oow}u;\n\
             }}\n\
             {sfn} uint rd16({tref} c, uint a) {{ return rd8(c, a) | (rd8(c, a + 1u) << 8u); }}\n\
             {sfn} void wr16({tref} c, uint a, uint v) {{ wr8(c, a, v); wr8(c, a + 1u, v >> 8u); }}\n\
             {sfn} uint rd32({tref} c, uint a) {{ return rd16(c, a) | (rd16(c, a + 2u) << 16u); }}\n\
             {sfn} void wr32({tref} c, uint a, uint v) {{ wr16(c, a, v); wr16(c, a + 2u, v >> 16u); }}\n",
            cb = CONST_BASE,
            sc = SCRATCH,
            sb = STATE_BASE,
            oow = STATUS_OOW,
            fuel_trap = STATUS_FUEL,
        );
    }

    /// A function's MSL name: cell prefix + `f_` + the IR name, non-identifier
    /// chars folded to `_`.
    fn mangle(&self, name: &str) -> String {
        let body: String = name
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
            .collect();
        format!("{}f_{body}", self.prefix)
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

    /// The interpreter's `tick()` at this point in the emission.
    fn tick(&mut self, ind: usize) {
        self.line(ind, "TICK");
    }

    /// Emit `uint NAME = EXPR;` and return the temp's name — every value is
    /// materialised at its evaluation point, so side effects (calls, traps)
    /// sequence exactly as the interpreter's left-to-right order.
    fn bind(&mut self, ind: usize, expr: &str) -> String {
        let t = self.temp();
        self.line(ind, &format!("uint {t} = {expr};"));
        t
    }

    fn frame(&self, name: &str) -> &FrameInfo {
        &self.frames[name]
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
        // eval16 ticks on entry for every node, literals included.
        self.tick(ind);
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
                // Truncation toward zero and the dividend's-sign remainder live
                // in the helpers (MIN/-1 wraps via the int32 quotient re-mask).
                if w == Width::SWord {
                    let f = if matches!(op, BinOp::Div) {
                        "sdiv16"
                    } else {
                        "srem16"
                    };
                    self.bind(ind, &format!("{f}({tl}, {tr})"))
                } else {
                    let f = if matches!(op, BinOp::Div) {
                        "udiv"
                    } else {
                        "urem"
                    };
                    self.bind(ind, &format!("{f}({tl}, {tr}) & {m}"))
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
        // eval32 ticks on entry for every node.
        self.tick(ind);
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
                        // rustc `wrapping_div`/`wrapping_rem` semantics live in
                        // the helpers (MIN/-1 selected out — C++ overflows there).
                        let f = match (op, *signed) {
                            (BinOp::Div, true) => "sdiv32",
                            (BinOp::Rem, true) => "srem32",
                            (BinOp::Div, false) => "udiv",
                            (BinOp::Rem, false) => "urem",
                            _ => unreachable!(),
                        };
                        Ok(self.bind(ind, &format!("{f}({tl}, {tr})")))
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
        // The bit-method kernels are reserved names lowered as calls — both
        // targets have them native; the u16 semantics ride explicit
        // masks/guards in the dialect's template.
        if let Some(text) = self.dialect.bits_intrinsic(name) {
            let t = self.e16(fr, &args[0], ind)?;
            return Ok(CallVal::Operand(text.replace("ARG", &t)));
        }
        let (callee_base, callee_wide, wide_param, wide_second) = {
            let f = self
                .frames
                .get(name)
                .ok_or_else(|| format!("msl: call to unknown fn `{name}`"))?;
            (f.base, f.wide_ret, f.wide_param, f.wide_second)
        };
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
        let m = self.mangle(name);
        self.line(ind, &format!("{m}(c);"));
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
        // exec_stmt ticks on entry for every statement.
        self.tick(ind);
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
            Stmt::While(cond, body) => {
                self.line(ind, "for (;;) {");
                // One tick per iteration (the interpreter's loop-top tick),
                // then the condition re-evaluates.
                self.tick(ind + 1);
                let tl = self.e16(fr, &cond.lhs, ind + 1)?;
                let tr = self.e16(fr, &cond.rhs, ind + 1)?;
                let c = cmp16_text(cond.cmp, &tl, &tr, cond.signed);
                self.line(ind + 1, &format!("if (!({c})) break;"));
                self.loops.push(None);
                self.stmts(fr, body, ind + 1)?;
                self.loops.pop();
                self.line(ind, "}");
            }
            Stmt::Loop(body) => {
                self.line(ind, "for (;;) {");
                self.tick(ind + 1);
                self.loops.push(None);
                self.stmts(fr, body, ind + 1)?;
                self.loops.pop();
                self.line(ind, "}");
            }
            Stmt::ForRange {
                var,
                end,
                inclusive,
                width,
                body,
            } => {
                let vi = self.slot_idx(fr, *var);
                self.line(ind, "for (;;) {");
                self.tick(ind + 1);
                // The bound re-evaluates every iteration (it lives in a temp
                // slot the loop header reads), the variable re-reads its slot.
                let tv = self.bind(ind + 1, &format!("(uint)c.slots[{vi}u]"));
                let tb = self.e16(fr, end, ind + 1)?;
                let keep = {
                    let cmp = if *inclusive { "<=" } else { "<" };
                    if *width == Width::SWord {
                        format!("sx16({tv}) {cmp} sx16({tb})")
                    } else {
                        format!("{tv} {cmp} {tb}")
                    }
                };
                self.line(ind + 1, &format!("if (!({keep})) break;"));
                // The body rides a do-while(false) wrapper: C `continue` exits
                // it into the induction step (the interpreter's continue
                // target); `break` raises a flag the wrapper turns into a real
                // loop exit. MSL has no `goto` to do this more directly.
                let brk = self.temp();
                self.line(ind + 1, &format!("uint {brk} = 0u;"));
                self.line(ind + 1, "do {");
                self.loops.push(Some(brk.clone()));
                self.stmts(fr, body, ind + 2)?;
                self.loops.pop();
                self.line(ind + 1, "} while (false);");
                self.line(ind + 1, &format!("if ({brk} != 0u) break;"));
                // The induction step re-reads the slot (the body may assign the
                // loop variable) and masks to the variable's width.
                let mask = if *width == Width::Byte {
                    "0xFFu"
                } else {
                    "0xFFFFu"
                };
                self.line(
                    ind + 1,
                    &format!("c.slots[{vi}u] = (ushort)(((uint)c.slots[{vi}u] + 1u) & {mask});"),
                );
                self.line(ind, "}");
            }
            Stmt::Break => match self.loops.last() {
                Some(Some(brk)) => {
                    let brk = brk.clone();
                    self.line(ind, &format!("{brk} = 1u;"));
                    self.line(ind, "break;");
                }
                Some(None) => self.line(ind, "break;"),
                None => return Err("msl: break outside a loop".into()),
            },
            Stmt::Continue => match self.loops.last() {
                // Inside a for body's wrapper, C `continue` exits the do-while
                // into the induction step; in while/loop it re-enters at the
                // iteration tick — both are the interpreter's continue target.
                Some(_) => self.line(ind, "continue;"),
                None => return Err("msl: continue outside a loop".into()),
            },
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

    fn gen_fn(&mut self, name: &str, f: &Func) -> Result<(), String> {
        let info = {
            let i = self.frame(name);
            FrameInfo {
                base: i.base,
                wide_ret: i.wide_ret,
                wide_param: i.wide_param,
                wide_second: i.wide_second,
            }
        };
        let m = self.mangle(name);
        let tref = self.dialect.thread_ref();
        self.line(0, &format!("static CELLFN void {m}({tref} c) {{"));
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

    /// The kernel: grid `n_cells × n_inputs`, cell-major; each case selects the
    /// cell's window (const slice, slot-file length, state slice), loads its
    /// args (a state cell's param 0 is the `&mut self` pointer), runs its
    /// entry, latches its result shape, and writes its state back.
    fn kernel(&mut self, cases: &[(usize, usize, u32, u16, usize, CellMeta)]) {
        let dialect = self.dialect;
        let o = &mut self.out;
        // The signature (and, on CUDA, the launch-index computation with its
        // grid-tail guard — Metal's exact dispatch has no tail) is the only
        // dialected part of the kernel; the dispatch body below is shared.
        match dialect {
            Dialect::Msl => {
                let _ = writeln!(
                    o,
                    "kernel void {KERNEL_NAME}(\n\
                     \x20   device const ushort* inp [[buffer(0)]],\n\
                     \x20   device ushort* outp [[buffer(1)]],\n\
                     \x20   device const uchar* cst [[buffer(2)]],\n\
                     \x20   constant uint& n_inputs [[buffer(3)]],\n\
                     \x20   device const uchar* stin [[buffer(4)]],\n\
                     \x20   device uchar* stout [[buffer(5)]],\n\
                     \x20   uint tid [[thread_position_in_grid]])\n\
                     {{"
                );
            }
            Dialect::Cuda => {
                // `n_cells` is a compile-time constant, so the grid-tail
                // guard needs no extra kernel parameter. The product fits
                // u32: Metal's grid shares the same practical bound.
                let n_cells = cases.len();
                let _ = writeln!(
                    o,
                    "extern \"C\" __global__ void {KERNEL_NAME}(\n\
                     \x20   const ushort* __restrict__ inp,\n\
                     \x20   ushort* __restrict__ outp,\n\
                     \x20   const uchar* __restrict__ cst,\n\
                     \x20   uint n_inputs,\n\
                     \x20   const uchar* __restrict__ stin,\n\
                     \x20   uchar* stout)\n\
                     {{\n\
                     \x20   uint tid = blockIdx.x * blockDim.x + threadIdx.x;\n\
                     \x20   if (tid >= {n_cells}u * n_inputs) return;"
                );
            }
        }
        let _ = writeln!(
            o,
            "\x20   uint cell = tid / n_inputs;\n\
             \x20   uint idx = tid % n_inputs;\n\
             \x20   Ctx c = {{}};\n\
             \x20   c.fuel = {FUEL}u;\n\
             \x20   uint r0 = 0u; uint r1 = 0u; uint r2 = 0u;\n\
             \x20   switch (cell) {{"
        );
        for (ci, (cst_off, cst_len, slot_bytes, entry_base, state_cum, meta)) in
            cases.iter().enumerate()
        {
            let _ = writeln!(
                o,
                "    case {ci}u: {{\n\
                 \x20       c.cst = cst + {cst_off}u;\n\
                 \x20       c.cst_len = {cst_len}u;\n\
                 \x20       c.slot_bytes = {slot_bytes}u;"
            );
            let sl = meta.state_len;
            if sl > 0 {
                // The state slice is cell-major like the outputs: this cell's
                // blocks start after every prior cell's `n_inputs` blocks.
                let _ = writeln!(
                    o,
                    "        c.state_len = {sl}u;\n\
                     \x20       for (uint si = 0u; si < {sl}u; si++) {{\n\
                     \x20           c.state[si] = stin[{state_cum}u * n_inputs + idx * {sl}u + si];\n\
                     \x20       }}"
                );
            }
            let self_param = usize::from(sl > 0);
            if self_param == 1 {
                let slot = *entry_base;
                let _ = writeln!(o, "        c.slots[{slot}u] = {}u;", STATE_BASE);
            }
            for i in self_param..meta.params {
                let slot = entry_base.wrapping_add(i as u16);
                let w = i - self_param;
                let _ = writeln!(
                    o,
                    "        c.slots[{slot}u] = inp[idx * {IN_STRIDE}u + {w}u];"
                );
            }
            // The per-cell prefix is positional, mirroring compile_library.
            let entry: String = meta
                .entry
                .chars()
                .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
                .collect();
            let _ = writeln!(o, "        c{ci}_f_{entry}(c);");
            if meta.wide_ret {
                let _ = writeln!(
                    o,
                    "        r0 = c.rw & 0xFFFFu;\n\
                     \x20       r1 = c.rw >> 16u;"
                );
            } else {
                let _ = writeln!(
                    o,
                    "        r0 = (c.rn > 0u) ? c.r0 : 0u;\n\
                     \x20       r1 = (c.rn > 1u) ? c.r1 : 0u;\n\
                     \x20       r2 = (c.rn > 2u) ? c.r2 : 0u;"
                );
            }
            if sl > 0 {
                // Written back even on a trap: the interpreter's memory at the
                // trap point is observable, and the tick placement is identical
                // on both substrates, so the mutation point is too.
                let _ = writeln!(
                    o,
                    "        for (uint so = 0u; so < {sl}u; so++) {{\n\
                     \x20           stout[{state_cum}u * n_inputs + idx * {sl}u + so] = c.state[so];\n\
                     \x20       }}"
                );
            }
            let _ = writeln!(o, "        break; }}");
        }
        let _ = writeln!(
            o,
            "    }}\n\
             \x20   uint steps = {FUEL}u - c.fuel;\n\
             \x20   if (c.trap != 0u) {{\n\
             \x20       r0 = (c.trap == {halt}u) ? (c.halt & 0xFFFFu) : 0u;\n\
             \x20       r1 = 0u;\n\
             \x20       r2 = 0u;\n\
             \x20   }}\n\
             \x20   outp[tid * {OUT_STRIDE}u + 0u] = (ushort)r0;\n\
             \x20   outp[tid * {OUT_STRIDE}u + 1u] = (ushort)r1;\n\
             \x20   outp[tid * {OUT_STRIDE}u + 2u] = (ushort)r2;\n\
             \x20   outp[tid * {OUT_STRIDE}u + 3u] = (ushort)c.trap;\n\
             \x20   outp[tid * {OUT_STRIDE}u + 4u] = (ushort)(steps & 0xFFFFu);\n\
             \x20   outp[tid * {OUT_STRIDE}u + 5u] = (ushort)(steps >> 16u);\n\
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
