//! RV32 codegen (WS-B/B1): naive, correct-first lowering of the `cell80-core`
//! typed IR — the same strategy that shipped rustz80 (memory-slot register file,
//! a couple of working registers, no allocator until profiling earns one).
//!
//! **The data model is the interpreter's.** All IR pointer values are 16-bit
//! addresses into a 64 KiB data window based at `s0`: consts at `0x8000`, the
//! slot file at `0x9000` (slot `i` of a function at `0x9000 + (base + i)·2`,
//! little-endian — the family-wide 2-byte slot ABI, docs 13 §2.2.1), state at
//! `0xB000`. Code lives *outside* the window ([`crate::exec::CODE_OFFSET`]), so
//! the window is pure data and its final image is comparable to the reference
//! interpreter's byte for byte.
//!
//! **Register plan** (the naive fixed roles): `s0` window base (set by the
//! runner, preserved), `s1` slot base (`s0 + 0x9000`, re-derived in every
//! prologue — same value everywhere, so calls never save it), `t0` the value
//! register, `t1` the popped left operand, `t2` address scratch, `a0–a2` the
//! ≤3-arg family convention. Intermediates spill to the real stack (`sp`).
//!
//! **Width discipline:** 16-bit values (u8/u16/i16) live zero-extended — the
//! invariant every node maintains (wrapping ops re-mask; `i16` compare/divide/
//! `>>` sign-extend at the operation and re-mask after, the SWord rule). Wide
//! values (u32/i32/f32 bits) are native registers; signed-32 lowers natively
//! (`slt`/`div`/`sra`) — the ops rustz80 gates are RV32's home turf.
//!
//! Alignment: slots and struct fields sit at even window addresses (halfword
//! ops); `LoadAt`/`StoreAt` reach byte-computed addresses (packed struct-array
//! elements) and use byte pairs — Hazard3 has no misaligned support and the
//! executor faults on it, so the battery proves alignment safety.

use crate::ins::{Alu, AluI, Bcc, Ins, LoadW, Reg, StoreW};
use cell80_core::ir::*;
use std::collections::HashMap;

/// Where const data lays in the window (the interpreter's convention).
pub const CONST_BASE: u16 = 0x8000;
/// The slot file's window base (the family scratch region).
pub const SCRATCH: u16 = 0x9000;

/// A compiled image: code (position-independent, entered via `symbols`), the
/// const blob to plant at [`CONST_BASE`], and each function's byte offset.
#[derive(Clone)]
pub struct Image {
    pub code: Vec<u8>,
    pub consts: Vec<u8>,
    pub symbols: HashMap<String, u32>,
}

/// The serialized image magic (the RV32 sibling of the Z80 body's `CZ80`).
pub const IMAGE_MAGIC: &[u8; 4] = b"CV32";

impl Image {
    /// Serialize: magic, version, code, consts, and the symbol table in sorted
    /// order (deterministic bytes — the artifact hash covers them).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(IMAGE_MAGIC);
        b.push(1); // image format version
        b.extend_from_slice(&(self.code.len() as u32).to_le_bytes());
        b.extend_from_slice(&self.code);
        b.extend_from_slice(&(self.consts.len() as u32).to_le_bytes());
        b.extend_from_slice(&self.consts);
        let mut syms: Vec<_> = self.symbols.iter().collect();
        syms.sort();
        b.extend_from_slice(&(syms.len() as u32).to_le_bytes());
        for (name, off) in syms {
            b.extend_from_slice(&(name.len() as u16).to_le_bytes());
            b.extend_from_slice(name.as_bytes());
            b.extend_from_slice(&off.to_le_bytes());
        }
        b
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut i = 0usize;
        let take = |i: &mut usize, n: usize| -> Result<&[u8], String> {
            let s = bytes
                .get(*i..*i + n)
                .ok_or_else(|| "rv32 image truncated".to_string())?;
            *i += n;
            Ok(s)
        };
        if take(&mut i, 4)? != IMAGE_MAGIC {
            return Err("not an rv32 cell image".into());
        }
        if take(&mut i, 1)?[0] != 1 {
            return Err("unsupported rv32 image version".into());
        }
        let u32v = |i: &mut usize| -> Result<u32, String> {
            Ok(u32::from_le_bytes(take(i, 4)?.try_into().unwrap()))
        };
        let code_len = u32v(&mut i)? as usize;
        let code = take(&mut i, code_len)?.to_vec();
        let consts_len = u32v(&mut i)? as usize;
        let consts = take(&mut i, consts_len)?.to_vec();
        let n_syms = u32v(&mut i)? as usize;
        let mut symbols = HashMap::with_capacity(n_syms);
        for _ in 0..n_syms {
            let name_len = u16::from_le_bytes(take(&mut i, 2)?.try_into().unwrap()) as usize;
            let name = String::from_utf8(take(&mut i, name_len)?.to_vec())
                .map_err(|_| "bad symbol name".to_string())?;
            let off = u32v(&mut i)?;
            symbols.insert(name, off);
        }
        Ok(Image {
            code,
            consts,
            symbols,
        })
    }
}

/// Compile lowered functions + const data to an RV32 image.
pub fn compile(funcs: &[(String, Func)], consts: &[(String, Vec<u8>)]) -> Result<Image, String> {
    let mut g = Gen {
        ins: Vec::new(),
        n_labels: 0,
        base: 0,
        loop_stack: Vec::new(),
        fn_end: None,
        ret_wide: false,
        wide_sigs: funcs
            .iter()
            .filter(|(_, f)| f.wide_param || f.wide_ret)
            .map(|(n, f)| (n.clone(), (f.wide_param, f.wide_second, f.wide_ret)))
            .collect(),
        consts: HashMap::new(),
    };
    let mut blob = Vec::new();
    let mut at = CONST_BASE;
    for (name, bytes) in consts {
        g.consts.insert(name.clone(), at);
        blob.extend_from_slice(bytes);
        at = at
            .checked_add(bytes.len() as u16)
            .ok_or("rustrv32: const data overruns the window")?;
    }
    let mut base = 0u16;
    for (name, f) in funcs {
        g.ins.push(Ins::Def(name.clone()));
        g.base = base;
        g.emit_func(f)?;
        base += f.n_locals as u16;
    }
    let (code, symbols) = crate::ins::encode_with_symbols(&g.ins)?;
    Ok(Image {
        code,
        consts: blob,
        symbols,
    })
}

struct Gen {
    ins: Vec<Ins>,
    n_labels: usize,
    /// The current function's slot base (frames laid in `funcs` order — the same
    /// assignment as codegen-zero and the interpreter, so windows compare).
    base: u16,
    /// `(continue target, break target)`, innermost last.
    loop_stack: Vec<(usize, usize)>,
    fn_end: Option<usize>,
    ret_wide: bool,
    wide_sigs: HashMap<String, (bool, bool, bool)>,
    consts: HashMap<String, u16>,
}

use Reg::{Ra, A0, A1, A2, S0, S1, T0, T1, T2, X0};

impl Gen {
    // ── tiny emitters ───────────────────────────────────────────────────────

    fn label(&mut self) -> usize {
        self.n_labels += 1;
        self.n_labels - 1
    }
    fn place(&mut self, l: usize) {
        self.ins.push(Ins::At(l));
    }
    fn jump(&mut self, l: usize) {
        self.ins.push(Ins::Jal(X0, l));
    }
    fn op(&mut self, a: Alu, rd: Reg, rs1: Reg, rs2: Reg) {
        self.ins.push(Ins::Op(a, rd, rs1, rs2));
    }
    fn opi(&mut self, a: AluI, rd: Reg, rs1: Reg, imm: i32) {
        self.ins.push(Ins::OpI(a, rd, rs1, imm));
    }
    fn mv(&mut self, rd: Reg, rs: Reg) {
        self.opi(AluI::Addi, rd, rs, 0);
    }
    /// `rd = v` (the standard lui+addi expansion).
    fn li(&mut self, rd: Reg, v: u32) {
        let lo = (v << 20) as i32 >> 20; // sign-extended low 12
        let hi = v.wrapping_sub(lo as u32);
        if hi == 0 {
            self.opi(AluI::Addi, rd, X0, lo);
        } else {
            self.ins.push(Ins::Lui(rd, hi >> 12));
            if lo != 0 {
                self.opi(AluI::Addi, rd, rd, lo);
            }
        }
    }
    /// Re-establish the 16-bit zero-extension invariant on `t0`.
    fn mask16(&mut self) {
        self.opi(AluI::Slli, T0, T0, 16);
        self.opi(AluI::Srli, T0, T0, 16);
    }
    fn mask_w(&mut self, w: Width) {
        if w == Width::Byte {
            self.opi(AluI::Andi, T0, T0, 0xFF);
        } else {
            self.mask16();
        }
    }
    /// Sign-extend a 16-bit-invariant value in `r` (the SWord operation entry).
    fn sext16(&mut self, r: Reg) {
        self.opi(AluI::Slli, r, r, 16);
        self.opi(AluI::Srai, r, r, 16);
    }
    fn push_t0(&mut self) {
        self.opi(AluI::Addi, Reg::Sp, Reg::Sp, -4);
        self.ins.push(Ins::Store(StoreW::Sw, Reg::Sp, T0, 0));
    }
    fn pop(&mut self, r: Reg) {
        self.ins.push(Ins::Load(LoadW::Lw, r, Reg::Sp, 0));
        self.opi(AluI::Addi, Reg::Sp, Reg::Sp, 4);
    }
    /// Evaluate `l` then `r` (source order, A2a): `t1 = l`, `t0 = r`.
    fn pair16(&mut self, l: &Expr, r: &Expr) -> Result<(), String> {
        self.e16(l)?;
        self.push_t0();
        self.e16(r)?;
        self.pop(T1);
        Ok(())
    }
    fn pair32(&mut self, l: &Expr, r: &Expr) -> Result<(), String> {
        self.e32(l)?;
        self.push_t0();
        self.e32(r)?;
        self.pop(T1);
        Ok(())
    }

    /// The window address of local `slot` (a compile-time constant).
    fn slot_off(&self, slot: usize) -> i32 {
        (self.base as i32 + slot as i32) * 2
    }
    /// `t2 = &slot` when the s1-relative offset outgrows the immediate.
    fn slot_reg(&mut self, slot: usize) -> (Reg, i32) {
        let off = self.slot_off(slot);
        if (0..2048).contains(&off) {
            (S1, off)
        } else {
            self.li(T2, off as u32);
            self.op(Alu::Add, T2, T2, S1);
            (T2, 0)
        }
    }
    fn load_slot(&mut self, rd: Reg, slot: usize) {
        let (b, off) = self.slot_reg(slot);
        self.ins.push(Ins::Load(LoadW::Lhu, rd, b, off));
    }
    fn store_slot(&mut self, rs: Reg, slot: usize) {
        let (b, off) = self.slot_reg(slot);
        self.ins.push(Ins::Store(StoreW::Sh, b, rs, off));
    }
    fn load_slot32(&mut self, slot: usize) {
        let (b, off) = self.slot_reg(slot);
        self.ins.push(Ins::Load(LoadW::Lhu, T0, b, off));
        self.ins.push(Ins::Load(LoadW::Lhu, T1, b, off + 2));
        self.opi(AluI::Slli, T1, T1, 16);
        self.op(Alu::Or, T0, T0, T1);
    }
    fn store_slot32(&mut self, slot: usize) {
        let (b, off) = self.slot_reg(slot);
        self.ins.push(Ins::Store(StoreW::Sh, b, T0, off));
        self.opi(AluI::Srli, T1, T0, 16);
        self.ins.push(Ins::Store(StoreW::Sh, b, T1, off + 2));
    }
    /// `t2 = s0 + t0` — a runtime window address.
    fn window_t2(&mut self) {
        self.op(Alu::Add, T2, S0, T0);
    }
    /// Add a constant byte offset to `t0` (field offsets are small).
    fn add_off(&mut self, off: usize) {
        if off != 0 {
            self.opi(AluI::Addi, T0, T0, off as i32);
        }
    }

    // ── functions ───────────────────────────────────────────────────────────

    fn emit_func(&mut self, f: &Func) -> Result<(), String> {
        self.ret_wide = f.wide_ret;
        // Prologue: save ra, derive s1 (same value in every frame), bind args.
        self.opi(AluI::Addi, Reg::Sp, Reg::Sp, -4);
        self.ins.push(Ins::Store(StoreW::Sw, Reg::Sp, Ra, 0));
        self.ins.push(Ins::Lui(S1, (SCRATCH as u32) >> 12));
        self.op(Alu::Add, S1, S1, S0);
        if f.wide_second {
            // a0/a1 wide into slot pairs 0-1 / 2-3; an optional third u16 in a2.
            self.mv(T0, A0);
            self.store_wide_arg(0);
            self.mv(T0, A1);
            self.store_wide_arg(2);
            if f.params == 5 {
                self.store_slot(A2, 4);
            }
        } else if f.wide_param {
            self.mv(T0, A0);
            self.store_wide_arg(0);
            if f.params == 3 {
                self.store_slot(A1, 2);
            }
        } else {
            const ARGS: [Reg; 3] = [A0, A1, A2];
            for (i, &reg) in ARGS.iter().enumerate().take(f.params) {
                self.store_slot(reg, i);
            }
        }
        let end = self.label();
        self.fn_end = Some(end);
        for s in &f.body {
            self.stmt(s)?;
        }
        self.ret_values(&f.ret)?;
        self.place(end);
        self.fn_end = None;
        self.pop(Ra);
        self.ins.push(Ins::Jalr(X0, Ra, 0));
        Ok(())
    }

    fn store_wide_arg(&mut self, slot: usize) {
        let (b, off) = self.slot_reg(slot);
        self.ins.push(Ins::Store(StoreW::Sh, b, T0, off));
        self.opi(AluI::Srli, T1, T0, 16);
        self.ins.push(Ins::Store(StoreW::Sh, b, T1, off + 2));
    }

    /// Evaluate the return values into `a0..a2` (first lands in `a0`).
    fn ret_values(&mut self, rets: &[Expr]) -> Result<(), String> {
        match rets.len() {
            0 => {}
            1 if self.ret_wide => {
                self.e32(&rets[0])?;
                self.mv(A0, T0);
            }
            1 => {
                self.e16(&rets[0])?;
                self.mv(A0, T0);
            }
            n => {
                for e in rets {
                    self.e16(e)?;
                    self.push_t0();
                }
                const OUT: [Reg; 3] = [A0, A1, A2];
                for i in (0..n).rev() {
                    self.pop(OUT[i]);
                }
            }
        }
        Ok(())
    }

    // ── calls ───────────────────────────────────────────────────────────────

    /// Emit a call; `Ok(true)` = a `__bits_*` builtin was inlined instead (its
    /// result is already in `t0`, not `a0`).
    fn call(&mut self, name: &str, args: &[Expr]) -> Result<bool, String> {
        if self.bits_builtin(name, args)? {
            return Ok(true);
        }
        let (wp, ws, _) = self
            .wide_sigs
            .get(name)
            .copied()
            .unwrap_or((false, false, false));
        // Logical argument widths by position (the family call convention).
        let wide_at = |i: usize| (i == 0 && wp) || (i == 1 && ws);
        for (i, a) in args.iter().enumerate() {
            if wide_at(i) {
                self.e32(a)?;
            } else {
                self.e16(a)?;
            }
            self.push_t0();
        }
        const ARGS: [Reg; 3] = [A0, A1, A2];
        for i in (0..args.len()).rev() {
            self.pop(ARGS[i]);
        }
        self.ins.push(Ins::Call(name.to_string()));
        Ok(false)
    }

    /// The `__bits_*` kernels, inlined (rustz80 appends machine-code blobs; here
    /// a short loop at the call site is the naive equivalent).
    fn bits_builtin(&mut self, name: &str, args: &[Expr]) -> Result<bool, String> {
        let (count_ones, leading) = match name {
            "__bits_count_ones" => (true, false),
            "__bits_leading_zeros" => (false, true),
            "__bits_trailing_zeros" => (false, false),
            _ => return Ok(false),
        };
        self.e16(&args[0])?;
        if count_ones {
            // t1 = 0; while t0 != 0 { t1 += t0 & 1; t0 >>= 1 }
            let (top, done) = (self.label(), self.label());
            self.li(T1, 0);
            self.place(top);
            self.ins.push(Ins::Branch(Bcc::Eq, T0, X0, done));
            self.opi(AluI::Andi, T2, T0, 1);
            self.op(Alu::Add, T1, T1, T2);
            self.opi(AluI::Srli, T0, T0, 1);
            self.jump(top);
            self.place(done);
            self.mv(T0, T1);
            return Ok(true);
        }
        // lz/tz: 16 for zero, else count from the top/bottom bit.
        let (zero, top, done) = (self.label(), self.label(), self.label());
        self.ins.push(Ins::Branch(Bcc::Eq, T0, X0, zero));
        self.li(T1, 0);
        self.place(top);
        if leading {
            self.li(T2, 0x8000);
            self.op(Alu::And, T2, T0, T2);
            self.ins.push(Ins::Branch(Bcc::Ne, T2, X0, done));
            self.opi(AluI::Addi, T1, T1, 1);
            self.opi(AluI::Slli, T0, T0, 1);
            self.mask16();
        } else {
            self.opi(AluI::Andi, T2, T0, 1);
            self.ins.push(Ins::Branch(Bcc::Ne, T2, X0, done));
            self.opi(AluI::Addi, T1, T1, 1);
            self.opi(AluI::Srli, T0, T0, 1);
        }
        self.jump(top);
        self.place(zero);
        self.li(T1, 16); // the zero case reports the full width
        self.place(done);
        self.mv(T0, T1);
        Ok(true)
    }

    // ── statements ──────────────────────────────────────────────────────────

    fn stmt(&mut self, s: &Stmt) -> Result<(), String> {
        match s {
            Stmt::Assign(slot, e) => {
                self.e16(e)?;
                self.store_slot(T0, *slot);
            }
            Stmt::Assign32(slot, e) => {
                self.e32(e)?;
                self.store_slot32(*slot);
            }
            Stmt::StoreIndex(base, index, value, w) => {
                self.e16(value)?;
                self.push_t0();
                self.e16(index)?;
                self.opi(AluI::Slli, T0, T0, 1);
                let (b, off) = self.slot_reg(*base);
                if off != 0 {
                    self.opi(AluI::Addi, T0, T0, off);
                }
                self.op(Alu::Add, T2, b, T0);
                self.pop(T1);
                // Low byte always; the high byte only for Word — the codegen-zero
                // (and interpreter) contract, mirrored.
                if *w == Width::Word {
                    self.ins.push(Ins::Store(StoreW::Sh, T2, T1, 0));
                } else {
                    self.ins.push(Ins::Store(StoreW::Sb, T2, T1, 0));
                }
            }
            Stmt::Poke(addr, value) => {
                self.e16(value)?;
                self.push_t0();
                self.e16(addr)?;
                self.window_t2();
                self.pop(T1);
                self.ins.push(Ins::Store(StoreW::Sb, T2, T1, 0));
            }
            Stmt::Store(ptr, off, value) => {
                self.e16(value)?;
                self.push_t0();
                self.e16(ptr)?;
                self.add_off(*off);
                self.window_t2();
                self.pop(T1);
                self.ins.push(Ins::Store(StoreW::Sh, T2, T1, 0));
            }
            Stmt::PtrStoreIndex {
                ptr,
                off,
                index,
                value,
            } => {
                self.e16(value)?;
                self.push_t0();
                self.e16(ptr)?;
                self.add_off(*off);
                self.push_t0();
                self.e16(index)?;
                self.opi(AluI::Slli, T0, T0, 1);
                self.pop(T1);
                self.op(Alu::Add, T0, T0, T1);
                self.window_t2();
                self.pop(T1);
                self.ins.push(Ins::Store(StoreW::Sh, T2, T1, 0));
            }
            Stmt::StoreAt(addr, value, w) => {
                self.e16(value)?;
                self.push_t0();
                self.e16(addr)?;
                self.window_t2();
                self.pop(T1);
                // Byte-computed addresses (packed struct-array elements) can be
                // odd — byte pairs keep Hazard3 alignment safety.
                self.ins.push(Ins::Store(StoreW::Sb, T2, T1, 0));
                if *w == Width::Word {
                    self.opi(AluI::Srli, T1, T1, 8);
                    self.ins.push(Ins::Store(StoreW::Sb, T2, T1, 1));
                }
            }
            Stmt::Store32(ptr, off, value) => {
                self.e32(value)?;
                self.push_t0();
                self.e16(ptr)?;
                self.add_off(*off);
                self.window_t2();
                self.pop(T1);
                self.ins.push(Ins::Store(StoreW::Sh, T2, T1, 0));
                self.opi(AluI::Srli, T1, T1, 16);
                self.ins.push(Ins::Store(StoreW::Sh, T2, T1, 2));
            }
            Stmt::Fill { base, count, value } => {
                if *count == 0 {
                    return Ok(());
                }
                self.e16(value)?;
                let (b, off) = self.slot_reg(*base);
                if b == S1 {
                    self.opi(AluI::Addi, T2, S1, off);
                }
                self.li(T1, *count as u32);
                let top = self.label();
                self.place(top);
                self.ins.push(Ins::Store(StoreW::Sh, T2, T0, 0));
                self.opi(AluI::Addi, T2, T2, 2);
                self.opi(AluI::Addi, T1, T1, -1);
                self.ins.push(Ins::Branch(Bcc::Ne, T1, X0, top));
            }
            Stmt::Eval(e) => {
                self.e16(e)?;
            }
            Stmt::AssignTuple(slots, call) => {
                let Expr::Call(name, args) = call else {
                    return Err("rustrv32: AssignTuple of a non-call".into());
                };
                self.call(name, args)?;
                const OUT: [Reg; 3] = [A0, A1, A2];
                for (i, slot) in slots.iter().enumerate() {
                    self.store_slot(OUT[i], *slot);
                }
            }
            Stmt::If(cond, then, els) => {
                let (else_l, end) = (self.label(), self.label());
                self.cond_skip(cond, else_l)?;
                for s in then {
                    self.stmt(s)?;
                }
                self.jump(end);
                self.place(else_l);
                for s in els {
                    self.stmt(s)?;
                }
                self.place(end);
            }
            Stmt::While(cond, body) => {
                let (top, end) = (self.label(), self.label());
                self.place(top);
                self.cond_skip(cond, end)?;
                self.loop_stack.push((top, end));
                for s in body {
                    self.stmt(s)?;
                }
                self.loop_stack.pop();
                self.jump(top);
                self.place(end);
            }
            Stmt::Loop(body) => {
                let (top, end) = (self.label(), self.label());
                self.place(top);
                self.loop_stack.push((top, end));
                for s in body {
                    self.stmt(s)?;
                }
                self.loop_stack.pop();
                self.jump(top);
                self.place(end);
            }
            Stmt::ForRange {
                var,
                end,
                inclusive,
                width,
                body,
            } => {
                let (top, cont, brk) = (self.label(), self.label(), self.label());
                self.place(top);
                // t1 = bound, t0 = var (bounds are effect-free by lowering).
                self.e16(end)?;
                self.push_t0();
                self.load_slot(T0, *var);
                self.pop(T1);
                let signed = *width == Width::SWord;
                if signed {
                    self.sext16(T0);
                    self.sext16(T1);
                }
                // keep = var < bound (or <=): false-jump to brk.
                let cc = match (*inclusive, signed) {
                    (false, false) => Bcc::Geu, // !(var < bound)
                    (false, true) => Bcc::Ge,
                    (true, false) => Bcc::Ltu, // !(var <= bound) = bound < var
                    (true, true) => Bcc::Lt,
                };
                if *inclusive {
                    self.ins.push(Ins::Branch(cc, T1, T0, brk));
                } else {
                    self.ins.push(Ins::Branch(cc, T0, T1, brk));
                }
                self.loop_stack.push((cont, brk));
                for s in body {
                    self.stmt(s)?;
                }
                self.loop_stack.pop();
                self.place(cont);
                self.load_slot(T0, *var);
                self.opi(AluI::Addi, T0, T0, 1);
                self.mask_w(*width);
                self.store_slot(T0, *var);
                self.jump(top);
                self.place(brk);
            }
            Stmt::Break => {
                let (_, brk) = *self
                    .loop_stack
                    .last()
                    .ok_or("rustrv32: `break` outside a loop")?;
                self.jump(brk);
            }
            Stmt::Continue => {
                let (cont, _) = *self
                    .loop_stack
                    .last()
                    .ok_or("rustrv32: `continue` outside a loop")?;
                self.jump(cont);
            }
            Stmt::Return(val) => {
                if let Some(e) = val {
                    if self.ret_wide {
                        self.e32(e)?;
                    } else {
                        self.e16(e)?;
                    }
                    self.mv(A0, T0);
                }
                let end = self.fn_end.ok_or("rustrv32: `return` outside a function")?;
                self.jump(end);
            }
        }
        Ok(())
    }

    /// Branch to `target` when the condition is FALSE (the skip form).
    fn cond_skip(&mut self, cond: &Cond, target: usize) -> Result<(), String> {
        self.pair16(&cond.lhs, &cond.rhs)?; // t1 = lhs, t0 = rhs
        let signed = cond.signed && !matches!(cond.cmp, Cmp::Eq | Cmp::Ne);
        if signed {
            self.sext16(T0);
            self.sext16(T1);
        }
        // Inverted-condition branches; `u` variants for unsigned ordering.
        let ins = match (cond.cmp, signed) {
            (Cmp::Eq, _) => Ins::Branch(Bcc::Ne, T1, T0, target),
            (Cmp::Ne, _) => Ins::Branch(Bcc::Eq, T1, T0, target),
            (Cmp::Lt, true) => Ins::Branch(Bcc::Ge, T1, T0, target),
            (Cmp::Lt, false) => Ins::Branch(Bcc::Geu, T1, T0, target),
            (Cmp::Ge, true) => Ins::Branch(Bcc::Lt, T1, T0, target),
            (Cmp::Ge, false) => Ins::Branch(Bcc::Ltu, T1, T0, target),
            (Cmp::Gt, true) => Ins::Branch(Bcc::Ge, T0, T1, target),
            (Cmp::Gt, false) => Ins::Branch(Bcc::Geu, T0, T1, target),
            (Cmp::Le, true) => Ins::Branch(Bcc::Lt, T0, T1, target),
            (Cmp::Le, false) => Ins::Branch(Bcc::Ltu, T0, T1, target),
        };
        self.ins.push(ins);
        Ok(())
    }

    // ── 16-bit expressions (zero-extension invariant) ───────────────────────

    fn e16(&mut self, e: &Expr) -> Result<(), String> {
        match e {
            Expr::Lit(n) => self.li(T0, *n as u32),
            Expr::Var(slot) => self.load_slot(T0, *slot),
            Expr::Bin(op, l, r, w) => self.bin16(*op, l, r, *w)?,
            Expr::Index(base, index, w) => {
                self.e16(index)?;
                self.opi(AluI::Slli, T0, T0, 1);
                let (b, off) = self.slot_reg(*base);
                if off != 0 {
                    self.opi(AluI::Addi, T0, T0, off);
                }
                self.op(Alu::Add, T2, b, T0);
                let lw = if *w == Width::Byte {
                    LoadW::Lbu
                } else {
                    LoadW::Lhu
                };
                self.ins.push(Ins::Load(lw, T0, T2, 0));
            }
            Expr::Call(name, args) => {
                let wide_ret = self
                    .wide_sigs
                    .get(name.as_str())
                    .is_some_and(|(_, _, wr)| *wr);
                if !self.call(name, args)? {
                    self.mv(T0, A0);
                    if wide_ret {
                        // The 16-bit consumer takes the low word (the HL reading).
                        self.mask16();
                    }
                }
            }
            Expr::Trunc(e) => {
                self.e16(e)?;
                self.opi(AluI::Andi, T0, T0, 0xFF);
            }
            Expr::Peek(addr) => {
                self.e16(addr)?;
                self.window_t2();
                self.ins.push(Ins::Load(LoadW::Lbu, T0, T2, 0));
            }
            Expr::InPort(_) => {
                return Err(
                    "rustrv32: `inport` has no meaning off the Z80 (no port space) — \
                     sensor inputs arrive as typed state (WS-C)"
                        .into(),
                )
            }
            Expr::AddrOf(slot) => {
                let addr = SCRATCH as u32 + self.slot_off(*slot) as u32;
                self.li(T0, addr);
            }
            Expr::ConstAddr(name) => {
                let addr = *self
                    .consts
                    .get(name)
                    .ok_or_else(|| format!("rustrv32: unknown const `{name}`"))?;
                self.li(T0, addr as u32);
            }
            Expr::Deref(ptr, off) => {
                self.e16(ptr)?;
                self.add_off(*off);
                self.window_t2();
                self.ins.push(Ins::Load(LoadW::Lhu, T0, T2, 0));
            }
            Expr::PtrIndex { ptr, off, index } => {
                self.e16(ptr)?;
                self.add_off(*off);
                self.push_t0();
                self.e16(index)?;
                self.opi(AluI::Slli, T0, T0, 1);
                self.pop(T1);
                self.op(Alu::Add, T0, T0, T1);
                self.window_t2();
                self.ins.push(Ins::Load(LoadW::Lhu, T0, T2, 0));
            }
            Expr::MulConst(e, k) => {
                self.e16(e)?;
                self.li(T1, *k as u32);
                self.op(Alu::Mul, T0, T0, T1);
                self.mask16();
            }
            Expr::LoadAt(addr, w) => {
                self.e16(addr)?;
                self.window_t2();
                if *w == Width::Byte {
                    self.ins.push(Ins::Load(LoadW::Lbu, T0, T2, 0));
                } else {
                    // Byte-computed addresses can be odd: byte pairs (see StoreAt).
                    self.ins.push(Ins::Load(LoadW::Lbu, T0, T2, 0));
                    self.ins.push(Ins::Load(LoadW::Lbu, T1, T2, 1));
                    self.opi(AluI::Slli, T1, T1, 8);
                    self.op(Alu::Or, T0, T0, T1);
                }
            }
            Expr::Cmp {
                cmp,
                lhs,
                rhs,
                signed,
            } => {
                self.pair16(lhs, rhs)?; // t1 = lhs, t0 = rhs
                let signed = *signed && !matches!(cmp, Cmp::Eq | Cmp::Ne);
                if signed {
                    self.sext16(T0);
                    self.sext16(T1);
                }
                self.cmp_value(*cmp, signed);
            }
            Expr::Logic { and, lhs, rhs } => {
                self.e16(lhs)?;
                let end = self.label();
                let cc = if *and { Bcc::Eq } else { Bcc::Ne };
                self.ins.push(Ins::Branch(cc, T0, X0, end));
                self.e16(rhs)?;
                self.place(end);
            }
            Expr::Cmp32 {
                cmp,
                lhs,
                rhs,
                signed,
            } => {
                self.pair32(lhs, rhs)?;
                self.cmp_value(*cmp, *signed);
            }
            Expr::ShiftVar { left, e, amount, w } => {
                self.e16(e)?;
                self.push_t0();
                self.e16(amount)?;
                self.opi(AluI::Andi, T2, T0, 0xFF); // count = low byte
                self.pop(T0);
                if !*left && *w == Width::SWord {
                    self.sext16(T0);
                }
                let (top, done) = (self.label(), self.label());
                self.place(top);
                self.ins.push(Ins::Branch(Bcc::Eq, T2, X0, done));
                if *left {
                    self.opi(AluI::Slli, T0, T0, 1);
                } else if *w == Width::SWord {
                    self.opi(AluI::Srai, T0, T0, 1);
                } else {
                    self.opi(AluI::Srli, T0, T0, 1);
                }
                self.opi(AluI::Addi, T2, T2, -1);
                self.jump(top);
                self.place(done);
                self.mask_w(*w);
            }
            Expr::Trunc32(e) => {
                self.e32(e)?;
                self.mask16();
            }
            Expr::Halt(code) => {
                self.e16(code)?;
                self.mv(A0, T0);
                self.ins.push(Ins::Ecall);
            }
            Expr::Lit32(_)
            | Expr::Var32(_)
            | Expr::Deref32(..)
            | Expr::Bin32(..)
            | Expr::Shift32 { .. }
            | Expr::Widen(..)
            | Expr::SignExtend(..) => {
                return Err("rustrv32: u32 node in a 16-bit context".into());
            }
        }
        Ok(())
    }

    /// Materialise `t0 = (t1 <cmp> t0) as 0/1` (operands already sign-extended
    /// when the ordering is signed).
    fn cmp_value(&mut self, cmp: Cmp, signed: bool) {
        let slt = if signed { Alu::Slt } else { Alu::Sltu };
        match cmp {
            Cmp::Lt => self.op(slt, T0, T1, T0),
            Cmp::Gt => self.op(slt, T0, T0, T1),
            Cmp::Ge => {
                self.op(slt, T0, T1, T0);
                self.opi(AluI::Xori, T0, T0, 1);
            }
            Cmp::Le => {
                self.op(slt, T0, T0, T1);
                self.opi(AluI::Xori, T0, T0, 1);
            }
            Cmp::Eq => {
                self.op(Alu::Sub, T0, T1, T0);
                self.opi(AluI::Sltiu, T0, T0, 1);
            }
            Cmp::Ne => {
                self.op(Alu::Sub, T0, T1, T0);
                self.op(Alu::Sltu, T0, X0, T0);
            }
        }
    }

    fn bin16(&mut self, op: BinOp, l: &Expr, r: &Expr, w: Width) -> Result<(), String> {
        // Constant shifts take only the left operand (the amount is a literal).
        if matches!(op, BinOp::Shl | BinOp::Shr) {
            let Expr::Lit(k) = r else {
                return Err("rustrv32: shift amount must be a constant".into());
            };
            let k = *k as u32;
            self.e16(l)?;
            match op {
                BinOp::Shl => {
                    if k >= 32 {
                        self.li(T0, 0);
                    } else {
                        self.opi(AluI::Slli, T0, T0, k as i32);
                        self.mask_w(w);
                    }
                }
                _ if w == Width::SWord => {
                    self.sext16(T0);
                    self.opi(AluI::Srai, T0, T0, k.min(31) as i32);
                    self.mask16();
                }
                _ => {
                    if k >= 32 {
                        self.li(T0, 0);
                    } else {
                        self.opi(AluI::Srli, T0, T0, k as i32);
                    }
                }
            }
            return Ok(());
        }
        self.pair16(l, r)?; // t1 = l, t0 = r
        let signed = w == Width::SWord;
        match op {
            BinOp::Add => {
                self.op(Alu::Add, T0, T1, T0);
                self.mask_w(w);
            }
            BinOp::Sub => {
                self.op(Alu::Sub, T0, T1, T0);
                self.mask_w(w);
            }
            BinOp::Mul => {
                self.op(Alu::Mul, T0, T1, T0);
                self.mask_w(w);
            }
            BinOp::Div | BinOp::Rem => {
                if signed {
                    self.sext16(T0);
                    self.sext16(T1);
                }
                let alu = match (op, signed) {
                    (BinOp::Div, true) => Alu::Div,
                    (BinOp::Div, false) => Alu::Divu,
                    (BinOp::Rem, true) => Alu::Rem,
                    (BinOp::Rem, false) => Alu::Remu,
                    _ => unreachable!(),
                };
                self.op(alu, T0, T1, T0);
                self.mask_w(w);
            }
            // Bitwise results mask too: `Bin(_, .., Byte)` is legal IR with
            // *oversized* operands (the widened-product saturating clamp) — the
            // width is the contract, not an operand invariant.
            BinOp::Or => {
                self.op(Alu::Or, T0, T1, T0);
                self.mask_w(w);
            }
            BinOp::And => {
                self.op(Alu::And, T0, T1, T0);
                self.mask_w(w);
            }
            BinOp::Xor => {
                self.op(Alu::Xor, T0, T1, T0);
                self.mask_w(w);
            }
            BinOp::Shl | BinOp::Shr => unreachable!("handled above"),
        }
        Ok(())
    }

    // ── 32-bit expressions (native) ─────────────────────────────────────────

    fn e32(&mut self, e: &Expr) -> Result<(), String> {
        match e {
            Expr::Lit32(n) => self.li(T0, *n),
            Expr::Var32(slot) => self.load_slot32(*slot),
            Expr::Deref32(ptr, off) => {
                self.e16(ptr)?;
                self.add_off(*off);
                self.window_t2();
                self.ins.push(Ins::Load(LoadW::Lhu, T0, T2, 0));
                self.ins.push(Ins::Load(LoadW::Lhu, T1, T2, 2));
                self.opi(AluI::Slli, T1, T1, 16);
                self.op(Alu::Or, T0, T0, T1);
            }
            Expr::Trunc32(e) => self.e32(e)?, // identity in wide position
            Expr::Call(name, args) => {
                if !self.call(name, args)? {
                    self.mv(T0, A0);
                }
            }
            Expr::Widen(inner) => self.e16(inner)?, // already zero-extended
            Expr::SignExtend(inner) => {
                self.e16(inner)?;
                self.sext16(T0);
            }
            Expr::Bin32(op, l, r, signed) => {
                self.pair32(l, r)?; // t1 = l, t0 = r
                let alu = match (op, *signed) {
                    (BinOp::Add, _) => Alu::Add,
                    (BinOp::Sub, _) => Alu::Sub,
                    (BinOp::Mul, _) => Alu::Mul,
                    (BinOp::Div, true) => Alu::Div,
                    (BinOp::Div, false) => Alu::Divu,
                    (BinOp::Rem, true) => Alu::Rem,
                    (BinOp::Rem, false) => Alu::Remu,
                    (BinOp::Or, _) => Alu::Or,
                    (BinOp::And, _) => Alu::And,
                    (BinOp::Xor, _) => Alu::Xor,
                    (BinOp::Shl | BinOp::Shr, _) => {
                        return Err("rustrv32: u32 shifts lower to Shift32".into())
                    }
                };
                self.op(alu, T0, T1, T0);
            }
            Expr::Shift32 { left, e, k, signed } => {
                self.e32(e)?;
                if *k >= 32 {
                    if *signed && !*left {
                        self.opi(AluI::Srai, T0, T0, 31);
                    } else {
                        self.li(T0, 0);
                    }
                } else if *left {
                    self.opi(AluI::Slli, T0, T0, *k as i32);
                } else if *signed {
                    self.opi(AluI::Srai, T0, T0, *k as i32);
                } else {
                    self.opi(AluI::Srli, T0, T0, *k as i32);
                }
            }
            _ => return Err("rustrv32: not a u32 expression".into()),
        }
        Ok(())
    }
}
