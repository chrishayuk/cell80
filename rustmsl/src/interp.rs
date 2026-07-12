//! The **bytecode interpreter** backend: a fixed-size MSL kernel that reads each
//! cell's IR from a data buffer, so a library dispatch's kernel size is constant
//! in the number of cells — the library-dispatch body of the two-body design
//! ([`crate::GpuBatch`]/[`crate::compile_library`] compile one cell × N inputs
//! for single-cell batch; this interprets a whole library × a probe set).
//!
//! Pricing (`cell80/examples/library_launch_cost.rs`) found that *compiling*
//! cells makes the kernel grow with the library and hits a kernel-size cliff at
//! ~64→128 fused cells (~44×); this backend has no such cliff — flat/no-cliff to
//! 500k distinct entries (153 MiB) at ~23 ns/eval on a representative corpus.
//! The trade is per-eval speed: at small scale the compiled path wins (~an order
//! of magnitude), so the two bodies hand off around 10²–10³ cells. What this
//! backend buys is *scale* — the compiled path cannot exist at thousands of cells.
//!
//! Everything here is **bit-identical to `cell80_core::Interp`** — values *and*
//! IR-step counts. Three design points are baked into the bytecode format:
//! 1. **Step parity via emitted `Step` markers**, placed at the tree-walker's
//!    exact charge points (per statement, per loop-iteration attempt, per
//!    expression node — except unrolled shift-amount literals), coalesced within
//!    basic blocks but never across a jump target, so completed-run counts match
//!    and every loop back-edge stays a fuel-check point.
//! 2. **No call stack** — calls are fully inlined at linearize time (the
//!    recursion gate makes that total); `__bits_*` builtins are intrinsic ops.
//! 3. **Per-cell offset table + concatenated code**, dispatched one cell per
//!    threadgroup with probes across lanes, so bytecode fetch is group-uniform.
//!
//! Supported subset (the rest bail with a typed [`Bail`], reported by callers):
//! value cells over u8/u16/i16/u32/i32/bool, incl. control flow, short-circuit
//! logic, `halt`, div/shift/compare at both widths, and inlined helper calls.
//! Not yet: state cells (state-window addressing is reserved), runtime-amount
//! shifts, memory/array ops, wide-returning inlined calls, `wide_second`.

use cell80_core::ir::{BinOp, Cmp, Cond, Expr, Func, Stmt, Width};

/// Outputs produced per (cell, probe): `[r0, r1, r2, status, steps_lo, steps_hi]`
/// — the same sextet the compiled backend produces.
pub use crate::OUT_STRIDE;
/// Inputs consumed per thread (the register-arg triple).
pub use crate::IN_STRIDE;

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
    ShiftLit { left: bool, k: u32, w: Width, signed: bool },
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
    Shift32 { left: bool, k: u8, signed: bool },
    /// Pop b(lo,hi) then a(lo,hi); push `(a cmp b) as 1|0` (a u16 bool).
    Cmp32(Cmp, bool),
    /// Sign-extend: pop lo (u16), push lo then the high word (0xFFFF if lo<0 else 0).
    SextHi,
}

/// A linearized cell: flat bytecode, its slot count, param count, and static max
/// operand-stack depth. In the buffer format this is one entry of the per-cell
/// offset table. Produce one with [`linearize`]; run it with [`cpu_run`] or feed
/// a slice of them to [`InterpBatch`].
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
    ResidualCall,        // multi-call-site helper survived inlining (note 2)
    WideValue,           // 32-bit / f32 node or wide param/ret
    Memory,              // array / pointer / peek-poke / port
    UnsupportedExpr(&'static str),
    UnsupportedStmt(&'static str),
    StackTooDeep,
}

/// The linearizer: emits code while tracking operand-stack height and holding
/// break/continue targets. Labels are allocated up front and resolved to
/// instruction indices in a final fixup pass.
struct Lin<'a> {
    funcs: &'a [(String, Func)],    // for full inlining of residual calls (note 2)
    code: Vec<Inst>,
    labels: Vec<usize>,             // label id → instruction index (filled by `place`)
    loops: Vec<(usize, usize)>,     // (continue target, break target) per enclosing loop
    ret_ctx: Vec<usize>,            // inline-return labels (empty ⇒ top level ⇒ Ret)
    frame_base: usize,              // current frame's slot offset (0 ⇒ entry frame)
    slots_used: usize,              // high-water mark of allocated slots
    wide_ret: bool,                 // entry returns u32 (Ret produces 2 words)
    cur_depth: usize,
    max_depth: usize,
}

impl<'a> Lin<'a> {
    fn new(funcs: &'a [(String, Func)], entry_locals: usize) -> Self {
        Lin {
            funcs,
            code: Vec::new(),
            labels: Vec::new(),
            loops: Vec::new(),
            ret_ctx: Vec::new(),
            frame_base: 0,
            slots_used: entry_locals,
            wide_ret: false,
            cur_depth: 0,
            max_depth: 0,
        }
    }

    fn new_label(&mut self) -> usize {
        self.labels.push(usize::MAX);
        self.labels.len() - 1
    }
    fn place(&mut self, id: usize) {
        self.labels[id] = self.code.len();
    }

    /// Emit an instruction, updating the operand-stack height by its net effect.
    fn emit(&mut self, inst: Inst) {
        let delta: isize = match &inst {
            Inst::PushLit(_) | Inst::PushVar(_) | Inst::Dup => 1,
            Inst::Bin(..) | Inst::Cmp(..) | Inst::Store(_) | Inst::Pop | Inst::JmpZero(_) => -1,
            Inst::ShiftLit { .. }
            | Inst::Trunc
            | Inst::Step
            | Inst::Jmp(_)
            | Inst::Halt
            | Inst::Popcnt
            | Inst::Clz
            | Inst::Ctz
            | Inst::Shift32 { .. } => 0,
            Inst::Bin32(..) => -2, // pop two u32 (4), push one u32 (2)
            Inst::Cmp32(..) => -3, // pop two u32 (4), push one u16 bool (1)
            Inst::SextHi => 1,     // pop u16 (1), push u32 (2)
            Inst::Ret(arity) => -(*arity as isize),
        };
        if delta > 0 {
            self.cur_depth += delta as usize;
            self.max_depth = self.max_depth.max(self.cur_depth);
        } else {
            self.cur_depth = self.cur_depth.saturating_sub((-delta) as usize);
        }
        self.code.push(inst);
    }

    /// Linearize an expression, emitting its node `Step` first (mirroring
    /// `eval16`'s tick at every node), then its children, then its combining op.
    fn expr(&mut self, e: &Expr) -> Result<(), Bail> {
        self.emit(Inst::Step); // one tick per expression node
        match e {
            Expr::Lit(n) => self.emit(Inst::PushLit(*n)),
            Expr::Var(slot) => self.emit(Inst::PushVar(self.frame_base + *slot)),
            Expr::Trunc(inner) => {
                self.expr(inner)?;
                self.emit(Inst::Trunc);
            }
            Expr::Cmp { cmp, lhs, rhs, signed } => {
                self.expr(lhs)?;
                self.expr(rhs)?;
                self.emit(Inst::Cmp(*cmp, *signed));
            }
            // Shifts: the literal RHS is NOT an evaluated operand (no tick).
            Expr::Bin(op @ (BinOp::Shl | BinOp::Shr), l, r, w) => {
                let Expr::Lit(k) = r.as_ref() else {
                    return Err(Bail::UnsupportedExpr("non-literal shift amount"));
                };
                self.expr(l)?;
                self.emit(Inst::ShiftLit {
                    left: matches!(op, BinOp::Shl),
                    k: *k as u32,
                    w: *w,
                    signed: *w == Width::SWord,
                });
            }
            Expr::Bin(op, l, r, w) => {
                self.expr(l)?;
                self.expr(r)?;
                self.emit(Inst::Bin(*op, *w));
            }
            // `__bits_*` builtins: node `Step` (above) + the arg's ticks, then the
            // intrinsic (no extra tick — mirrors `call()`'s builtin path). Any
            // other call is fully inlined (note 2).
            Expr::Call(name, args) => match name.as_str() {
                "__bits_count_ones" => {
                    self.expr(&args[0])?;
                    self.emit(Inst::Popcnt);
                }
                "__bits_leading_zeros" => {
                    self.expr(&args[0])?;
                    self.emit(Inst::Clz);
                }
                "__bits_trailing_zeros" => {
                    self.expr(&args[0])?;
                    self.emit(Inst::Ctz);
                }
                _ => self.inline_call(name, args)?,
            },
            // 32-bit → 16-bit bridges (evaluated by eval16): node Step (above) +
            // the wide subtree's ticks, then narrow the result.
            Expr::Trunc32(inner) => {
                self.expr32(inner)?;
                self.emit(Inst::Pop); // drop the high word, keep low
            }
            Expr::Cmp32 { cmp, lhs, rhs, signed } => {
                self.expr32(lhs)?;
                self.expr32(rhs)?;
                self.emit(Inst::Cmp32(*cmp, *signed));
            }
            // Pure-wide nodes never appear in 16-bit value position.
            Expr::Lit32(_)
            | Expr::Var32(_)
            | Expr::Bin32(..)
            | Expr::Shift32 { .. }
            | Expr::Deref32(..)
            | Expr::Widen(_)
            | Expr::SignExtend(_) => return Err(Bail::WideValue),
            Expr::Index(..)
            | Expr::Peek(_)
            | Expr::InPort(_)
            | Expr::AddrOf(_)
            | Expr::ConstAddr(_)
            | Expr::Deref(..)
            | Expr::PtrIndex { .. }
            | Expr::LoadAt(..) => return Err(Bail::Memory),
            Expr::MulConst(..) => return Err(Bail::UnsupportedExpr("MulConst")),
            // Short-circuit: the deciding operand value *is* the result when it
            // decides; the RHS ticks only when actually evaluated. `Dup` the
            // decider to test it while keeping it as the fall-through result.
            Expr::Logic { and, lhs, rhs } => {
                self.expr(lhs)?;
                self.emit(Inst::Dup);
                if *and {
                    // `&&`: decider 0 short-circuits (keep it); else drop, eval rhs.
                    let end = self.new_label();
                    self.emit(Inst::JmpZero(end));
                    self.emit(Inst::Pop);
                    self.expr(rhs)?;
                    self.place(end);
                } else {
                    // `||`: decider ≠ 0 short-circuits (keep it); else drop, eval rhs.
                    let rhs_l = self.new_label();
                    let end = self.new_label();
                    self.emit(Inst::JmpZero(rhs_l));
                    self.emit(Inst::Jmp(end));
                    self.place(rhs_l);
                    self.emit(Inst::Pop);
                    self.expr(rhs)?;
                    self.place(end);
                }
            }
            Expr::ShiftVar { .. } => return Err(Bail::UnsupportedExpr("ShiftVar (runtime amount)")),
            // `halt(code)`: node `Step` (emitted above) then the code subtree
            // (its own ticks), then stop — mirrors `eval16(Halt)` tick + code eval.
            Expr::Halt(code) => {
                self.expr(code)?;
                self.emit(Inst::Halt);
            }
        }
        Ok(())
    }

    /// A condition in branch position: `cond_true` evaluates lhs and rhs (each
    /// ticks) then `cmp16` — which does NOT tick. So emit the operand `Step`s via
    /// `expr`, but the `Cmp` op carries no `Step`. Leaves a 1|0 on the stack.
    fn cond(&mut self, c: &Cond) -> Result<(), Bail> {
        self.expr(&c.lhs)?;
        self.expr(&c.rhs)?;
        self.emit(Inst::Cmp(c.cmp, c.signed));
        Ok(())
    }

    /// Linearize a 32-bit expression, leaving a u32 on the stack as two entries
    /// (low word first, high on top). Emits the node `Step` first (eval32 ticks
    /// per node, exactly like eval16).
    fn expr32(&mut self, e: &Expr) -> Result<(), Bail> {
        self.emit(Inst::Step);
        match e {
            Expr::Lit32(n) => {
                self.emit(Inst::PushLit((*n & 0xFFFF) as u16));
                self.emit(Inst::PushLit((*n >> 16) as u16));
            }
            Expr::Var32(slot) => {
                self.emit(Inst::PushVar(self.frame_base + *slot));
                self.emit(Inst::PushVar(self.frame_base + *slot + 1));
            }
            Expr::Widen(inner) => {
                self.expr(inner)?; // 16-bit low word
                self.emit(Inst::PushLit(0)); // zero-extend
            }
            Expr::SignExtend(inner) => {
                self.expr(inner)?;
                self.emit(Inst::SextHi);
            }
            // Identity in wide position (the interpreter re-evaluates wide).
            Expr::Trunc32(inner) => self.expr32(inner)?,
            Expr::Bin32(op, l, r, signed) => match op {
                BinOp::Shl | BinOp::Shr => return Err(Bail::WideValue), // → Shift32
                _ => {
                    self.expr32(l)?;
                    self.expr32(r)?;
                    self.emit(Inst::Bin32(*op, *signed));
                }
            },
            Expr::Shift32 { left, e: inner, k, signed } => {
                self.expr32(inner)?;
                self.emit(Inst::Shift32 { left: *left, k: *k, signed: *signed });
            }
            Expr::Call(..) => return Err(Bail::ResidualCall), // wide-returning call: not inlined yet
            Expr::Deref32(..) => return Err(Bail::Memory),
            _ => return Err(Bail::WideValue),
        }
        Ok(())
    }

    /// Linearize a statement. Emits the per-statement `Step` first (mirroring
    /// `exec_stmt`'s tick at the top, before the match — so break/continue/return
    /// all tick once too).
    fn stmt(&mut self, s: &Stmt) -> Result<(), Bail> {
        self.emit(Inst::Step); // one tick per statement
        match s {
            Stmt::Assign(slot, e) => {
                self.expr(e)?;
                self.emit(Inst::Store(self.frame_base + *slot));
            }
            Stmt::Eval(e) => {
                self.expr(e)?;
                self.emit(Inst::Pop);
            }
            // Inside an inlined callee, `return` leaves its value on the stack and
            // jumps to the inline-end; at top level it's a real `Ret`. The
            // per-statement `Step` (emitted above) matches `exec_stmt`'s tick either way.
            Stmt::Return(val) => match self.ret_ctx.last().copied() {
                Some(end) => {
                    let e = val.as_ref().ok_or(Bail::ResidualCall)?; // void return in a value callee
                    self.expr(e)?; // inlined callees are narrow-return (wide bail in inline_call)
                    self.emit(Inst::Jmp(end));
                }
                None => match (val, self.wide_ret) {
                    (None, _) => self.emit(Inst::Ret(0)),
                    // Wide return: eval as u32, produce r0=low, r1=high (run()'s convention).
                    (Some(e), true) => {
                        self.expr32(e)?;
                        self.emit(Inst::Ret(2));
                    }
                    (Some(e), false) => {
                        self.expr(e)?;
                        self.emit(Inst::Ret(1));
                    }
                },
            },
            Stmt::If(cond, then, els) => {
                let else_l = self.new_label();
                let end_l = self.new_label();
                self.cond(cond)?;
                self.emit(Inst::JmpZero(else_l));
                self.block(then)?;
                self.emit(Inst::Jmp(end_l));
                self.place(else_l);
                self.block(els)?;
                self.place(end_l);
            }
            Stmt::While(cond, body) => {
                let top = self.new_label();
                let end = self.new_label();
                self.place(top);
                self.emit(Inst::Step); // loop-iteration tick (Interp ticks each attempt)
                self.cond(cond)?;
                self.emit(Inst::JmpZero(end));
                self.loops.push((top, end));
                self.block(body)?;
                self.loops.pop();
                self.emit(Inst::Jmp(top));
                self.place(end);
            }
            Stmt::Loop(body) => {
                let top = self.new_label();
                let end = self.new_label();
                self.place(top);
                self.emit(Inst::Step); // loop-iteration tick
                self.loops.push((top, end));
                self.block(body)?;
                self.loops.pop();
                self.emit(Inst::Jmp(top));
                self.place(end);
            }
            Stmt::Break => {
                let (_, end) = *self.loops.last().ok_or(Bail::UnsupportedStmt("break outside loop"))?;
                self.emit(Inst::Jmp(end));
            }
            Stmt::Continue => {
                let (top, _) = *self.loops.last().ok_or(Bail::UnsupportedStmt("continue outside loop"))?;
                self.emit(Inst::Jmp(top));
            }
            Stmt::ForRange { .. } => return Err(Bail::UnsupportedStmt("ForRange")),
            Stmt::AssignTuple(..) => return Err(Bail::UnsupportedStmt("AssignTuple (call)")),
            // Wide local store: eval u32, store high then low (stack has low under high).
            Stmt::Assign32(slot, e) => {
                self.expr32(e)?;
                self.emit(Inst::Store(self.frame_base + *slot + 1));
                self.emit(Inst::Store(self.frame_base + *slot));
            }
            Stmt::Store32(..) => return Err(Bail::Memory),
            Stmt::StoreIndex(..)
            | Stmt::Poke(..)
            | Stmt::Store(..)
            | Stmt::PtrStoreIndex { .. }
            | Stmt::StoreAt(..)
            | Stmt::Fill { .. } => return Err(Bail::Memory),
        }
        Ok(())
    }

    fn block(&mut self, stmts: &[Stmt]) -> Result<(), Bail> {
        for s in stmts {
            self.stmt(s)?;
        }
        Ok(())
    }

    /// Fully inline a call in expression position — no call stack (note 2). The
    /// tick accounting mirrors `Interp::call`: the `Step` for the call node is
    /// already emitted by `expr`; each arg's eval ticks as a *caller* node; the
    /// param-binding stores do NOT tick; the callee body ticks normally; and the
    /// fall-through return evals `ret` (ticking) with no statement tick. The
    /// callee's frame is a fresh slot range so its locals never alias the caller's.
    fn inline_call(&mut self, name: &str, args: &[Expr]) -> Result<(), Bail> {
        let callee = self
            .funcs
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, f)| f)
            .ok_or(Bail::ResidualCall)?;
        // Value-returning, narrow, non-wide callees only (the common helper shape).
        if callee.wide_param || callee.wide_second || callee.wide_ret || callee.ret.len() != 1 {
            return Err(Bail::WideValue);
        }
        if callee.params != args.len() {
            return Err(Bail::ResidualCall);
        }
        // Evaluate every arg (in the CALLER frame) before binding any param — the
        // interpreter's rule (an arg may read a pre-call value). Then store into
        // the fresh callee frame in reverse (stack order); stores don't tick.
        let base = self.slots_used;
        self.slots_used += callee.n_locals;
        for a in args {
            self.expr(a)?;
        }
        for i in (0..callee.params).rev() {
            self.emit(Inst::Store(base + i));
        }
        // Inline the body in the callee frame; a callee `return` jumps to `end`.
        let saved = self.frame_base;
        self.frame_base = base;
        let end = self.new_label();
        self.ret_ctx.push(end);
        self.block(&callee.body)?;
        self.ret_ctx.pop();
        // Fall-through return: eval the single ret expr (dead if the body always
        // returned — its `Step`s then never execute, so parity holds).
        self.expr(&callee.ret[0])?;
        self.place(end);
        self.frame_base = saved;
        Ok(())
    }

    /// Resolve label ids embedded in jumps to instruction indices.
    fn resolve(&mut self) {
        let labels = &self.labels;
        for inst in &mut self.code {
            match inst {
                Inst::Jmp(t) | Inst::JmpZero(t) => *t = labels[*t],
                _ => {}
            }
        }
    }
}

/// Linearize the entry `entry` from `funcs`, fully inlining any calls it makes.
/// Bails (with a reason) on anything outside the supported subset.
pub fn linearize(funcs: &[(String, Func)], entry: &str) -> Result<CellProgram, Bail> {
    let f = funcs
        .iter()
        .find(|(n, _)| n == entry)
        .map(|(_, f)| f)
        .ok_or(Bail::ResidualCall)?;
    // wide_param (u32 first arg → 2 slots) and wide_ret (u32 return → 2 words)
    // are handled; wide_second (the __mul32 two-wide-param stack shape) is not.
    if f.wide_second {
        return Err(Bail::WideValue);
    }
    let mut lin = Lin::new(funcs, f.n_locals);
    lin.wide_ret = f.wide_ret;
    lin.block(&f.body)?;
    // Fall-through return: `Interp` evaluates each `f.ret` expr (each ticks) with
    // no statement tick, then returns them. A wide return produces two words.
    if f.wide_ret {
        lin.expr32(&f.ret[0])?;
        lin.emit(Inst::Ret(2));
    } else {
        for e in &f.ret {
            lin.expr(e)?;
        }
        lin.emit(Inst::Ret(f.ret.len()));
    }
    if lin.max_depth > STACK_CAP {
        return Err(Bail::StackTooDeep);
    }
    lin.resolve();
    Ok(CellProgram {
        code: lin.code,
        n_locals: lin.slots_used,
        params: f.params,
        max_depth: lin.max_depth,
    })
}

// ── the reference VM (transliteration target for the MSL kernel) ──────────────

/// The VM's outcome, mirroring `Interp`'s: a value return, a `halt(code)`, a
/// fuel-exhaustion trap (with the step count *at* the trap — the parity point),
/// or divide-by-zero.
#[derive(Debug, PartialEq)]
pub enum VmOut {
    Value(Vec<u16>, u64),
    Halt(u16, u64),
    Fuel(u64),
    DivZero,
}

const VM_FUEL: u64 = 100_000_000;

fn mask(v: u16, w: Width) -> u16 {
    if w == Width::Byte {
        v & 0xFF
    } else {
        v
    }
}

fn cmp16(cmp: Cmp, l: u16, r: u16, signed: bool) -> bool {
    if signed && !matches!(cmp, Cmp::Eq | Cmp::Ne) {
        let (l, r) = (l as i16, r as i16);
        match cmp {
            Cmp::Lt => l < r,
            Cmp::Le => l <= r,
            Cmp::Gt => l > r,
            Cmp::Ge => l >= r,
            _ => unreachable!(),
        }
    } else {
        match cmp {
            Cmp::Lt => l < r,
            Cmp::Le => l <= r,
            Cmp::Gt => l > r,
            Cmp::Ge => l >= r,
            Cmp::Eq => l == r,
            Cmp::Ne => l != r,
        }
    }
}

fn cmp32(cmp: Cmp, l: u32, r: u32, signed: bool) -> bool {
    if signed && !matches!(cmp, Cmp::Eq | Cmp::Ne) {
        let (l, r) = (l as i32, r as i32);
        match cmp {
            Cmp::Lt => l < r,
            Cmp::Le => l <= r,
            Cmp::Gt => l > r,
            Cmp::Ge => l >= r,
            _ => unreachable!(),
        }
    } else {
        match cmp {
            Cmp::Lt => l < r,
            Cmp::Le => l <= r,
            Cmp::Gt => l > r,
            Cmp::Ge => l >= r,
            Cmp::Eq => l == r,
            Cmp::Ne => l != r,
        }
    }
}

/// Execute a `CellProgram`. Returns the outcome + IR steps — the things the
/// correctness gate compares against `Interp`.
pub fn cpu_run(prog: &CellProgram, args: &[u16]) -> VmOut {
    let mut slots = vec![0u16; prog.n_locals];
    for i in 0..prog.params {
        slots[i] = args.get(i).copied().unwrap_or(0);
    }
    let mut stack: Vec<u16> = Vec::with_capacity(prog.max_depth + 1);
    let mut steps = 0u64;
    let mut pc = 0usize;
    loop {
        match &prog.code[pc] {
            Inst::Step => {
                steps += 1;
                if steps >= VM_FUEL {
                    return VmOut::Fuel(steps);
                }
            }
            Inst::PushLit(n) => stack.push(*n),
            Inst::PushVar(s) => stack.push(slots[*s]),
            Inst::Bin(op, w) => {
                let b = stack.pop().unwrap();
                let a = stack.pop().unwrap();
                let raw = match op {
                    BinOp::Add => a.wrapping_add(b),
                    BinOp::Sub => a.wrapping_sub(b),
                    BinOp::Mul => a.wrapping_mul(b),
                    BinOp::Or => a | b,
                    BinOp::And => a & b,
                    BinOp::Xor => a ^ b,
                    BinOp::Div | BinOp::Rem => {
                        if b == 0 {
                            return VmOut::DivZero;
                        }
                        match (op, *w == Width::SWord) {
                            (BinOp::Div, true) => (a as i16).wrapping_div(b as i16) as u16,
                            (BinOp::Rem, true) => (a as i16).wrapping_rem(b as i16) as u16,
                            (BinOp::Div, false) => a / b,
                            (BinOp::Rem, false) => a % b,
                            _ => unreachable!(),
                        }
                    }
                    BinOp::Shl | BinOp::Shr => unreachable!("shifts are ShiftLit"),
                };
                stack.push(mask(raw, *w));
            }
            Inst::ShiftLit { left, k, w, signed } => {
                let a = stack.pop().unwrap();
                let raw = if *left {
                    if *k >= 16 {
                        0
                    } else {
                        a << *k
                    }
                } else if *signed {
                    ((a as i16) >> (*k).min(15)) as u16
                } else if *k >= 16 {
                    0
                } else {
                    a >> *k
                };
                stack.push(mask(raw, *w));
            }
            Inst::Cmp(cmp, signed) => {
                let b = stack.pop().unwrap();
                let a = stack.pop().unwrap();
                stack.push(cmp16(*cmp, a, b, *signed) as u16);
            }
            Inst::Trunc => {
                let a = stack.pop().unwrap();
                stack.push(a & 0xFF);
            }
            Inst::Popcnt => {
                let a = stack.pop().unwrap();
                stack.push(a.count_ones() as u16);
            }
            Inst::Clz => {
                let a = stack.pop().unwrap();
                stack.push(a.leading_zeros() as u16);
            }
            Inst::Ctz => {
                let a = stack.pop().unwrap();
                stack.push(a.trailing_zeros() as u16);
            }
            Inst::Bin32(op, signed) => {
                let bh = stack.pop().unwrap() as u32;
                let bl = stack.pop().unwrap() as u32;
                let ah = stack.pop().unwrap() as u32;
                let al = stack.pop().unwrap() as u32;
                let (a, b) = (al | (ah << 16), bl | (bh << 16));
                let res = match op {
                    BinOp::Add => a.wrapping_add(b),
                    BinOp::Sub => a.wrapping_sub(b),
                    BinOp::Mul => a.wrapping_mul(b),
                    BinOp::Or => a | b,
                    BinOp::And => a & b,
                    BinOp::Xor => a ^ b,
                    BinOp::Div | BinOp::Rem => {
                        if b == 0 {
                            return VmOut::DivZero;
                        }
                        match (op, *signed) {
                            (BinOp::Div, true) => (a as i32).wrapping_div(b as i32) as u32,
                            (BinOp::Rem, true) => (a as i32).wrapping_rem(b as i32) as u32,
                            (BinOp::Div, false) => a / b,
                            (BinOp::Rem, false) => a % b,
                            _ => unreachable!(),
                        }
                    }
                    BinOp::Shl | BinOp::Shr => unreachable!("wide shifts are Shift32"),
                };
                stack.push((res & 0xFFFF) as u16);
                stack.push((res >> 16) as u16);
            }
            Inst::Shift32 { left, k, signed } => {
                let ah = stack.pop().unwrap() as u32;
                let al = stack.pop().unwrap() as u32;
                let a = al | (ah << 16);
                let res = if *signed && !*left {
                    ((a as i32) >> (*k).min(31) as u32) as u32
                } else if *k >= 32 {
                    0
                } else if *left {
                    a << *k
                } else {
                    a >> *k
                };
                stack.push((res & 0xFFFF) as u16);
                stack.push((res >> 16) as u16);
            }
            Inst::Cmp32(cmp, signed) => {
                let bh = stack.pop().unwrap() as u32;
                let bl = stack.pop().unwrap() as u32;
                let ah = stack.pop().unwrap() as u32;
                let al = stack.pop().unwrap() as u32;
                let r = cmp32(*cmp, al | (ah << 16), bl | (bh << 16), *signed);
                stack.push(r as u16);
            }
            Inst::SextHi => {
                let lo = stack.pop().unwrap();
                stack.push(lo);
                stack.push(if lo & 0x8000 != 0 { 0xFFFF } else { 0 });
            }
            Inst::Store(s) => slots[*s] = stack.pop().unwrap(),
            Inst::Pop => {
                stack.pop().unwrap();
            }
            Inst::Dup => {
                let v = *stack.last().unwrap();
                stack.push(v);
            }
            Inst::JmpZero(t) => {
                if stack.pop().unwrap() == 0 {
                    pc = *t;
                    continue;
                }
            }
            Inst::Jmp(t) => {
                pc = *t;
                continue;
            }
            Inst::Ret(arity) => {
                return VmOut::Value(stack[..*arity].to_vec(), steps);
            }
            Inst::Halt => {
                return VmOut::Halt(*stack.last().unwrap(), steps);
            }
        }
        pc += 1;
    }
}
/// The GPU library-dispatch backend (macOS/Metal). Lives here so the bytecode
/// and linearizer above stay buildable everywhere.
#[cfg(target_os = "macos")]
mod gpu {
    use super::{CellProgram, Inst, IN_STRIDE, OUT_STRIDE};
    use cell80_core::ir::{BinOp, Cmp, Width};
    use metal::{Buffer, CommandQueue, ComputePipelineState, Device, MTLResourceOptions, MTLSize};

    // Opcodes — kept in lockstep with the MSL decoder below.
    const OP_STEP: u32 = 0;
    const OP_PUSHLIT: u32 = 1;
    const OP_PUSHVAR: u32 = 2;
    const OP_BIN: u32 = 3;
    const OP_SHIFTLIT: u32 = 4;
    const OP_CMP: u32 = 5;
    const OP_TRUNC: u32 = 6;
    const OP_STORE: u32 = 7;
    const OP_POP: u32 = 8;
    const OP_JMPZERO: u32 = 9;
    const OP_JMP: u32 = 10;
    const OP_RET: u32 = 11;
    const OP_DUP: u32 = 12;
    const OP_HALT: u32 = 13;
    const OP_POPCNT: u32 = 14;
    const OP_CLZ: u32 = 15;
    const OP_CTZ: u32 = 16;
    const OP_BIN32: u32 = 17;
    const OP_SHIFT32: u32 = 18;
    const OP_CMP32: u32 = 19;
    const OP_SEXTHI: u32 = 20;


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

    const KERNEL: &str = r#"
#include <metal_stdlib>
using namespace metal;

constant uint OP_STEP=0,OP_PUSHLIT=1,OP_PUSHVAR=2,OP_BIN=3,OP_SHIFTLIT=4,
              OP_CMP=5,OP_TRUNC=6,OP_STORE=7,OP_POP=8,OP_JMPZERO=9,OP_JMP=10,OP_RET=11,OP_DUP=12,OP_HALT=13,
              OP_POPCNT=14,OP_CLZ=15,OP_CTZ=16,OP_BIN32=17,OP_SHIFT32=18,OP_CMP32=19,OP_SEXTHI=20;
constant uint ST_OK=0u, ST_DIV0=1u, ST_HALT=2u, ST_FUEL=4u;
constant uint FUEL=100000000u;
#define MAX_LOCALS 64
#define MAX_STACK  32

kernel void interp(
    const device uint*   code       [[buffer(0)]],
    const device uint*   cell_table  [[buffer(1)]],
    const device ushort* probes     [[buffer(2)]],
    device ushort*       out        [[buffer(3)]],
    constant uint&       n_probes    [[buffer(4)]],
    uint cell [[threadgroup_position_in_grid]],
    uint p    [[thread_position_in_threadgroup]])
{
    if (p >= n_probes) return;
    uint code_off = cell_table[cell*3+0];
    uint n_locals = cell_table[cell*3+1];
    uint params   = cell_table[cell*3+2];

    ushort slots[MAX_LOCALS];
    for (uint i=0;i<MAX_LOCALS;i++) slots[i]=0;
    for (uint i=0;i<params && i<3u;i++) slots[i]=probes[p*3+i];

    ushort stack[MAX_STACK];
    int sp=0;
    uint steps=0u, status=ST_OK, pc=code_off, guard=0u;
    ushort r0=0,r1=0,r2=0;
    bool done=false;
    while(!done){
        if(++guard > 400000000u){ status=ST_FUEL; break; }
        uint op = code[pc*2]; uint arg = code[pc*2+1];
        switch(op){
          case OP_STEP: steps+=arg; if(steps>=FUEL){status=ST_FUEL;done=true;} break;
          case OP_PUSHLIT: stack[sp++]=(ushort)(arg & 0xFFFFu); break;
          case OP_PUSHVAR: stack[sp++]=slots[arg]; break;
          case OP_BIN: {
             ushort b=stack[--sp]; ushort a=stack[--sp];
             uint binop=arg&0xFFu; uint w=(arg>>8)&0xFFu; bool sw=(w==2u);
             ushort res=0;
             switch(binop){
               case 0: res=a+b; break;
               case 1: res=a-b; break;
               case 2: res=a*b; break;
               case 5: res=a|b; break;
               case 6: res=a&b; break;
               case 7: res=a^b; break;
               case 3: case 4: {
                  if(b==0u){ status=ST_DIV0; done=true; res=0; }
                  else if(sw){ short sa=(short)a, sb=(short)b; res=(binop==3u)?(ushort)(sa/sb):(ushort)(sa%sb); }
                  else { res=(binop==3u)?(a/b):(a%b); }
                  break;
               }
             }
             if(w==0u) res&=0xFFu;
             stack[sp++]=res; break;
          }
          case OP_SHIFTLIT: {
             ushort a=stack[--sp];
             uint k=arg&0xFFFFu; bool left=((arg>>16)&1u)!=0u; bool sgn=((arg>>17)&1u)!=0u; uint w=(arg>>18)&0x3u;
             ushort res;
             if(left){ res=(k>=16u)?0:(ushort)(a<<k); }
             else if(sgn){ short sa=(short)a; uint kk=min(k,15u); res=(ushort)(sa>>kk); }
             else { res=(k>=16u)?0:(ushort)(a>>k); }
             if(w==0u) res&=0xFFu;
             stack[sp++]=res; break;
          }
          case OP_CMP: {
             ushort b=stack[--sp]; ushort a=stack[--sp];
             uint cmp=arg&0xFFu; bool sgn=((arg>>8)&1u)!=0u; bool r;
             if(sgn && cmp<4u){ short sa=(short)a, sb=(short)b;
                switch(cmp){case 0:r=sa<sb;break;case 1:r=sa<=sb;break;case 2:r=sa>sb;break;default:r=sa>=sb;break;}
             } else {
                switch(cmp){case 0:r=a<b;break;case 1:r=a<=b;break;case 2:r=a>b;break;case 3:r=a>=b;break;case 4:r=a==b;break;default:r=a!=b;break;}
             }
             stack[sp++]=r?1:0; break;
          }
          case OP_TRUNC: { ushort a=stack[--sp]; stack[sp++]=a&0xFFu; break; }
          // u16 bit intrinsics: popcount is width-agnostic on the zero-extended
          // value; clz/ctz must be forced to the 16-bit answer (uint clz is +16;
          // uint ctz(0) is 32, but u16 wants 16).
          case OP_POPCNT: { uint x=(uint)stack[--sp]; stack[sp++]=(ushort)popcount(x); break; }
          case OP_CLZ:    { uint x=(uint)stack[--sp]; stack[sp++]=(ushort)(clz(x)-16u); break; }
          case OP_CTZ:    { uint x=(uint)stack[--sp]; stack[sp++]=(ushort)(x==0u?16u:ctz(x)); break; }
          // 32-bit ops: a u32 is two stack entries (low, then high on top).
          case OP_BIN32: {
             uint bh=stack[--sp], bl=stack[--sp], ah=stack[--sp], al=stack[--sp];
             uint a=al|(ah<<16), b=bl|(bh<<16);
             uint binop=arg&0xFFu; bool sg=((arg>>8)&1u)!=0u; uint res=0u;
             switch(binop){
               case 0: res=a+b; break;
               case 1: res=a-b; break;
               case 2: res=a*b; break;
               case 5: res=a|b; break;
               case 6: res=a&b; break;
               case 7: res=a^b; break;
               case 3: case 4: {
                  if(b==0u){ status=ST_DIV0; done=true; res=0u; }
                  else if(sg){
                     // guard MIN/-1 — 32-bit int div overflows (C UB), unlike 16-bit
                     bool ov=(a==0x80000000u && b==0xFFFFFFFFu);
                     if(binop==3u) res=ov?a:(uint)((int)a/(int)b);
                     else res=ov?0u:(uint)((int)a%(int)b);
                  } else { res=(binop==3u)?(a/b):(a%b); }
                  break;
               }
             }
             stack[sp++]=(ushort)(res&0xFFFFu); stack[sp++]=(ushort)(res>>16); break;
          }
          case OP_SHIFT32: {
             uint ah=stack[--sp], al=stack[--sp]; uint a=al|(ah<<16);
             uint k=arg&0xFFu; bool left=((arg>>16)&1u)!=0u; bool sg=((arg>>17)&1u)!=0u; uint res;
             if(sg && !left){ int sa=(int)a; uint kk=min(k,31u); res=(uint)(sa>>kk); }
             else if(k>=32u){ res=0u; }
             else if(left){ res=a<<k; }
             else { res=a>>k; }
             stack[sp++]=(ushort)(res&0xFFFFu); stack[sp++]=(ushort)(res>>16); break;
          }
          case OP_CMP32: {
             uint bh=stack[--sp], bl=stack[--sp], ah=stack[--sp], al=stack[--sp];
             uint a=al|(ah<<16), b=bl|(bh<<16);
             uint cmp=arg&0xFFu; bool sg=((arg>>8)&1u)!=0u; bool r;
             if(sg && cmp<4u){ int sa=(int)a, sb=(int)b;
                switch(cmp){case 0:r=sa<sb;break;case 1:r=sa<=sb;break;case 2:r=sa>sb;break;default:r=sa>=sb;break;}
             } else {
                switch(cmp){case 0:r=a<b;break;case 1:r=a<=b;break;case 2:r=a>b;break;case 3:r=a>=b;break;case 4:r=a==b;break;default:r=a!=b;break;}
             }
             stack[sp++]=r?1:0; break;
          }
          case OP_SEXTHI: {
             ushort lo=stack[--sp]; stack[sp++]=lo;
             stack[sp++]=((lo&0x8000u)!=0u)?(ushort)0xFFFFu:(ushort)0u; break;
          }
          case OP_STORE: slots[arg]=stack[--sp]; break;
          case OP_POP: --sp; break;
          case OP_DUP: { ushort v=stack[sp-1]; stack[sp++]=v; break; }
          case OP_JMPZERO: { ushort v=stack[--sp]; if(v==0){ pc=code_off+arg; continue; } break; }
          case OP_JMP: pc=code_off+arg; continue;
          case OP_RET: {
             uint arity=arg;
             if(arity>=1u) r0=stack[0];
             if(arity>=2u) r1=stack[1];
             if(arity>=3u) r2=stack[2];
             done=true; break;
          }
          case OP_HALT: { r0=stack[sp-1]; status=ST_HALT; done=true; break; }
          default: done=true; break;
        }
        pc++;
    }
    uint base=(cell*n_probes+p)*6u;
    out[base+0]=r0; out[base+1]=r1; out[base+2]=r2;
    out[base+3]=(ushort)status;
    out[base+4]=(ushort)(steps&0xFFFFu);
    out[base+5]=(ushort)(steps>>16);
}
"#;

    /// Kernel slot-array bound — cells needing more locals are skipped at build.
    const MAX_LOCALS: usize = 64;

    /// A whole library compiled for GPU dispatch: the fixed interpreter kernel
    /// plus the concatenated bytecode + per-cell offset table for a set of
    /// [`CellProgram`]s. One threadgroup per cell, probes across lanes — kernel
    /// size is constant in the number of cells (the point of this backend). The
    /// sextet grid is cell-major: `cell * probes.len() + probe`.
    pub struct InterpBatch {
        device: Device,
        queue: CommandQueue,
        pipeline: ComputePipelineState,
        code_buf: Buffer,
        table_buf: Buffer,
        n_cells: usize,
        max_tpg: usize,
    }

    impl InterpBatch {
        /// Build from linearized cells. Cells whose local count exceeds the
        /// kernel bound are skipped; the count of skipped cells is returned
        /// alongside the batch. `n_cells()` reflects the admitted cells.
        pub fn new(progs: &[CellProgram]) -> Result<(Self, usize), String> {
            let device =
                Device::system_default().ok_or_else(|| "msl: no Metal device".to_string())?;
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
            let n_cells = table.len() / 3;
            let opts = metal::CompileOptions::new();
            opts.set_fast_math_enabled(false);
            opts.set_language_version(metal::MTLLanguageVersion::V3_1);
            let lib = device
                .new_library_with_source(KERNEL, &opts)
                .map_err(|e| format!("msl interp: kernel compile failed: {e}"))?;
            let func = lib
                .get_function("interp", None)
                .map_err(|e| format!("msl interp: missing kernel: {e}"))?;
            let pipeline = device
                .new_compute_pipeline_state_with_function(&func)
                .map_err(|e| format!("msl interp: pipeline creation failed: {e}"))?;
            let queue = device.new_command_queue();
            let max_tpg = pipeline.max_total_threads_per_threadgroup() as usize;
            // Metal wants ≥1 byte even for an empty library.
            let mk = |v: &[u32]| {
                let bytes: &[u32] = if v.is_empty() { &[0] } else { v };
                device.new_buffer_with_data(
                    bytes.as_ptr() as *const _,
                    (bytes.len() * 4) as u64,
                    MTLResourceOptions::StorageModeShared,
                )
            };
            let code_buf = mk(&code);
            let table_buf = mk(&table);
            Ok((
                InterpBatch { device, queue, pipeline, code_buf, table_buf, n_cells, max_tpg },
                skipped,
            ))
        }

        /// Admitted cell count (skipped cells excluded).
        pub fn n_cells(&self) -> usize {
            self.n_cells
        }

        /// Run every admitted cell against every probe in one dispatch. Returns
        /// the sextets `[r0, r1, r2, status, steps_lo, steps_hi]`, cell-major.
        pub fn run(&self, probes: &[[u16; IN_STRIDE]]) -> Vec<[u16; OUT_STRIDE]> {
            if self.n_cells == 0 || probes.is_empty() {
                return Vec::new();
            }
            let flat: Vec<u16> = probes.iter().flat_map(|p| p.iter().copied()).collect();
            let probe_buf = self.device.new_buffer_with_data(
                flat.as_ptr() as *const _,
                (flat.len() * 2) as u64,
                MTLResourceOptions::StorageModeShared,
            );
            let out_buf = self.device.new_buffer(
                (self.n_cells * probes.len() * OUT_STRIDE * 2) as u64,
                MTLResourceOptions::StorageModeShared,
            );
            let n_probes = probes.len() as u32;
            let tpg = probes.len().min(self.max_tpg); // probes across lanes
            let cmd = self.queue.new_command_buffer();
            let enc = cmd.new_compute_command_encoder();
            enc.set_compute_pipeline_state(&self.pipeline);
            enc.set_buffer(0, Some(&self.code_buf), 0);
            enc.set_buffer(1, Some(&self.table_buf), 0);
            enc.set_buffer(2, Some(&probe_buf), 0);
            enc.set_buffer(3, Some(&out_buf), 0);
            enc.set_bytes(4, 4, &n_probes as *const u32 as *const _);
            enc.dispatch_thread_groups(
                MTLSize::new(self.n_cells as u64, 1, 1),
                MTLSize::new(tpg as u64, 1, 1),
            );
            enc.end_encoding();
            cmd.commit();
            cmd.wait_until_completed();
            let n = self.n_cells * probes.len();
            unsafe { std::slice::from_raw_parts(out_buf.contents() as *const [u16; OUT_STRIDE], n) }
                .to_vec()
        }
    }
}

#[cfg(target_os = "macos")]
pub use gpu::InterpBatch;

#[cfg(test)]
mod tests {
    use super::*;
    use cell80_core::ir::{BinOp, Expr, Func, Stmt};
    use cell80_core::{Interp, Target};

    /// Build a one-function library whose `run` returns `ret`.
    fn cell(params: usize, n_locals: usize, body: Vec<Stmt>, ret: Expr) -> Vec<(String, Func)> {
        vec![(
            "run".into(),
            Func { params, n_locals, body, ret: vec![ret], wide_param: false, wide_second: false, wide_ret: false },
        )]
    }

    /// cpu_run must match Interp bit-for-bit (values AND steps) on `args`.
    fn assert_parity(funcs: &[(String, Func)], args: &[u16]) {
        let prog = linearize(funcs, "run").expect("linearizes");
        let mut interp = Interp::new(funcs, Vec::<(&str, &[u8])>::new(), Target::Cell.descriptor());
        let iref = interp.run("run", args);
        let isteps = interp.steps();
        match (iref, cpu_run(&prog, args)) {
            (Ok(v), VmOut::Value(o, s)) => {
                assert_eq!(v, o, "values @ {args:?}");
                assert_eq!(isteps, s, "steps @ {args:?}");
            }
            (Err(e), out) if e.contains("divide by zero") => assert!(matches!(out, VmOut::DivZero)),
            (a, b) => panic!("mismatch @ {args:?}: interp={a:?} vm={b:?}"),
        }
    }

    #[test]
    fn arithmetic_and_steps() {
        // run(x, y) = (x + y) * x   over Word
        let add = Expr::Bin(BinOp::Add, Box::new(Expr::Var(0)), Box::new(Expr::Var(1)), Width::Word);
        let mul = Expr::Bin(BinOp::Mul, Box::new(add), Box::new(Expr::Var(0)), Width::Word);
        let c = cell(2, 2, vec![], mul);
        for args in [[3u16, 4], [0, 0], [65535, 1], [12345, 6789]] {
            assert_parity(&c, &args);
        }
    }

    #[test]
    fn div_by_zero_traps() {
        // run(x) = x / (x - x)  — always divide by zero
        let z = Expr::Bin(BinOp::Sub, Box::new(Expr::Var(0)), Box::new(Expr::Var(0)), Width::Word);
        let d = Expr::Bin(BinOp::Div, Box::new(Expr::Var(0)), Box::new(z), Width::Word);
        assert_parity(&cell(1, 1, vec![], d), &[7]);
    }

    #[test]
    fn signed_min_div_neg_one_wraps() {
        // i16::MIN / -1 wraps to i16::MIN (0x8000), not a trap.
        let d = Expr::Bin(BinOp::Div, Box::new(Expr::Lit(0x8000)), Box::new(Expr::Lit(0xFFFF)), Width::SWord);
        assert_parity(&cell(1, 1, vec![], d), &[0]);
    }

    #[test]
    fn loop_and_control_flow() {
        // run(n): s=0; i=0; while i<n { s = s + i; i = i + 1 } ; return s
        use cell80_core::ir::{Cmp, Cond};
        let cond = Cond { cmp: Cmp::Lt, lhs: Expr::Var(2), rhs: Expr::Var(0), signed: false };
        let body = vec![
            Stmt::Assign(1, Expr::Bin(BinOp::Add, Box::new(Expr::Var(1)), Box::new(Expr::Var(2)), Width::Word)),
            Stmt::Assign(2, Expr::Bin(BinOp::Add, Box::new(Expr::Var(2)), Box::new(Expr::Lit(1)), Width::Word)),
        ];
        let c = cell(1, 3, vec![Stmt::Assign(1, Expr::Lit(0)), Stmt::Assign(2, Expr::Lit(0)), Stmt::While(cond, body)], Expr::Var(1));
        for n in [0u16, 1, 5, 100] {
            assert_parity(&c, &[n]);
        }
    }

    #[test]
    fn bits_intrinsic() {
        let call = Expr::Call("__bits_count_ones".into(), vec![Expr::Var(0)]);
        for x in [0u16, 1, 0xF0F0, 0xFFFF] {
            assert_parity(&cell(1, 1, vec![], call.clone()), &[x]);
        }
    }
}
