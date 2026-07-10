//! The RV32 symbolic instruction layer — the `Ins` sibling of rustz80's
//! (docs 13 §2.3: per-ISA `Ins`, shared *discipline*, not shared code). Operands
//! stay symbolic (labels) until [`encode`]; every emitted word is a base RV32I or
//! M-extension instruction, laid little-endian.
//!
//! Encodings follow the RISC-V unprivileged spec exactly (R/I/S/B/U/J formats);
//! the encoding goldens in `tests/encodings.rs` pin known-good words, and the
//! **Sail model stands as the emission adversary in CI** (linux-only job — spec
//! §6 risk 2; the goldens are the local, always-on check).

/// An RV32 integer register, by ABI name. `X0` reads zero and swallows writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Reg {
    X0 = 0,
    /// Return address.
    Ra = 1,
    /// Stack pointer.
    Sp = 2,
    /// Temporaries.
    T0 = 5,
    T1 = 6,
    T2 = 7,
    /// Frame/saved.
    S0 = 8,
    S1 = 9,
    /// Arguments / returns (the ≤3-arg family convention uses a0–a2).
    A0 = 10,
    A1 = 11,
    A2 = 12,
    A3 = 13,
    A4 = 14,
    A5 = 15,
}

impl Reg {
    fn n(self) -> u32 {
        self as u32
    }
}

/// Branch conditions (the B-type funct3 values).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bcc {
    Eq,
    Ne,
    Lt,
    Ge,
    Ltu,
    Geu,
}

impl Bcc {
    fn funct3(self) -> u32 {
        match self {
            Bcc::Eq => 0b000,
            Bcc::Ne => 0b001,
            Bcc::Lt => 0b100,
            Bcc::Ge => 0b101,
            Bcc::Ltu => 0b110,
            Bcc::Geu => 0b111,
        }
    }
}

/// Register-register ALU ops (R-type, opcode `OP`), including the M extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alu {
    Add,
    Sub,
    Sll,
    Slt,
    Sltu,
    Xor,
    Srl,
    Sra,
    Or,
    And,
    // M extension (funct7 = 0000001)
    Mul,
    Mulh,
    Mulhsu,
    Mulhu,
    Div,
    Divu,
    Rem,
    Remu,
}

impl Alu {
    fn f3_f7(self) -> (u32, u32) {
        match self {
            Alu::Add => (0b000, 0),
            Alu::Sub => (0b000, 0b0100000),
            Alu::Sll => (0b001, 0),
            Alu::Slt => (0b010, 0),
            Alu::Sltu => (0b011, 0),
            Alu::Xor => (0b100, 0),
            Alu::Srl => (0b101, 0),
            Alu::Sra => (0b101, 0b0100000),
            Alu::Or => (0b110, 0),
            Alu::And => (0b111, 0),
            Alu::Mul => (0b000, 1),
            Alu::Mulh => (0b001, 1),
            Alu::Mulhsu => (0b010, 1),
            Alu::Mulhu => (0b011, 1),
            Alu::Div => (0b100, 1),
            Alu::Divu => (0b101, 1),
            Alu::Rem => (0b110, 1),
            Alu::Remu => (0b111, 1),
        }
    }
}

/// Immediate ALU ops (I-type, opcode `OP-IMM`). Shifts carry their amount in the
/// immediate's low five bits (the encoder masks).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AluI {
    Addi,
    Slti,
    Sltiu,
    Xori,
    Ori,
    Andi,
    Slli,
    Srli,
    Srai,
}

impl AluI {
    fn f3_f7(self) -> (u32, Option<u32>) {
        match self {
            AluI::Addi => (0b000, None),
            AluI::Slti => (0b010, None),
            AluI::Sltiu => (0b011, None),
            AluI::Xori => (0b100, None),
            AluI::Ori => (0b110, None),
            AluI::Andi => (0b111, None),
            AluI::Slli => (0b001, Some(0)),
            AluI::Srli => (0b101, Some(0)),
            AluI::Srai => (0b101, Some(0b0100000)),
        }
    }
}

/// Load widths (I-type, opcode `LOAD`). `Lh`/`Lb` sign-extend; the `u` forms zero-extend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadW {
    Lb,
    Lh,
    Lw,
    Lbu,
    Lhu,
}

/// Store widths (S-type, opcode `STORE`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreW {
    Sb,
    Sh,
    Sw,
}

/// One symbolic RV32 instruction. Branch/jump targets are label ids placed with
/// [`Ins::At`]; everything else is concrete. (`Call`/`Def` symbols arrive with
/// the B1 codegen — labels carry the bootstrap.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ins {
    /// A label placement (assembles to nothing).
    At(usize),
    /// `rd = imm << 12` (U-type LUI).
    Lui(Reg, u32),
    /// R-type ALU: `rd = rs1 <op> rs2`.
    Op(Alu, Reg, Reg, Reg),
    /// I-type ALU: `rd = rs1 <op> imm` (imm is a sign-extended 12-bit value).
    OpI(AluI, Reg, Reg, i32),
    /// `rd = mem[rs1 + off]` at the load width.
    Load(LoadW, Reg, Reg, i32),
    /// `mem[rs1 + off] = rs2` at the store width.
    Store(StoreW, Reg, Reg, i32),
    /// Conditional branch to a label.
    Branch(Bcc, Reg, Reg, usize),
    /// `jal rd, label` — `rd = pc + 4`, jump.
    Jal(Reg, usize),
    /// `jalr rd, rs1, off` — indirect jump (bit 0 of the target cleared).
    Jalr(Reg, Reg, i32),
    /// The executor trap (`ecall`) — forbidden in deployed cells (docs 13 §2.1).
    Ecall,
}

/// The encoded form of `ins` with branch/jump displacements resolved, plus each
/// label's byte offset. Two passes, like the Z80 encoder: place labels (every
/// real instruction is 4 bytes), then emit. `Err` on an unplaced label or an
/// out-of-range displacement/immediate — diagnostics, never silent truncation.
pub fn encode(ins: &[Ins]) -> Result<Vec<u8>, String> {
    // Pass 1: label offsets.
    let mut labels: Vec<Option<u32>> = Vec::new();
    let mut pc = 0u32;
    for i in ins {
        if let Ins::At(l) = i {
            if labels.len() <= *l {
                labels.resize(l + 1, None);
            }
            labels[*l] = Some(pc);
        } else {
            pc += 4;
        }
    }
    let label = |l: usize, at: u32| -> Result<i32, String> {
        let target = labels
            .get(l)
            .copied()
            .flatten()
            .ok_or_else(|| format!("rustrv32: unplaced label {l}"))?;
        Ok(target as i32 - at as i32)
    };

    // Pass 2: emit.
    let mut out = Vec::with_capacity(ins.len() * 4);
    let mut pc = 0u32;
    for i in ins {
        let word = match i {
            Ins::At(_) => continue,
            Ins::Lui(rd, imm20) => {
                if *imm20 > 0xF_FFFF {
                    return Err(format!(
                        "rustrv32: LUI immediate {imm20:#x} exceeds 20 bits"
                    ));
                }
                (imm20 << 12) | (rd.n() << 7) | 0b0110111
            }
            Ins::Op(op, rd, rs1, rs2) => {
                let (f3, f7) = op.f3_f7();
                (f7 << 25)
                    | (rs2.n() << 20)
                    | (rs1.n() << 15)
                    | (f3 << 12)
                    | (rd.n() << 7)
                    | 0b0110011
            }
            Ins::OpI(op, rd, rs1, imm) => {
                let (f3, shift_f7) = op.f3_f7();
                let imm = match shift_f7 {
                    // Shift amounts live in imm[4:0] with funct7 above.
                    Some(f7) => {
                        if !(0..32).contains(imm) {
                            return Err(format!("rustrv32: shift amount {imm} out of range"));
                        }
                        (f7 << 5) | *imm as u32
                    }
                    None => imm12(*imm)?,
                };
                (imm << 20) | (rs1.n() << 15) | (f3 << 12) | (rd.n() << 7) | 0b0010011
            }
            Ins::Load(w, rd, rs1, off) => {
                let f3 = match w {
                    LoadW::Lb => 0b000,
                    LoadW::Lh => 0b001,
                    LoadW::Lw => 0b010,
                    LoadW::Lbu => 0b100,
                    LoadW::Lhu => 0b101,
                };
                (imm12(*off)? << 20) | (rs1.n() << 15) | (f3 << 12) | (rd.n() << 7) | 0b0000011
            }
            Ins::Store(w, rs1, rs2, off) => {
                let f3 = match w {
                    StoreW::Sb => 0b000,
                    StoreW::Sh => 0b001,
                    StoreW::Sw => 0b010,
                };
                let imm = imm12(*off)?;
                ((imm >> 5) << 25)
                    | (rs2.n() << 20)
                    | (rs1.n() << 15)
                    | (f3 << 12)
                    | ((imm & 0x1F) << 7)
                    | 0b0100011
            }
            Ins::Branch(cc, rs1, rs2, l) => {
                let d = label(*l, pc)?;
                if !(-4096..4096).contains(&d) || d & 1 != 0 {
                    return Err(format!("rustrv32: branch displacement {d} out of range"));
                }
                let d = d as u32;
                (((d >> 12) & 1) << 31)
                    | (((d >> 5) & 0x3F) << 25)
                    | (rs2.n() << 20)
                    | (rs1.n() << 15)
                    | (cc.funct3() << 12)
                    | (((d >> 1) & 0xF) << 8)
                    | (((d >> 11) & 1) << 7)
                    | 0b1100011
            }
            Ins::Jal(rd, l) => {
                let d = label(*l, pc)?;
                if !(-(1 << 20)..(1 << 20)).contains(&d) || d & 1 != 0 {
                    return Err(format!("rustrv32: jal displacement {d} out of range"));
                }
                let d = d as u32;
                (((d >> 20) & 1) << 31)
                    | (((d >> 1) & 0x3FF) << 21)
                    | (((d >> 11) & 1) << 20)
                    | (((d >> 12) & 0xFF) << 12)
                    | (rd.n() << 7)
                    | 0b1101111
            }
            Ins::Jalr(rd, rs1, off) => {
                (imm12(*off)? << 20) | (rs1.n() << 15) | (rd.n() << 7) | 0b1100111
            }
            Ins::Ecall => 0b1110011,
        };
        out.extend_from_slice(&word.to_le_bytes());
        pc += 4;
    }
    Ok(out)
}

/// A sign-extended 12-bit immediate, encoded into its field bits.
fn imm12(v: i32) -> Result<u32, String> {
    if !(-2048..2048).contains(&v) {
        return Err(format!("rustrv32: immediate {v} exceeds 12 bits"));
    }
    Ok((v as u32) & 0xFFF)
}
