//! The linearizer: lowers inlined IR (`cell80_core::ir::{Func, Stmt, Expr}`) to
//! flat [`Inst`] bytecode, emitting one step-charge per tree-walker tick so cost
//! parity with `Interp` holds by construction rather than by re-derivation.

use super::bytecode::{Bail, CellProgram, Inst, STACK_CAP};
use cell80_core::ir::{BinOp, Cond, Expr, Func, Stmt, Width};

/// The linearizer: emits code while tracking operand-stack height and holding
/// break/continue targets. Labels are allocated up front and resolved to
/// instruction indices in a final fixup pass.
struct Lin<'a> {
    funcs: &'a [(String, Func)], // for full inlining of residual calls (note 2)
    code: Vec<Inst>,
    labels: Vec<usize>,         // label id → instruction index (filled by `place`)
    loops: Vec<(usize, usize)>, // (continue target, break target) per enclosing loop
    ret_ctx: Vec<(usize, bool)>, // (inline-return label, callee returns u32); empty ⇒ top level ⇒ Ret
    frame_base: usize,           // current frame's slot offset (0 ⇒ entry frame)
    slots_used: usize,           // high-water mark of allocated slots
    wide_ret: bool,              // entry returns u32 (Ret produces 2 words)
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
            Expr::Cmp {
                cmp,
                lhs,
                rhs,
                signed,
            } => {
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
                _ => {
                    // A wide-returning call in 16-bit position yields its low word.
                    if self.inline_call(name, args)? {
                        self.emit(Inst::Pop);
                    }
                }
            },
            // 32-bit → 16-bit bridges (evaluated by eval16): node Step (above) +
            // the wide subtree's ticks, then narrow the result.
            Expr::Trunc32(inner) => {
                self.expr32(inner)?;
                self.emit(Inst::Pop); // drop the high word, keep low
            }
            Expr::Cmp32 {
                cmp,
                lhs,
                rhs,
                signed,
            } => {
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
            Expr::ShiftVar { .. } => {
                return Err(Bail::UnsupportedExpr("ShiftVar (runtime amount)"))
            }
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
            Expr::Shift32 {
                left,
                e: inner,
                k,
                signed,
            } => {
                self.expr32(inner)?;
                self.emit(Inst::Shift32 {
                    left: *left,
                    k: *k,
                    signed: *signed,
                });
            }
            // A call in u32 position: inline it; it must be wide-returning
            // (`Interp` errors on a narrow call in a u32 context).
            Expr::Call(name, args) => {
                if !self.inline_call(name, args)? {
                    return Err(Bail::ResidualCall);
                }
            }
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
                Some((end, wide)) => {
                    let e = val.as_ref().ok_or(Bail::ResidualCall)?; // void return in a value callee
                    if wide {
                        self.expr32(e)?; // wide-returning callee: leave a u32
                    } else {
                        self.expr(e)?;
                    }
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
                let (_, end) = *self
                    .loops
                    .last()
                    .ok_or(Bail::UnsupportedStmt("break outside loop"))?;
                self.emit(Inst::Jmp(end));
            }
            Stmt::Continue => {
                let (top, _) = *self
                    .loops
                    .last()
                    .ok_or(Bail::UnsupportedStmt("continue outside loop"))?;
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

    /// Fully inline a call in expression position — no call stack (note 2).
    /// Returns whether the result is wide (u32, two stack entries). The tick
    /// accounting mirrors `Interp::call`: the call node's `Step` is already
    /// emitted by the caller; each arg's eval ticks as a *caller* node (`expr32`
    /// for a `wide_param`); the param-binding stores do NOT tick; the callee body
    /// ticks normally; and the fall-through return evals `ret` (ticking, `expr32`
    /// when `wide_ret`) with no statement tick. The callee's frame is a fresh slot
    /// range so its locals never alias the caller's.
    fn inline_call(&mut self, name: &str, args: &[Expr]) -> Result<bool, Bail> {
        let callee = self
            .funcs
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, f)| f)
            .ok_or(Bail::ResidualCall)?;
        // `wide_second` (the __mul32 two-wide-param stack shape) isn't handled.
        if callee.wide_second || callee.ret.len() != 1 {
            return Err(Bail::WideValue);
        }
        // Param slot layout: a `wide_param` makes arg0 a u32 (two slots); the rest
        // are narrow. Verify the plan fills exactly the callee's param slots.
        let mut plan: Vec<(usize, bool)> = Vec::new(); // (slot offset in callee frame, wide)
        let mut slot = 0usize;
        for i in 0..args.len() {
            let wide = i == 0 && callee.wide_param;
            plan.push((slot, wide));
            slot += if wide { 2 } else { 1 };
        }
        if slot != callee.params {
            return Err(Bail::ResidualCall);
        }
        let base = self.slots_used;
        self.slots_used += callee.n_locals;
        // Evaluate every arg (in the CALLER frame) before binding any param, then
        // store into the fresh callee frame in reverse (stack order) — no ticks.
        for (a, &(_, wide)) in args.iter().zip(&plan) {
            if wide {
                self.expr32(a)?;
            } else {
                self.expr(a)?;
            }
        }
        for &(off, wide) in plan.iter().rev() {
            if wide {
                self.emit(Inst::Store(base + off + 1)); // hi (top of stack)
                self.emit(Inst::Store(base + off)); // lo
            } else {
                self.emit(Inst::Store(base + off));
            }
        }
        // Inline the body in the callee frame; a callee `return` jumps to `end`,
        // leaving its value (narrow or wide per `wide_ret`) on the stack.
        let saved = self.frame_base;
        self.frame_base = base;
        let end = self.new_label();
        self.ret_ctx.push((end, callee.wide_ret));
        self.block(&callee.body)?;
        self.ret_ctx.pop();
        // Fall-through return (dead if the body always returned — its `Step`s then
        // never execute, so parity holds).
        if callee.wide_ret {
            self.expr32(&callee.ret[0])?;
        } else {
            self.expr(&callee.ret[0])?;
        }
        self.place(end);
        self.frame_base = saved;
        Ok(callee.wide_ret)
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
