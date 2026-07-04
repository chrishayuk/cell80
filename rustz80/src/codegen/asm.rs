//! The `Asm` assembler — typed emission into the [`Ins`] stream, label/call/slot
//! symbols, the runtime-append seal, and the encode to bytes.
use super::ins::{encode, stream_len, FxBytes, Imm, Ins, R16};
use super::peephole::{self, PeepholeCounts};
use super::runtime::{
    emit_divmod32, emit_mul16w, emit_mul32, emit_sdivmod16, BIT_ROUTINES, DIVMOD16, MUL16,
};
use super::Target;
use std::collections::HashMap;

/// Default base for locals: slot `i` lives at `SCRATCH + i*2` (`u16` each). Each function
/// reuses the same region (Stage 1 has no recursion / overlapping live ranges yet). This is
/// only a *default* (used by `codegen_program`); the frame loop ([`super::codegen_loop`])
/// overrides `Asm::scratch` to sit just above the emitted code, so a large program's code
/// can't grow into the locals region.
pub(super) const SCRATCH: u16 = 0x9000;

pub(super) struct Asm {
    pub(super) org: u16,
    pub(super) target: Target,
    /// The emitted instruction stream — operands symbolic until [`Asm::finish`].
    pub(super) ins: Vec<Ins>,
    n_labels: usize,
    pub(super) needs_mul: bool,
    pub(super) needs_div: bool,
    pub(super) needs_mul32: bool,
    pub(super) needs_div32: bool,
    pub(super) needs_sdiv: bool,
    /// Slot offset for the function currently being emitted, so each function's
    /// locals occupy a disjoint scratch region (correct for non-recursive calls;
    /// real stack frames are a later stage).
    pub(super) base: u16,
    /// Base address of the locals scratch region. Defaults to [`SCRATCH`]; the frame loop
    /// raises it to just above the code so code and locals never overlap.
    pub(super) scratch: u16,
    /// Enclosing loops as `(continue target, break target)` labels — the innermost
    /// is last. `continue`/`break` jump to the top entry's targets.
    pub(super) loop_stack: Vec<(usize, usize)>,
    /// The current function's epilogue label — `return` jumps here (the value is
    /// already in `HL`).
    pub(super) func_end: Option<usize>,
    /// Whether the used runtime routines have been appended ([`Asm::seal`] ran).
    sealed: bool,
    /// Per-rule peephole fire counts from [`Asm::seal`] (measurement, not behaviour).
    pub(super) peep: PeepholeCounts,
}

impl Asm {
    pub(super) fn new(org: u16, target: Target) -> Self {
        Asm {
            org,
            target,
            ins: Vec::new(),
            n_labels: 0,
            needs_mul: false,
            needs_div: false,
            needs_mul32: false,
            needs_div32: false,
            needs_sdiv: false,
            base: 0,
            scratch: SCRATCH,
            loop_stack: Vec::new(),
            func_end: None,
            sealed: false,
            peep: PeepholeCounts::default(),
        }
    }

    // ── emission ────────────────────────────────────────────────────────────────

    /// One fixed instruction, as its exact bytes (boundaries survive: never two).
    pub(super) fn fx(&mut self, bytes: &[u8]) {
        self.ins.push(Ins::Fx(FxBytes::new(bytes)));
    }
    pub(super) fn push(&mut self, r: R16) {
        self.ins.push(Ins::Push(r));
    }
    pub(super) fn pop(&mut self, r: R16) {
        self.ins.push(Ins::Pop(r));
    }
    pub(super) fn ex_de_hl(&mut self) {
        self.ins.push(Ins::ExDeHl);
    }
    pub(super) fn add_hl(&mut self, r: R16) {
        self.ins.push(Ins::AddHl(r));
    }
    /// `LD HL/DE/BC, imm`.
    pub(super) fn ld_imm(&mut self, r: R16, m: Imm) {
        self.ins.push(Ins::LdImm(r, m));
    }
    /// `LD HL, (imm)`.
    pub(super) fn ld_hl_mem(&mut self, m: Imm) {
        self.ins.push(Ins::LdHlMem(m));
    }
    /// `LD (imm), HL`.
    pub(super) fn st_hl_mem(&mut self, m: Imm) {
        self.ins.push(Ins::StHlMem(m));
    }
    /// `LD DE/BC, (imm)` (the `ED` forms).
    pub(super) fn ld_wide_mem(&mut self, r: R16, m: Imm) {
        self.ins.push(Ins::LdWideMem(r, m));
    }
    /// `LD (imm), DE/BC` (the `ED` forms).
    pub(super) fn st_wide_mem(&mut self, r: R16, m: Imm) {
        self.ins.push(Ins::StWideMem(r, m));
    }
    /// A 2-byte data word in the stream (a runtime routine's static scratch).
    pub(super) fn data_word(&mut self, m: Imm) {
        self.ins.push(Ins::Word(m));
    }
    /// `LD rr, name` — load a symbol's address (a const-data item), resolved at
    /// encode against the symbol table.
    pub(super) fn ld_sym(&mut self, r: R16, name: &str) {
        self.ins.push(Ins::LdImmSym(r, name.to_string()));
    }
    /// Owned raw bytes laid into the image — the const-data section's payload.
    pub(super) fn data_bytes(&mut self, bytes: Vec<u8>) {
        self.ins.push(Ins::Bytes(bytes));
    }

    // ── labels, symbols, slots ──────────────────────────────────────────────────

    pub(super) fn label(&mut self) -> usize {
        self.n_labels += 1;
        self.n_labels - 1
    }
    pub(super) fn place(&mut self, l: usize) {
        self.ins.push(Ins::At(l));
    }
    pub(super) fn jump(&mut self, opcode: u8, l: usize) {
        self.ins.push(Ins::Jp(opcode, l));
    }
    /// Emit `CALL name` (resolved to the symbol address at finish).
    pub(super) fn call(&mut self, name: &str) {
        self.ins.push(Ins::Call(name.to_string()));
    }
    pub(super) fn define(&mut self, name: &str) {
        self.ins.push(Ins::Def(name.to_string()));
    }
    /// The (symbolic) address of local `slot` for the function currently being emitted.
    pub(super) fn slot(&self, slot: usize) -> Imm {
        Imm::Slot(self.base.wrapping_add(slot as u16), 0)
    }
    /// The high word of a `u32` slot pair: [`Asm::slot`]` + 2`.
    pub(super) fn slot_hi(&self, slot: usize) -> Imm {
        Imm::Slot(self.base.wrapping_add(slot as u16), 2)
    }

    // ── seal + encode ───────────────────────────────────────────────────────────

    /// Append the micro-runtime routines that were used (idempotent). The 32-bit
    /// routines come first: `__mul32` calls `__mul16`/`__mul16w`, so it turns those
    /// flags on.
    pub(super) fn seal(&mut self) {
        if self.sealed {
            return;
        }
        self.sealed = true;
        if self.needs_mul32 {
            emit_mul32(self);
            emit_mul16w(self);
            self.needs_mul = true;
        }
        if self.needs_div32 {
            emit_divmod32(self);
        }
        if self.needs_sdiv {
            // The signed wrapper forks per target inside: the Spectrum path calls the
            // software `__divmod16`, so that must be appended too.
            emit_sdivmod16(self);
            if self.target == Target::Spectrum48 {
                self.needs_div = true;
            }
        }
        if self.needs_mul {
            self.define("__mul16");
            self.ins.push(Ins::Blob(MUL16));
        }
        if self.needs_div {
            self.define("__divmod16");
            self.ins.push(Ins::Blob(DIVMOD16));
        }
        // The bit-method kernels (`__bits_*`, reserved names): appended when the
        // lowered stream calls them — same bytes on both targets (plain Z80 loops,
        // honest cycles, no traps).
        for (name, blob) in BIT_ROUTINES {
            let called = self
                .ins
                .iter()
                .any(|i| matches!(i, Ins::Call(n) if n == name));
            let defined = self
                .ins
                .iter()
                .any(|i| matches!(i, Ins::Def(n) if n == name));
            if called && !defined {
                self.define(name);
                self.ins.push(Ins::Blob(blob));
            }
        }
        // The Stage-2 peephole, over the whole stream (bodies + label-emitted
        // runtime) — before any measurement, so lengths are post-optimization.
        self.peep = peephole::optimize(&mut self.ins);
    }

    /// Encoded code length. Static per instruction (operands are 2-byte immediates),
    /// so it is **independent of the scratch base** — the frame loop measures with
    /// this, then places scratch just past the code. Call after [`Asm::seal`].
    pub(super) fn encoded_len(&self) -> u16 {
        debug_assert!(self.sealed, "measure after seal — the runtime counts");
        stream_len(&self.ins)
    }

    /// Seal, assign PCs, resolve operands (slots against `self.scratch`), and return
    /// the image. `Err` on an unknown call target (a `fn` referenced but never
    /// defined — e.g. an unconfigured prelude route) or an unplaced label (an internal
    /// codegen invariant). Returns a diagnostic rather than panicking so every compile
    /// entry surfaces it as a normal compile error.
    pub(super) fn finish(mut self) -> Result<(Vec<u8>, HashMap<String, u16>), String> {
        self.seal();
        encode(&self.ins, self.org, self.scratch, self.n_labels)
    }
}
