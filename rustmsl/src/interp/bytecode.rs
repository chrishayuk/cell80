//! The flat instruction set and program container shared by the linearizer
//! ([`super::linearize`]), the CPU reference VM ([`super::cpu`]), and the GPU
//! encoder ([`super::gpu`]) — one bytecode vocabulary, three consumers.

use cell80_core::ir::{BinOp, Cmp, Width};

/// Fixed operand-stack cap (note 4), in stack entries — matches the kernel's
/// MAX_STACK. A u32 uses two entries. Cells whose static max depth exceeds this
/// are excluded and counted.
pub const STACK_CAP: usize = 32;

/// One flat instruction. Value ops act on an operand stack; `Step` is the only
/// step-charging op (note 1); control ops carry a resolved instruction index.
#[derive(Debug, Clone)]
pub(crate) enum Inst {
    /// Charge one IR step — placed to mirror `Interp::tick` exactly.
    Step,
    PushLit(u16),
    PushVar(usize),
    /// Non-shift binary op: pop b, pop a, push `mask(a op b, w)`.
    Bin(BinOp, Width),
    /// Shift by a compile-time-literal amount (the RHS `Interp` never evaluates):
    /// pop a, push the shifted value. `signed` selects arithmetic `>>` (SWord).
    ShiftLit {
        left: bool,
        k: u32,
        w: Width,
        signed: bool,
    },
    /// Comparison as a value/condition: pop b, pop a, push `(a cmp b) as 1|0`.
    Cmp(Cmp, bool),
    /// Truncate to 8 bits: pop a, push `a & 0xFF`.
    Trunc,
    /// Pop a, store into slot.
    Store(usize),
    /// Pop and discard (a `Stmt::Eval`'d expression's result).
    Pop,
    /// Duplicate the top of stack (short-circuit `Logic`: keep the deciding
    /// value while also testing it).
    Dup,
    /// Pop a; if zero, jump to the target instruction index.
    JmpZero(usize),
    Jmp(usize),
    /// Finish: the result is the bottom `arity` operands (reg0..), in order.
    Ret(usize),
    /// `halt(code)` — stop with STATUS_HALT, the code (top of stack) riding r0.
    /// A diverging expression: statically it leaves its "result" in place so the
    /// enclosing context stays balanced, but execution ends here.
    Halt,
    /// `__bits_*` intrinsics over u16 (pop x, push result): `count_ones`,
    /// `leading_zeros`, `trailing_zeros`. These lower as calls in the IR but the
    /// interpreter owns their rustc-identical semantics, so they're one op here.
    Popcnt,
    Clz,
    Ctz,
    // ── 32-bit ops. A u32 lives as two stack entries: low word pushed first,
    // high word on top. Existing u16 ops are untouched; these are the only
    // width-aware additions. Var32/Lit32/Trunc32/Widen/Assign32 decompose into
    // existing PushVar/PushLit/Pop/Store pairs and need no new op.
    /// Pop b(lo,hi) then a(lo,hi); push `a op b` as (lo,hi). `signed` selects
    /// i32 div/rem (MIN/-1 guarded on GPU — 32-bit div overflows, unlike 16-bit).
    Bin32(BinOp, bool),
    /// Pop a(lo,hi); push the shift by literal `k` as (lo,hi).
    Shift32 {
        left: bool,
        k: u8,
        signed: bool,
    },
    /// Pop b(lo,hi) then a(lo,hi); push `(a cmp b) as 1|0` (a u16 bool).
    Cmp32(Cmp, bool),
    /// Sign-extend: pop lo (u16), push lo then the high word (0xFFFF if lo<0 else 0).
    SextHi,
}

/// A linearized cell: flat bytecode, its slot count, param count, and static max
/// operand-stack depth. In the buffer format this is one entry of the per-cell
/// offset table. Produce one with [`linearize`](super::linearize), run it with
/// [`cpu_run`](super::cpu_run) or feed a slice of them to
/// [`InterpBatch`](super::InterpBatch). `Clone` is cheap (a bytecode `memcpy`) —
/// far cheaper than re-`linearize`, so a search loop can carry survivors forward.
#[derive(Clone)]
pub struct CellProgram {
    pub(crate) code: Vec<Inst>,
    /// Total 2-byte local slots the cell uses (after inlining).
    pub n_locals: usize,
    /// Entry parameter slots consumed from the probe triple (a u32 arg is 2).
    pub params: usize,
    /// Static maximum operand-stack depth in entries (a u32 uses two).
    pub max_depth: usize,
}

/// Why a cell is outside the supported subset — callers aggregate these into a
/// coverage histogram (the empirical input to what to support next).
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Bail {
    ResidualCall, // multi-call-site helper survived inlining (note 2)
    WideValue,    // 32-bit / f32 node or wide param/ret
    Memory,       // array / pointer / peek-poke / port
    UnsupportedExpr(&'static str),
    UnsupportedStmt(&'static str),
    StackTooDeep,
}

// Opcodes — the kernel decoder's constant block is *generated* from these
// (see `super::source::interp_const_block`), so the two cannot drift. Shared
// by every dialect's kernel-source generator and by [`pack`], the encoder
// both GPU backends ([`super::gpu`], `cuda.rs`) and the CPU emulator
// (`cpu_emu.rs`) call to flatten a program set for dispatch.
pub(crate) const OP_STEP: u32 = 0;
pub(crate) const OP_PUSHLIT: u32 = 1;
pub(crate) const OP_PUSHVAR: u32 = 2;
pub(crate) const OP_BIN: u32 = 3;
pub(crate) const OP_SHIFTLIT: u32 = 4;
pub(crate) const OP_CMP: u32 = 5;
pub(crate) const OP_TRUNC: u32 = 6;
pub(crate) const OP_STORE: u32 = 7;
pub(crate) const OP_POP: u32 = 8;
pub(crate) const OP_JMPZERO: u32 = 9;
pub(crate) const OP_JMP: u32 = 10;
pub(crate) const OP_RET: u32 = 11;
pub(crate) const OP_DUP: u32 = 12;
pub(crate) const OP_HALT: u32 = 13;
pub(crate) const OP_POPCNT: u32 = 14;
pub(crate) const OP_CLZ: u32 = 15;
pub(crate) const OP_CTZ: u32 = 16;
pub(crate) const OP_BIN32: u32 = 17;
pub(crate) const OP_SHIFT32: u32 = 18;
pub(crate) const OP_CMP32: u32 = 19;
pub(crate) const OP_SEXTHI: u32 = 20;

/// Kernel slot-array bound — cells needing more locals are skipped at build.
pub(crate) const MAX_LOCALS: usize = 64;

fn binop_code(op: BinOp) -> u32 {
    match op {
        BinOp::Add => 0,
        BinOp::Sub => 1,
        BinOp::Mul => 2,
        BinOp::Div => 3,
        BinOp::Rem => 4,
        BinOp::Or => 5,
        BinOp::And => 6,
        BinOp::Xor => 7,
        BinOp::Shl | BinOp::Shr => unreachable!("shifts encode as SHIFTLIT"),
    }
}
fn width_code(w: Width) -> u32 {
    match w {
        Width::Byte => 0,
        Width::SWord => 2,
        _ => 1,
    }
}
fn cmp_code(c: Cmp) -> u32 {
    match c {
        Cmp::Lt => 0,
        Cmp::Le => 1,
        Cmp::Gt => 2,
        Cmp::Ge => 3,
        Cmp::Eq => 4,
        Cmp::Ne => 5,
    }
}

/// Flatten one `CellProgram` to `(op, arg)` u32 pairs, coalescing consecutive
/// `Step`s into `STEP k` — but never across a jump target (point 3): every
/// basic-block leader (including each loop back-edge) starts a fresh
/// instruction, so completed-run step totals are identical to `Interp` and
/// every loop iteration remains a fuel-check point. Jump targets are remapped
/// to post-coalesce indices.
fn encode(prog: &CellProgram) -> Vec<u32> {
    let code = &prog.code;
    // Basic-block leaders: every jump target.
    let mut leader = vec![false; code.len()];
    for inst in code {
        if let Inst::Jmp(t) | Inst::JmpZero(t) = inst {
            leader[*t] = true;
        }
    }
    let mut words: Vec<(u32, u32)> = Vec::new();
    let mut old2new = vec![0u32; code.len()];
    let mut i = 0usize;
    while i < code.len() {
        if matches!(code[i], Inst::Step) {
            let new_idx = words.len() as u32;
            let mut k = 0u32;
            let mut j = i;
            while j < code.len() && matches!(code[j], Inst::Step) && (j == i || !leader[j]) {
                old2new[j] = new_idx;
                k += 1;
                j += 1;
            }
            words.push((OP_STEP, k));
            i = j;
        } else {
            old2new[i] = words.len() as u32;
            let pair = match &code[i] {
                Inst::PushLit(n) => (OP_PUSHLIT, *n as u32),
                Inst::PushVar(s) => (OP_PUSHVAR, *s as u32),
                Inst::Bin(op, w) => (OP_BIN, binop_code(*op) | (width_code(*w) << 8)),
                Inst::ShiftLit { left, k, w, signed } => (
                    OP_SHIFTLIT,
                    (*k & 0xFFFF)
                        | ((*left as u32) << 16)
                        | ((*signed as u32) << 17)
                        | (width_code(*w) << 18),
                ),
                Inst::Cmp(c, signed) => (OP_CMP, cmp_code(*c) | ((*signed as u32) << 8)),
                Inst::Trunc => (OP_TRUNC, 0),
                Inst::Store(s) => (OP_STORE, *s as u32),
                Inst::Pop => (OP_POP, 0),
                Inst::Dup => (OP_DUP, 0),
                Inst::JmpZero(t) => (OP_JMPZERO, *t as u32), // remapped below
                Inst::Jmp(t) => (OP_JMP, *t as u32),         // remapped below
                Inst::Ret(arity) => (OP_RET, *arity as u32),
                Inst::Halt => (OP_HALT, 0),
                Inst::Popcnt => (OP_POPCNT, 0),
                Inst::Clz => (OP_CLZ, 0),
                Inst::Ctz => (OP_CTZ, 0),
                Inst::Bin32(op, signed) => (OP_BIN32, binop_code(*op) | ((*signed as u32) << 8)),
                Inst::Shift32 { left, k, signed } => (
                    OP_SHIFT32,
                    (*k as u32) | ((*left as u32) << 16) | ((*signed as u32) << 17),
                ),
                Inst::Cmp32(c, signed) => (OP_CMP32, cmp_code(*c) | ((*signed as u32) << 8)),
                Inst::SextHi => (OP_SEXTHI, 0),
                Inst::Step => unreachable!(),
            };
            words.push(pair);
            i += 1;
        }
    }
    // Remap jump targets to post-coalesce instruction indices.
    for w in &mut words {
        if w.0 == OP_JMP || w.0 == OP_JMPZERO {
            w.1 = old2new[w.1 as usize];
        }
    }
    // Flatten to [op, arg, op, arg, ...].
    let mut flat = Vec::with_capacity(words.len() * 2);
    for (op, arg) in words {
        flat.push(op);
        flat.push(arg);
    }
    flat
}

/// Encode a program set into the concatenated code + per-cell offset table,
/// skipping (and counting) cells over the kernel's local bound. Shared by
/// every dispatch backend ([`super::gpu`]'s `InterpBatch`, `cuda.rs`'s
/// `CudaInterpBatch`, `cpu_emu.rs`'s pre-silicon validator).
pub(crate) fn pack(progs: &[CellProgram]) -> (Vec<u32>, Vec<u32>, usize) {
    let (mut code, mut table, mut skipped) = (Vec::new(), Vec::new(), 0usize);
    for prog in progs {
        if prog.n_locals > MAX_LOCALS {
            skipped += 1;
            continue;
        }
        table.push((code.len() / 2) as u32); // code offset in instructions
        code.extend_from_slice(&encode(prog));
        table.push(prog.n_locals as u32);
        table.push(prog.params as u32);
    }
    (code, table, skipped)
}
