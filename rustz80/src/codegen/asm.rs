//! The `Asm` assembler — emit primitives, label/call fixups, the local scratch layout.
use super::runtime::{emit_divmod32, emit_mul16w, emit_mul32, DIVMOD16, MUL16};
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
    pub(super) code: Vec<u8>,
    pub(super) labels: Vec<Option<u16>>,
    pub(super) label_fixups: Vec<(usize, usize)>,
    pub(super) symbols: HashMap<String, u16>,
    pub(super) call_fixups: Vec<(usize, String)>,
    pub(super) needs_mul: bool,
    pub(super) needs_div: bool,
    pub(super) needs_mul32: bool,
    pub(super) needs_div32: bool,
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
}

impl Asm {
    pub(super) fn new(org: u16, target: Target) -> Self {
        Asm {
            org,
            target,
            code: Vec::new(),
            labels: Vec::new(),
            label_fixups: Vec::new(),
            symbols: HashMap::new(),
            call_fixups: Vec::new(),
            needs_mul: false,
            needs_div: false,
            needs_mul32: false,
            needs_div32: false,
            base: 0,
            scratch: SCRATCH,
            loop_stack: Vec::new(),
            func_end: None,
        }
    }
    /// Address of local `slot` for the function currently being emitted.
    pub(super) fn slot_addr(&self, slot: usize) -> u16 {
        self.scratch + (self.base + slot as u16) * 2
    }
    pub(super) fn here(&self) -> u16 {
        self.org.wrapping_add(self.code.len() as u16)
    }
    pub(super) fn byte(&mut self, b: u8) {
        self.code.push(b);
    }
    pub(super) fn word(&mut self, w: u16) {
        self.code.push(w as u8);
        self.code.push((w >> 8) as u8);
    }
    pub(super) fn label(&mut self) -> usize {
        self.labels.push(None);
        self.labels.len() - 1
    }
    pub(super) fn place(&mut self, l: usize) {
        let here = self.here();
        self.labels[l] = Some(here);
    }
    pub(super) fn jump(&mut self, opcode: u8, l: usize) {
        self.byte(opcode);
        self.label_fixups.push((self.code.len(), l));
        self.word(0);
    }
    /// Emit a 16-bit operand that resolves to `l`'s placed address — an absolute
    /// data reference (`LD HL,(label)` / `LD (label),HL`) inside an emitted routine.
    pub(super) fn word_label(&mut self, l: usize) {
        self.label_fixups.push((self.code.len(), l));
        self.word(0);
    }
    /// Emit `CALL name` (resolved to the symbol address at finish).
    pub(super) fn call(&mut self, name: &str) {
        self.byte(0xCD);
        self.call_fixups.push((self.code.len(), name.to_string()));
        self.word(0);
    }
    pub(super) fn define(&mut self, name: &str) {
        let here = self.here();
        self.symbols.insert(name.to_string(), here);
    }
    /// Resolve fixups and return the image. `Err` on an unknown call target (a `fn`
    /// referenced but never defined — e.g. an unconfigured prelude route) or an unplaced
    /// label (an internal codegen invariant). Returns a diagnostic rather than panicking so
    /// every compile entry surfaces it as a normal compile error.
    pub(super) fn finish(mut self) -> Result<(Vec<u8>, HashMap<String, u16>), String> {
        // Append the micro-runtime routines that were used. The 32-bit routines come
        // first: `__mul32` calls `__mul16`/`__mul16w`, so it turns those flags on.
        if self.needs_mul32 {
            emit_mul32(&mut self);
            emit_mul16w(&mut self);
            self.needs_mul = true;
        }
        if self.needs_div32 {
            emit_divmod32(&mut self);
        }
        if self.needs_mul {
            self.define("__mul16");
            self.code.extend_from_slice(MUL16);
        }
        if self.needs_div {
            self.define("__divmod16");
            self.code.extend_from_slice(DIVMOD16);
        }
        for (pos, l) in &self.label_fixups {
            let a = self.labels[*l].ok_or("rustz80: internal codegen error — unplaced label")?;
            self.code[*pos] = a as u8;
            self.code[*pos + 1] = (a >> 8) as u8;
        }
        for (pos, name) in &self.call_fixups {
            let a = *self
                .symbols
                .get(name)
                .ok_or_else(|| format!("rustz80: unknown call target `{name}`"))?;
            self.code[*pos] = a as u8;
            self.code[*pos + 1] = (a >> 8) as u8;
        }
        Ok((self.code, self.symbols))
    }
}
