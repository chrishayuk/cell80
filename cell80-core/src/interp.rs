//! The reference IR interpreter (Phase 5 WS-A/A4 — `docs/13-multi-target-spec.md`
//! §2.3): a direct, backend-independent executor for the typed IR. Three jobs:
//! it checks IR semantics before a backend exists (signed widening lands against it
//! first), it is the semantic anchor for the family hash ("same cell, N bodies" is
//! checkable against one executable definition), and it stands as a fourth adversary
//! in the verification matrix — the diff battery runs every `check!` against it
//! alongside both Z80 targets and the rustc oracle.
//!
//! **Fidelity contract.** The interpreter mirrors the *observable* semantics the
//! diff battery pins — including the memory image: it executes against a flat 64 KiB
//! byte array with locals at the descriptor's scratch base (slot `i` of a function at
//! `scratch + (base + i) * 2`, little-endian), so `AddrOf`/`Deref`/`Peek`/`Poke`
//! alias exactly as on the Z80, and a memory-effect comparison masks only the
//! execution substrate (code image, trampoline, hardware stack) the interpreter
//! doesn't have. Evaluation is left-to-right throughout — the canonical order
//! codegen shares since A2a (side-effecting operand pairs evaluate in source order;
//! effect-free pairs may reorder, which no program can observe). It is a
//! *reference*, not a deployment target: no cycle model, no peephole, deliberately
//! naive.
//!
//! Two Z80-target behaviours have no single IR meaning and surface as errors here:
//! `halt(code)` (a Cell trap, a Spectrum no-op — the interpreter stops with the
//! code, the Cell reading) and divide-by-zero (rustc panics; the battery never
//! exercises it).

use crate::descriptor::TargetDescriptor;
use crate::ir::*;
use std::collections::HashMap;

/// Execution fuel: one unit per statement and per loop iteration. The battery's
/// heaviest programs (softfloat kernels) burn a few thousand; the guard exists so a
/// lowering bug can't hang the suite.
const FUEL: u64 = 100_000_000;

/// A call's outcome: up to three scalar registers' worth, or one wide value.
enum CallOut {
    Scalars(Vec<u16>),
    Wide(u32),
}

/// Statement-level control flow.
enum Flow {
    Normal,
    Break,
    Continue,
    Return(Option<RetVal>),
}

enum RetVal {
    Narrow(u16),
    Wide(u32),
}

/// The per-call frame context: where the function's slots live and how it returns.
#[derive(Clone, Copy)]
struct Frame {
    /// Slot base index — slot `i` lives at `scratch + (base + i) * 2`.
    base: u16,
    wide_ret: bool,
}

pub struct Interp<'p> {
    funcs: HashMap<&'p str, (&'p Func, Frame)>,
    pub mem: Vec<u8>,
    scratch: u16,
    consts: HashMap<&'p str, u16>,
    fuel: u64,
}

impl<'p> Interp<'p> {
    /// Build an interpreter over lowered functions + const data (`(name, bytes)`
    /// pairs — the lowering's `DataConst` pool, projected). Frames are laid in
    /// `funcs` order with a running base, and const data is laid from the
    /// descriptor's `org` — the same assignment rule as codegen, so slot addresses
    /// (and therefore `AddrOf`/`Deref` aliasing and the memory image) match the
    /// compiled program wherever the program can observe them.
    pub fn new(
        funcs: &'p [(String, Func)],
        consts: impl IntoIterator<Item = (&'p str, &'p [u8])>,
        desc: &TargetDescriptor,
    ) -> Self {
        let mut map = HashMap::new();
        let mut base = 0u16;
        for (name, f) in funcs {
            map.insert(
                name.as_str(),
                (
                    f,
                    Frame {
                        base,
                        wide_ret: f.wide_ret,
                    },
                ),
            );
            base += f.n_locals as u16;
        }
        let mut mem = vec![0u8; 0x1_0000];
        let mut const_map = HashMap::new();
        let mut at = desc.org;
        for (name, bytes) in consts {
            const_map.insert(name, at);
            mem[at as usize..at as usize + bytes.len()].copy_from_slice(bytes);
            at = at.wrapping_add(bytes.len() as u16);
        }
        Interp {
            funcs: map,
            mem,
            scratch: desc.scratch,
            consts: const_map,
            fuel: FUEL,
        }
    }

    /// Pre-lay input data (the `run_str` buffer pattern).
    pub fn plant(&mut self, addr: u16, bytes: &[u8]) {
        self.mem[addr as usize..addr as usize + bytes.len()].copy_from_slice(bytes);
    }

    // ── memory ──────────────────────────────────────────────────────────────────

    fn rd8(&self, a: u16) -> u16 {
        self.mem[a as usize] as u16
    }
    fn wr8(&mut self, a: u16, v: u8) {
        self.mem[a as usize] = v;
    }
    fn rd16(&self, a: u16) -> u16 {
        u16::from_le_bytes([self.mem[a as usize], self.mem[a.wrapping_add(1) as usize]])
    }
    fn wr16(&mut self, a: u16, v: u16) {
        self.wr8(a, v as u8);
        self.wr8(a.wrapping_add(1), (v >> 8) as u8);
    }
    fn rd32(&self, a: u16) -> u32 {
        self.rd16(a) as u32 | (self.rd16(a.wrapping_add(2)) as u32) << 16
    }
    fn wr32(&mut self, a: u16, v: u32) {
        self.wr16(a, v as u16);
        self.wr16(a.wrapping_add(2), (v >> 16) as u16);
    }

    fn slot_addr(&self, fr: Frame, slot: usize) -> u16 {
        self.scratch
            .wrapping_add(fr.base.wrapping_add(slot as u16).wrapping_mul(2))
    }

    // ── entry ───────────────────────────────────────────────────────────────────

    /// Run `entry` with 16-bit register args (the `HL`/`DE`/`BC` convention) and
    /// return its result registers: `ret` arity values, a wide return as
    /// `[low, high]`.
    pub fn run(&mut self, entry: &str, args: &[u16]) -> Result<Vec<u16>, String> {
        let (f, fr) = *self
            .funcs
            .get(entry)
            .ok_or_else(|| format!("interp: unknown entry `{entry}`"))?;
        for (i, &v) in args.iter().enumerate().take(f.params) {
            let addr = self.slot_addr(fr, i);
            self.wr16(addr, v);
        }
        match self.exec_fn(f, fr)? {
            CallOut::Wide(v) => Ok(vec![v as u16, (v >> 16) as u16]),
            CallOut::Scalars(v) => Ok(v),
        }
    }

    // ── calls ───────────────────────────────────────────────────────────────────

    fn exec_fn(&mut self, f: &'p Func, fr: Frame) -> Result<CallOut, String> {
        match self.exec_stmts(fr, &f.body)? {
            Flow::Return(v) => Ok(match v {
                Some(RetVal::Wide(v)) => CallOut::Wide(v),
                Some(RetVal::Narrow(v)) => CallOut::Scalars(vec![v]),
                None => CallOut::Scalars(vec![]),
            }),
            Flow::Break | Flow::Continue => Err("interp: break/continue escaped a loop".into()),
            Flow::Normal => {
                if fr.wide_ret {
                    Ok(CallOut::Wide(self.eval32(fr, &f.ret[0])?))
                } else {
                    let mut out = Vec::with_capacity(f.ret.len());
                    for e in &f.ret {
                        out.push(self.eval16(fr, e)?);
                    }
                    Ok(CallOut::Scalars(out))
                }
            }
        }
    }

    fn call(&mut self, caller: Frame, name: &str, args: &[Expr]) -> Result<CallOut, String> {
        // The bit-method kernels are reserved names lowered as calls but appended as
        // machine-code blobs — the interpreter owns their (rustc-identical) semantics.
        if let Some(builtin) = match name {
            "__bits_count_ones" => Some(u16::count_ones as fn(u16) -> u32),
            "__bits_leading_zeros" => Some(u16::leading_zeros as fn(u16) -> u32),
            "__bits_trailing_zeros" => Some(u16::trailing_zeros as fn(u16) -> u32),
            _ => None,
        } {
            let x = self.eval16(caller, &args[0])?;
            return Ok(CallOut::Scalars(vec![builtin(x) as u16]));
        }
        let (f, fr) = *self
            .funcs
            .get(name)
            .ok_or_else(|| format!("interp: call to unknown fn `{name}`"))?;
        // Evaluate every argument in the caller's frame *before* writing any callee
        // slot (codegen pushes all values first) — an argument that aliases the
        // callee's slots through a pointer must read pre-call values.
        if f.wide_second {
            let a0 = self.eval32(caller, &args[0])?;
            let a1 = self.eval32(caller, &args[1])?;
            let a2 = match args.get(2) {
                Some(e) => Some(self.eval16(caller, e)?),
                None => None,
            };
            self.wr32(self.slot_addr(fr, 0), a0);
            self.wr32(self.slot_addr(fr, 2), a1);
            if let Some(v) = a2 {
                self.wr16(self.slot_addr(fr, 4), v);
            }
        } else if f.wide_param {
            let a0 = self.eval32(caller, &args[0])?;
            let a1 = match args.get(1) {
                Some(e) => Some(self.eval16(caller, e)?),
                None => None,
            };
            self.wr32(self.slot_addr(fr, 0), a0);
            if let Some(v) = a1 {
                self.wr16(self.slot_addr(fr, 2), v);
            }
        } else {
            let mut vals = Vec::with_capacity(args.len());
            for e in args {
                vals.push(self.eval16(caller, e)?);
            }
            for (i, v) in vals.into_iter().enumerate() {
                let addr = self.slot_addr(fr, i);
                self.wr16(addr, v);
            }
        }
        self.exec_fn(f, fr)
    }

    // ── statements ──────────────────────────────────────────────────────────────

    fn exec_stmts(&mut self, fr: Frame, stmts: &[Stmt]) -> Result<Flow, String> {
        for s in stmts {
            match self.exec_stmt(fr, s)? {
                Flow::Normal => {}
                f => return Ok(f),
            }
        }
        Ok(Flow::Normal)
    }

    fn tick(&mut self) -> Result<(), String> {
        self.fuel -= 1;
        if self.fuel == 0 {
            return Err("interp: fuel exhausted (runaway loop?)".into());
        }
        Ok(())
    }

    fn exec_stmt(&mut self, fr: Frame, s: &Stmt) -> Result<Flow, String> {
        self.tick()?;
        match s {
            Stmt::Assign(slot, e) => {
                let v = self.eval16(fr, e)?;
                let addr = self.slot_addr(fr, *slot);
                self.wr16(addr, v);
            }
            // Codegen stores the low byte always and the high byte only for `Word` —
            // mirrored literally (an i16/byte element store leaves the high byte).
            Stmt::StoreIndex(base, index, value, w) => {
                let v = self.eval16(fr, value)?;
                let i = self.eval16(fr, index)?;
                let addr = self.slot_addr(fr, *base).wrapping_add(i.wrapping_mul(2));
                self.wr8(addr, v as u8);
                if *w == Width::Word {
                    self.wr8(addr.wrapping_add(1), (v >> 8) as u8);
                }
            }
            Stmt::Poke(addr, value) => {
                let v = self.eval16(fr, value)?;
                let a = self.eval16(fr, addr)?;
                self.wr8(a, v as u8);
            }
            Stmt::Store(ptr, off, value) => {
                let v = self.eval16(fr, value)?;
                let p = self.eval16(fr, ptr)?.wrapping_add(*off as u16);
                self.wr16(p, v);
            }
            Stmt::PtrStoreIndex {
                ptr,
                off,
                index,
                value,
            } => {
                let v = self.eval16(fr, value)?;
                let i = self.eval16(fr, index)?;
                let p = self
                    .eval16(fr, ptr)?
                    .wrapping_add(*off as u16)
                    .wrapping_add(i.wrapping_mul(2));
                self.wr16(p, v);
            }
            Stmt::StoreAt(addr, value, w) => {
                let v = self.eval16(fr, value)?;
                let a = self.eval16(fr, addr)?;
                self.wr8(a, v as u8);
                if *w == Width::Word {
                    self.wr8(a.wrapping_add(1), (v >> 8) as u8);
                }
            }
            Stmt::Assign32(slot, e) => {
                let v = self.eval32(fr, e)?;
                let addr = self.slot_addr(fr, *slot);
                self.wr32(addr, v);
            }
            Stmt::Store32(ptr, off, value) => {
                let v = self.eval32(fr, value)?;
                let p = self.eval16(fr, ptr)?.wrapping_add(*off as u16);
                self.wr32(p, v);
            }
            Stmt::Fill { base, count, value } => {
                if *count > 0 {
                    let v = self.eval16(fr, value)?;
                    let base_addr = self.slot_addr(fr, *base);
                    for i in 0..*count {
                        self.wr16(base_addr.wrapping_add(i as u16 * 2), v);
                    }
                }
            }
            Stmt::Eval(e) => {
                self.eval16(fr, e)?;
            }
            Stmt::AssignTuple(slots, call) => {
                let Expr::Call(name, args) = call else {
                    return Err("interp: AssignTuple of a non-call".into());
                };
                match self.call(fr, name, args)? {
                    CallOut::Scalars(vals) => {
                        for (slot, v) in slots.iter().zip(vals) {
                            let addr = self.slot_addr(fr, *slot);
                            self.wr16(addr, v);
                        }
                    }
                    CallOut::Wide(_) => {
                        return Err("interp: AssignTuple of a wide return".into());
                    }
                }
            }
            Stmt::If(cond, then, els) => {
                let branch = if self.cond_true(fr, cond)? { then } else { els };
                return self.exec_stmts(fr, branch);
            }
            Stmt::While(cond, body) => loop {
                self.tick()?;
                if !self.cond_true(fr, cond)? {
                    break;
                }
                match self.exec_stmts(fr, body)? {
                    Flow::Normal | Flow::Continue => {}
                    Flow::Break => break,
                    f @ Flow::Return(_) => return Ok(f),
                }
            },
            Stmt::Loop(body) => loop {
                self.tick()?;
                match self.exec_stmts(fr, body)? {
                    Flow::Normal | Flow::Continue => {}
                    Flow::Break => break,
                    f @ Flow::Return(_) => return Ok(f),
                }
            },
            Stmt::ForRange {
                var,
                end,
                inclusive,
                width,
                body,
            } => {
                let var_addr = self.slot_addr(fr, *var);
                loop {
                    self.tick()?;
                    let v = self.rd16(var_addr);
                    let bound = self.eval16(fr, end)?;
                    let keep = if *width == Width::SWord {
                        let (v, b) = (v as i16, bound as i16);
                        if *inclusive {
                            v <= b
                        } else {
                            v < b
                        }
                    } else if *inclusive {
                        v <= bound
                    } else {
                        v < bound
                    };
                    if !keep {
                        break;
                    }
                    match self.exec_stmts(fr, body)? {
                        Flow::Normal | Flow::Continue => {}
                        Flow::Break => break,
                        f @ Flow::Return(_) => return Ok(f),
                    }
                    // The induction step re-reads the slot (the body may assign the
                    // loop variable) and masks to the variable's width.
                    let mut next = self.rd16(var_addr).wrapping_add(1);
                    if *width == Width::Byte {
                        next &= 0xFF;
                    }
                    self.wr16(var_addr, next);
                }
            }
            Stmt::Break => return Ok(Flow::Break),
            Stmt::Continue => return Ok(Flow::Continue),
            Stmt::Return(val) => {
                let v = match val {
                    None => None,
                    Some(e) if fr.wide_ret => Some(RetVal::Wide(self.eval32(fr, e)?)),
                    Some(e) => Some(RetVal::Narrow(self.eval16(fr, e)?)),
                };
                return Ok(Flow::Return(v));
            }
        }
        Ok(Flow::Normal)
    }

    fn cond_true(&mut self, fr: Frame, cond: &Cond) -> Result<bool, String> {
        let l = self.eval16(fr, &cond.lhs)?;
        let r = self.eval16(fr, &cond.rhs)?;
        Ok(cmp16(cond.cmp, l, r, cond.signed))
    }

    // ── 16-bit expressions ──────────────────────────────────────────────────────

    fn eval16(&mut self, fr: Frame, e: &Expr) -> Result<u16, String> {
        self.tick()?;
        Ok(match e {
            Expr::Lit(n) => *n,
            Expr::Var(slot) => {
                let a = self.slot_addr(fr, *slot);
                self.rd16(a)
            }
            Expr::Bin(op, l, r, w) => {
                let lv = self.eval16(fr, l)?;
                // Shift amounts are literal by construction — not evaluated as an
                // operand (codegen unrolls them).
                let raw = match op {
                    BinOp::Add => lv.wrapping_add(self.eval16(fr, r)?),
                    BinOp::Sub => lv.wrapping_sub(self.eval16(fr, r)?),
                    BinOp::Mul => lv.wrapping_mul(self.eval16(fr, r)?),
                    BinOp::Div | BinOp::Rem => {
                        let rv = self.eval16(fr, r)?;
                        if rv == 0 {
                            return Err("interp: divide by zero".into());
                        }
                        match (op, *w == Width::SWord) {
                            // Signed: truncate toward zero, remainder takes the
                            // dividend's sign (rustc semantics; MIN/-1 wraps like
                            // the abs-through-unsigned kernel).
                            (BinOp::Div, true) => (lv as i16).wrapping_div(rv as i16) as u16,
                            (BinOp::Rem, true) => (lv as i16).wrapping_rem(rv as i16) as u16,
                            (BinOp::Div, false) => lv / rv,
                            (BinOp::Rem, false) => lv % rv,
                            _ => unreachable!(),
                        }
                    }
                    BinOp::Or => lv | self.eval16(fr, r)?,
                    BinOp::And => lv & self.eval16(fr, r)?,
                    BinOp::Xor => lv ^ self.eval16(fr, r)?,
                    BinOp::Shl => {
                        let k = lit_shift(r)?;
                        if k >= 16 {
                            0
                        } else {
                            lv << k
                        }
                    }
                    BinOp::Shr if *w == Width::SWord => {
                        // Arithmetic: per-step SRA saturates at the sign fill.
                        ((lv as i16) >> lit_shift(r)?.min(15)) as u16
                    }
                    BinOp::Shr => {
                        let k = lit_shift(r)?;
                        if k >= 16 {
                            0
                        } else {
                            lv >> k
                        }
                    }
                };
                mask(raw, *w)
            }
            Expr::Index(base, index, w) => {
                let i = self.eval16(fr, index)?;
                let addr = self.slot_addr(fr, *base).wrapping_add(i.wrapping_mul(2));
                match w {
                    Width::Byte => self.rd8(addr),
                    _ => self.rd16(addr),
                }
            }
            Expr::Call(name, args) => match self.call(fr, name, args)? {
                CallOut::Scalars(v) => v.first().copied().unwrap_or(0),
                // A wide-returning call in 16-bit position leaves its low word
                // (codegen: the value sits in HL:DE, HL is what's consumed).
                CallOut::Wide(v) => v as u16,
            },
            Expr::Trunc(e) => self.eval16(fr, e)? & 0xFF,
            Expr::Peek(addr) => {
                let a = self.eval16(fr, addr)?;
                self.rd8(a)
            }
            // The harness bus answers every port with 0xFF; the interpreter is the
            // same closed world.
            Expr::InPort(port) => {
                self.eval16(fr, port)?;
                0xFF
            }
            Expr::AddrOf(slot) => self.slot_addr(fr, *slot),
            Expr::ConstAddr(name) => *self
                .consts
                .get(name.as_str())
                .ok_or_else(|| format!("interp: unknown const `{name}`"))?,
            Expr::Deref(ptr, off) => {
                let p = self.eval16(fr, ptr)?.wrapping_add(*off as u16);
                self.rd16(p)
            }
            Expr::PtrIndex { ptr, off, index } => {
                let p = self.eval16(fr, ptr)?;
                let i = self.eval16(fr, index)?;
                self.rd16(p.wrapping_add(*off as u16).wrapping_add(i.wrapping_mul(2)))
            }
            Expr::MulConst(e, k) => self.eval16(fr, e)?.wrapping_mul(*k),
            Expr::LoadAt(addr, w) => {
                let a = self.eval16(fr, addr)?;
                match w {
                    Width::Byte => self.rd8(a),
                    _ => self.rd16(a),
                }
            }
            Expr::Cmp {
                cmp,
                lhs,
                rhs,
                signed,
            } => {
                let l = self.eval16(fr, lhs)?;
                let r = self.eval16(fr, rhs)?;
                cmp16(*cmp, l, r, *signed) as u16
            }
            Expr::Logic { and, lhs, rhs } => {
                let l = self.eval16(fr, lhs)?;
                // Short-circuit: the left value *is* the result when it decides.
                if *and && l == 0 || !*and && l != 0 {
                    l
                } else {
                    self.eval16(fr, rhs)?
                }
            }
            Expr::Cmp32 {
                cmp,
                lhs,
                rhs,
                signed,
            } => {
                let l = self.eval32(fr, lhs)?;
                let r = self.eval32(fr, rhs)?;
                let t = if *signed {
                    let (l, r) = (l as i32, r as i32);
                    match cmp {
                        Cmp::Lt => l < r,
                        Cmp::Le => l <= r,
                        Cmp::Gt => l > r,
                        Cmp::Ge => l >= r,
                        Cmp::Eq => l == r,
                        Cmp::Ne => l != r,
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
                };
                t as u16
            }
            Expr::ShiftVar { left, e, amount, w } => {
                let mut v = self.eval16(fr, e)?;
                let count = self.eval16(fr, amount)? as u8;
                for _ in 0..count {
                    v = if *left {
                        v.wrapping_shl(1)
                    } else if *w == Width::SWord {
                        ((v as i16) >> 1) as u16
                    } else {
                        v >> 1
                    };
                }
                mask(v, *w)
            }
            Expr::Trunc32(e) => self.eval32(fr, e)? as u16,
            Expr::Halt(code) => {
                let c = self.eval16(fr, code)?;
                return Err(format!("interp: halt({c})"));
            }
            Expr::Lit32(_)
            | Expr::Var32(_)
            | Expr::Deref32(..)
            | Expr::Bin32(..)
            | Expr::Shift32 { .. }
            | Expr::Widen(..)
            | Expr::SignExtend(..) => {
                return Err("interp: u32 node in a 16-bit context".into());
            }
        })
    }

    // ── 32-bit expressions ──────────────────────────────────────────────────────

    fn eval32(&mut self, fr: Frame, e: &Expr) -> Result<u32, String> {
        self.tick()?;
        Ok(match e {
            Expr::Lit32(n) => *n,
            Expr::Var32(slot) => {
                let a = self.slot_addr(fr, *slot);
                self.rd32(a)
            }
            Expr::Deref32(ptr, off) => {
                let p = self.eval16(fr, ptr)?.wrapping_add(*off as u16);
                self.rd32(p)
            }
            // Identity in wide position (codegen re-evaluates wide) — the value's
            // low word is what a 16-bit consumer takes, via `eval16`'s arm.
            Expr::Trunc32(e) => self.eval32(fr, e)?,
            Expr::Call(name, args) => match self.call(fr, name, args)? {
                CallOut::Wide(v) => v,
                CallOut::Scalars(_) => {
                    return Err("interp: narrow call in a u32 context".into());
                }
            },
            Expr::Widen(inner) => self.eval16(fr, inner)? as u32,
            Expr::SignExtend(inner) => self.eval16(fr, inner)? as i16 as i32 as u32,
            Expr::Bin32(op, l, r, signed) => {
                let lv = self.eval32(fr, l)?;
                let rv = self.eval32(fr, r)?;
                match op {
                    BinOp::Add => lv.wrapping_add(rv),
                    BinOp::Sub => lv.wrapping_sub(rv),
                    BinOp::Mul => lv.wrapping_mul(rv),
                    BinOp::Div | BinOp::Rem => {
                        if rv == 0 {
                            return Err("interp: divide by zero".into());
                        }
                        // Signed: truncate toward zero, remainder takes the
                        // dividend's sign; MIN/-1 wraps (rustc's wrapping_*).
                        match (op, *signed) {
                            (BinOp::Div, true) => (lv as i32).wrapping_div(rv as i32) as u32,
                            (BinOp::Rem, true) => (lv as i32).wrapping_rem(rv as i32) as u32,
                            (BinOp::Div, false) => lv / rv,
                            (BinOp::Rem, false) => lv % rv,
                            _ => unreachable!(),
                        }
                    }
                    BinOp::Or => lv | rv,
                    BinOp::And => lv & rv,
                    BinOp::Xor => lv ^ rv,
                    BinOp::Shl | BinOp::Shr => {
                        return Err("interp: u32 shifts lower to Shift32".into());
                    }
                }
            }
            Expr::Shift32 { left, e, k, signed } => {
                let v = self.eval32(fr, e)?;
                if *signed && !*left {
                    // Arithmetic: sign-propagating; a count ≥ 32 saturates at the
                    // sign fill (the per-step reading).
                    ((v as i32) >> (*k).min(31) as u32) as u32
                } else if *k >= 32 {
                    0
                } else if *left {
                    v << k
                } else {
                    v >> k
                }
            }
            _ => return Err("interp: not a u32 expression".into()),
        })
    }
}

/// Mask a raw 16-bit result to its width (`Byte` wraps mod 256; the others are
/// already 16-bit).
fn mask(v: u16, w: Width) -> u16 {
    if w == Width::Byte {
        v & 0xFF
    } else {
        v
    }
}

/// A shift amount operand — literal by construction.
fn lit_shift(e: &Expr) -> Result<u32, String> {
    match e {
        Expr::Lit(k) => Ok(*k as u32),
        _ => Err("interp: shift amount must be a constant".into()),
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
