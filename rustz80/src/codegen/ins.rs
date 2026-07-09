//! The instruction layer (Stage 2 seam): codegen emits a list of [`Ins`] with
//! **symbolic operands** — labels, call targets, and locals *slots* stay symbolic —
//! and a final [`encode`] pass assigns PCs, resolves operands against the scratch
//! base, and lowers to bytes. Interposing this between codegen and the image is what
//! makes a peephole pass, register tracking, and instruction-level measurement
//! possible: raw bytes have no boundaries to rewrite.
//!
//! Every variant has a **static encoded length** (operands are always 2-byte
//! immediates), so code length is invariant to the scratch *value* — the property the
//! frame loop's measure-then-place pass relies on.

use std::collections::HashMap;

/// A 16-bit register pair operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum R16 {
    Hl,
    De,
    Bc,
    Af,
}

/// A 16-bit immediate operand, symbolic until [`encode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Imm {
    /// A concrete value (literal, absolute address, port, offset…).
    Abs(u16),
    /// The address of locals slot `idx` (function base already folded in) plus a byte
    /// offset (`2` = the high word of a `u32` slot pair): `scratch + idx*2 + off`.
    Slot(u16, u16),
    /// The placed address of a label (an absolute data reference into emitted code).
    Label(usize),
}

/// One emitted instruction (or stream marker), operands symbolic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Ins {
    /// `LD rr, imm` (`HL`/`DE`/`BC`).
    LdImm(R16, Imm),
    /// `LD HL, (imm)`.
    LdHlMem(Imm),
    /// `LD A, (imm)` — the 8-bit accumulator lane (a `u8` value loaded from a slot).
    LdAMem(Imm),
    /// `LD (imm), HL`.
    StHlMem(Imm),
    /// `LD DE/BC, (imm)` (the `ED` forms).
    LdWideMem(R16, Imm),
    /// `LD (imm), DE/BC` (the `ED` forms).
    StWideMem(R16, Imm),
    /// `PUSH rr`.
    Push(R16),
    /// `POP rr`.
    Pop(R16),
    /// `EX DE, HL`.
    ExDeHl,
    /// `ADD HL, rr` (`BC`/`DE`/`HL`).
    AddHl(R16),
    /// An absolute jump — the opcode (`JP` / `JP cc`: `C3`/`CA`/`C2`/`DA`/`D2`/`E2`/
    /// `F2`/`FA`) and the target label.
    Jp(u8, usize),
    /// `CALL name`, resolved against the symbol table at encode.
    Call(String),
    /// `LD rr, name` — an immediate load of a *symbol's* address (a const-data
    /// item), resolved against the symbol table at encode like [`Ins::Call`].
    LdImmSym(R16, String),
    /// Raw data bytes laid into the image (the const-data section) — owned, unlike
    /// the `'static` runtime [`Ins::Blob`]s.
    Bytes(Vec<u8>),
    /// Any other single instruction, as its exact bytes (`RET`, `LD r,r'`, `CB`/`ED`
    /// prefixed ops, immediates like `LD A,n` — one instruction per entry, so
    /// boundaries survive).
    Fx(FxBytes),
    /// Place label `l` here.
    At(usize),
    /// Define symbol `name` here.
    Def(String),
    /// A 2-byte data word (static scratch words after a runtime routine).
    Word(Imm),
    /// A hand-assembled runtime routine, appended verbatim (`__mul16`/`__divmod16`).
    Blob(&'static [u8]),
}

/// The exact bytes of one fixed instruction (longest used: prefix + operand = 2;
/// room for one more).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FxBytes {
    len: u8,
    b: [u8; 3],
}

impl FxBytes {
    pub(super) fn new(bytes: &[u8]) -> Self {
        debug_assert!((1..=3).contains(&bytes.len()), "one instruction per Fx");
        let mut b = [0u8; 3];
        b[..bytes.len()].copy_from_slice(bytes);
        FxBytes {
            len: bytes.len() as u8,
            b,
        }
    }
    pub(super) fn bytes(&self) -> &[u8] {
        &self.b[..self.len as usize]
    }
}

impl Ins {
    /// Encoded length in bytes — static per variant (operands are always 2-byte
    /// immediates), which is what keeps code length independent of the scratch base.
    pub(super) fn len(&self) -> u16 {
        match self {
            Ins::Push(_) | Ins::Pop(_) | Ins::ExDeHl | Ins::AddHl(_) => 1,
            Ins::Word(_) => 2,
            Ins::LdImm(..)
            | Ins::LdImmSym(..)
            | Ins::LdHlMem(_)
            | Ins::LdAMem(_)
            | Ins::StHlMem(_)
            | Ins::Jp(..)
            | Ins::Call(_) => 3,
            Ins::LdWideMem(..) | Ins::StWideMem(..) => 4,
            Ins::Fx(fx) => fx.len as u16,
            Ins::At(_) | Ins::Def(_) => 0,
            Ins::Blob(b) => b.len() as u16,
            Ins::Bytes(b) => b.len() as u16,
        }
    }
}

/// Total encoded length of an instruction stream.
pub(super) fn stream_len(ins: &[Ins]) -> u16 {
    ins.iter().map(Ins::len).fold(0u16, u16::wrapping_add)
}

/// Assign PCs and lower to bytes: place labels/symbols (pass 1 — lengths are static),
/// then emit with every [`Imm`] resolved (`Slot` against `scratch`). `Err` on an
/// unknown call target (a `fn` referenced but never defined — e.g. an unconfigured
/// prelude route) or an unplaced label (an internal codegen invariant); a diagnostic
/// rather than a panic so every compile entry surfaces it as a normal compile error.
pub(super) fn encode(
    ins: &[Ins],
    org: u16,
    scratch: u16,
    n_labels: usize,
    externs: &HashMap<String, u16>,
) -> Result<(Vec<u8>, HashMap<String, u16>), String> {
    // Pass 1: label/symbol addresses. Extern symbols (the resident kernel bank)
    // seed the table at their absolute addresses; a local definition of the same
    // name shadows them (pass 1 inserts over the seed).
    let mut labels: Vec<Option<u16>> = vec![None; n_labels];
    let mut symbols: HashMap<String, u16> = externs.clone();
    let mut pc = org;
    for i in ins {
        match i {
            Ins::At(l) => labels[*l] = Some(pc),
            Ins::Def(name) => {
                symbols.insert(name.clone(), pc);
            }
            _ => pc = pc.wrapping_add(i.len()),
        }
    }
    let imm = |m: &Imm| -> Result<u16, String> {
        Ok(match m {
            Imm::Abs(v) => *v,
            Imm::Slot(idx, off) => scratch.wrapping_add(idx.wrapping_mul(2)).wrapping_add(*off),
            Imm::Label(l) => {
                labels[*l].ok_or("rustz80: internal codegen error — unplaced label")?
            }
        })
    };

    // Pass 2: emit.
    let mut code = Vec::with_capacity(pc.wrapping_sub(org) as usize);
    let word = |code: &mut Vec<u8>, v: u16| {
        code.push(v as u8);
        code.push((v >> 8) as u8);
    };
    for i in ins {
        match i {
            Ins::LdImm(r, m) => {
                code.push(match r {
                    R16::Hl => 0x21,
                    R16::De => 0x11,
                    R16::Bc => 0x01,
                    R16::Af => unreachable!("LD AF,imm does not exist"),
                });
                word(&mut code, imm(m)?);
            }
            Ins::LdHlMem(m) => {
                code.push(0x2A);
                word(&mut code, imm(m)?);
            }
            Ins::LdAMem(m) => {
                code.push(0x3A);
                word(&mut code, imm(m)?);
            }
            Ins::StHlMem(m) => {
                code.push(0x22);
                word(&mut code, imm(m)?);
            }
            Ins::LdWideMem(r, m) => {
                code.push(0xED);
                code.push(match r {
                    R16::De => 0x5B,
                    R16::Bc => 0x4B,
                    _ => unreachable!("ED loads exist for DE/BC only"),
                });
                word(&mut code, imm(m)?);
            }
            Ins::StWideMem(r, m) => {
                code.push(0xED);
                code.push(match r {
                    R16::De => 0x53,
                    R16::Bc => 0x43,
                    _ => unreachable!("ED stores exist for DE/BC only"),
                });
                word(&mut code, imm(m)?);
            }
            Ins::Push(r) => code.push(match r {
                R16::Hl => 0xE5,
                R16::De => 0xD5,
                R16::Bc => 0xC5,
                R16::Af => 0xF5,
            }),
            Ins::Pop(r) => code.push(match r {
                R16::Hl => 0xE1,
                R16::De => 0xD1,
                R16::Bc => 0xC1,
                R16::Af => 0xF1,
            }),
            Ins::ExDeHl => code.push(0xEB),
            Ins::AddHl(r) => code.push(match r {
                R16::Bc => 0x09,
                R16::De => 0x19,
                R16::Hl => 0x29,
                R16::Af => unreachable!("ADD HL,AF does not exist"),
            }),
            Ins::Jp(op, l) => {
                code.push(*op);
                let a = labels[*l].ok_or("rustz80: internal codegen error — unplaced label")?;
                word(&mut code, a);
            }
            Ins::Call(name) => {
                code.push(0xCD);
                let a = *symbols
                    .get(name)
                    .ok_or_else(|| format!("rustz80: unknown call target `{name}`"))?;
                word(&mut code, a);
            }
            Ins::LdImmSym(r, name) => {
                code.push(match r {
                    R16::Hl => 0x21,
                    R16::De => 0x11,
                    R16::Bc => 0x01,
                    R16::Af => unreachable!("LD AF,imm does not exist"),
                });
                let a = *symbols.get(name).ok_or_else(|| {
                    format!(
                        "rustz80: unknown const-data symbol `{name}` — was the const's \
                         data section emitted (a `*_full` codegen entry)?"
                    )
                })?;
                word(&mut code, a);
            }
            Ins::Fx(fx) => code.extend_from_slice(fx.bytes()),
            Ins::At(_) | Ins::Def(_) => {}
            Ins::Word(m) => word(&mut code, imm(m)?),
            Ins::Blob(b) => code.extend_from_slice(b),
            Ins::Bytes(b) => code.extend_from_slice(b),
        }
    }
    Ok((code, symbols))
}
