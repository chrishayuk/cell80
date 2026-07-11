//! The independent-assembler adversary (WS-B/B1a): every `Ins` shape rendered to
//! GNU assembly and assembled by **binutils' RISC-V gas** — a fully independent
//! implementation of the encodings — then compared byte-for-byte against our
//! encoder. This breaks the encoder's self-refereeing (encoder and executor
//! otherwise share one reading of the ISA manual, so a systematic field-packing
//! error would agree with itself straight through the five-system battery).
//!
//! Scope note (spec §6 risk 2): this is the *emission* adversary. The Sail model
//! (or spike, the pre-registered fallback) as the *execution* adversary is still
//! owed for B2 — gas checks what bytes mean to an assembler, not what they do.
//!
//! Discovery: `$RV32_GAS`, then `riscv64-elf-as` (brew), then
//! `riscv64-unknown-elf-as` (apt). Absent → the test skips locally with a note;
//! CI's ubuntu leg installs binutils and sets `RV32_GAS_REQUIRED=1`, so the
//! adversary can never silently vanish from the gate.

use rustrv32::{encode, Alu, AluI, Bcc, Ins, LoadW, Reg, StoreW};
use std::process::Command;

fn find_gas() -> Option<String> {
    if let Ok(p) = std::env::var("RV32_GAS") {
        return Some(p);
    }
    for cand in [
        "riscv64-elf-as",
        "riscv64-unknown-elf-as",
        "riscv32-unknown-elf-as",
    ] {
        if Command::new(cand)
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
        {
            return Some(cand.to_string());
        }
    }
    None
}

fn reg(r: Reg) -> &'static str {
    match r {
        Reg::X0 => "zero",
        Reg::Ra => "ra",
        Reg::Sp => "sp",
        Reg::T0 => "t0",
        Reg::T1 => "t1",
        Reg::T2 => "t2",
        Reg::S0 => "s0",
        Reg::S1 => "s1",
        Reg::A0 => "a0",
        Reg::A1 => "a1",
        Reg::A2 => "a2",
        Reg::A3 => "a3",
        Reg::A4 => "a4",
        Reg::A5 => "a5",
    }
}

fn alu(op: Alu) -> &'static str {
    match op {
        Alu::Add => "add",
        Alu::Sub => "sub",
        Alu::Sll => "sll",
        Alu::Slt => "slt",
        Alu::Sltu => "sltu",
        Alu::Xor => "xor",
        Alu::Srl => "srl",
        Alu::Sra => "sra",
        Alu::Or => "or",
        Alu::And => "and",
        Alu::Mul => "mul",
        Alu::Mulh => "mulh",
        Alu::Mulhsu => "mulhsu",
        Alu::Mulhu => "mulhu",
        Alu::Div => "div",
        Alu::Divu => "divu",
        Alu::Rem => "rem",
        Alu::Remu => "remu",
    }
}

fn alui(op: AluI) -> &'static str {
    match op {
        AluI::Addi => "addi",
        AluI::Slti => "slti",
        AluI::Sltiu => "sltiu",
        AluI::Xori => "xori",
        AluI::Ori => "ori",
        AluI::Andi => "andi",
        AluI::Slli => "slli",
        AluI::Srli => "srli",
        AluI::Srai => "srai",
    }
}

/// Render a stream to GNU assembly. Labels/symbols print as `.L<n>`/names, so
/// gas resolves displacements **independently** — the field packing *and* the
/// two-pass placement both face the adversary.
fn render(ins: &[Ins]) -> String {
    let mut out = String::from(".text\n");
    for i in ins {
        let line = match i {
            Ins::At(l) => format!(".L{l}:"),
            Ins::Def(name) => format!("{name}:"),
            Ins::Call(name) => format!("jal ra, {name}"),
            Ins::Lui(rd, imm) => format!("lui {}, {imm}", reg(*rd)),
            Ins::Op(op, rd, rs1, rs2) => {
                format!("{} {}, {}, {}", alu(*op), reg(*rd), reg(*rs1), reg(*rs2))
            }
            Ins::OpI(op, rd, rs1, imm) => {
                format!("{} {}, {}, {imm}", alui(*op), reg(*rd), reg(*rs1))
            }
            Ins::Load(w, rd, rs1, off) => {
                let m = match w {
                    LoadW::Lb => "lb",
                    LoadW::Lh => "lh",
                    LoadW::Lw => "lw",
                    LoadW::Lbu => "lbu",
                    LoadW::Lhu => "lhu",
                };
                format!("{m} {}, {off}({})", reg(*rd), reg(*rs1))
            }
            Ins::Store(w, rs1, rs2, off) => {
                let m = match w {
                    StoreW::Sb => "sb",
                    StoreW::Sh => "sh",
                    StoreW::Sw => "sw",
                };
                format!("{m} {}, {off}({})", reg(*rs2), reg(*rs1))
            }
            Ins::Branch(cc, rs1, rs2, l) => {
                let m = match cc {
                    Bcc::Eq => "beq",
                    Bcc::Ne => "bne",
                    Bcc::Lt => "blt",
                    Bcc::Ge => "bge",
                    Bcc::Ltu => "bltu",
                    Bcc::Geu => "bgeu",
                };
                format!("{m} {}, {}, .L{l}", reg(*rs1), reg(*rs2))
            }
            Ins::Jal(rd, l) => format!("jal {}, .L{l}", reg(*rd)),
            Ins::Jalr(rd, rs1, off) => format!("jalr {}, {off}({})", reg(*rd), reg(*rs1)),
            Ins::Ecall => "ecall".into(),
        };
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// Assemble with gas and pull `.text` out of the ELF32 relocatable (a minimal
/// section-header walk — no objdump in the loop).
fn gas_bytes(gas: &str, asm: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("rv32-gas-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let s_path = dir.join("t.s");
    let o_path = dir.join("t.o");
    std::fs::write(&s_path, asm).unwrap();
    let out = Command::new(gas)
        .args(["-march=rv32im", "-mabi=ilp32", "-o"])
        .arg(&o_path)
        .arg(&s_path)
        .output()
        .expect("run gas");
    assert!(
        out.status.success(),
        "gas rejected our rendering:\n{}\n---\n{asm}",
        String::from_utf8_lossy(&out.stderr)
    );
    let elf = std::fs::read(&o_path).unwrap();
    elf_text(&elf)
}

/// `.text` from an ELF32 little-endian relocatable.
fn elf_text(elf: &[u8]) -> Vec<u8> {
    let u16v = |o: usize| u16::from_le_bytes([elf[o], elf[o + 1]]) as usize;
    let u32v = |o: usize| u32::from_le_bytes([elf[o], elf[o + 1], elf[o + 2], elf[o + 3]]) as usize;
    assert_eq!(&elf[..4], b"\x7fELF", "not an ELF");
    assert_eq!(elf[4], 1, "not ELF32");
    let shoff = u32v(0x20);
    let shentsize = u16v(0x2E);
    let shnum = u16v(0x30);
    let shstrndx = u16v(0x32);
    let strtab_off = u32v(shoff + shstrndx * shentsize + 0x10);
    for i in 0..shnum {
        let sh = shoff + i * shentsize;
        let name_off = strtab_off + u32v(sh);
        let name_end = elf[name_off..].iter().position(|&b| b == 0).unwrap() + name_off;
        if &elf[name_off..name_end] == b".text" {
            let off = u32v(sh + 0x10);
            let size = u32v(sh + 0x14);
            return elf[off..off + size].to_vec();
        }
    }
    panic!("no .text section");
}

fn check(gas: &str, ins: &[Ins]) {
    let ours = encode(ins).unwrap();
    let theirs = gas_bytes(gas, &render(ins));
    if ours != theirs {
        let at = ours
            .iter()
            .zip(&theirs)
            .position(|(a, b)| a != b)
            .unwrap_or(ours.len().min(theirs.len()));
        panic!(
            "encoder vs gas diverged at byte {at} (word {}):\n  ours:   {}\n  gas:    {}\n  asm:\n{}",
            at / 4,
            hex(&ours),
            hex(&theirs),
            render(ins)
        );
    }
}

fn hex(b: &[u8]) -> String {
    b.chunks(4)
        .map(|w| {
            format!(
                "{:08x}",
                u32::from_le_bytes([w[0], w[1], w[2], w[3.min(w.len() - 1)]])
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn every_instruction_shape_agrees_with_gas() {
    let Some(gas) = find_gas() else {
        if std::env::var("RV32_GAS_REQUIRED").is_ok_and(|v| !v.is_empty()) {
            panic!("RV32_GAS_REQUIRED is set but no RISC-V gas was found");
        }
        eprintln!("skipping: no RISC-V gas on PATH (brew install riscv64-elf-binutils)");
        return;
    };
    use Reg::{Ra, Sp, A0, A1, A5, S1, T0, X0};

    // R-type: every ALU/M op over mixed registers, including x0 and the high regs.
    let mut r_ops = Vec::new();
    for op in [
        Alu::Add,
        Alu::Sub,
        Alu::Sll,
        Alu::Slt,
        Alu::Sltu,
        Alu::Xor,
        Alu::Srl,
        Alu::Sra,
        Alu::Or,
        Alu::And,
        Alu::Mul,
        Alu::Mulh,
        Alu::Mulhsu,
        Alu::Mulhu,
        Alu::Div,
        Alu::Divu,
        Alu::Rem,
        Alu::Remu,
    ] {
        r_ops.push(Ins::Op(op, A0, A1, A5));
        r_ops.push(Ins::Op(op, S1, X0, T0));
    }
    check(&gas, &r_ops);

    // I-type: the immediate edges and the shift-amount edges.
    let mut i_ops = Vec::new();
    for op in [
        AluI::Addi,
        AluI::Slti,
        AluI::Sltiu,
        AluI::Xori,
        AluI::Ori,
        AluI::Andi,
    ] {
        for imm in [0, 1, -1, 7, -8, 2047, -2048] {
            i_ops.push(Ins::OpI(op, A0, S1, imm));
        }
    }
    for op in [AluI::Slli, AluI::Srli, AluI::Srai] {
        for k in [0, 1, 15, 16, 31] {
            i_ops.push(Ins::OpI(op, T0, A1, k));
        }
    }
    check(&gas, &i_ops);

    // Loads/stores: every width at the offset edges.
    let mut mem_ops = Vec::new();
    for w in [LoadW::Lb, LoadW::Lh, LoadW::Lw, LoadW::Lbu, LoadW::Lhu] {
        for off in [0, 1, -1, 2047, -2048] {
            mem_ops.push(Ins::Load(w, A0, Sp, off));
        }
    }
    for w in [StoreW::Sb, StoreW::Sh, StoreW::Sw] {
        for off in [0, 1, -1, 2047, -2048] {
            mem_ops.push(Ins::Store(w, Sp, A1, off));
        }
    }
    check(&gas, &mem_ops);

    // LUI edges, JALR offsets, ecall.
    check(
        &gas,
        &[
            Ins::Lui(A0, 0),
            Ins::Lui(T0, 1),
            Ins::Lui(S1, 0x12345),
            Ins::Lui(A5, 0xF_FFFF),
            Ins::Jalr(X0, Ra, 0),
            Ins::Jalr(Ra, T0, 2047),
            Ins::Jalr(A0, S1, -2048),
            Ins::Ecall,
        ],
    );

    // Branches: all six conditions, forward and backward, and a distance that
    // crosses the imm[11]/imm[12] field boundaries — gas resolves the labels
    // itself, so placement *and* packing face the adversary.
    for cc in [Bcc::Eq, Bcc::Ne, Bcc::Lt, Bcc::Ge, Bcc::Ltu, Bcc::Geu] {
        let mut prog = vec![
            Ins::At(0),
            Ins::OpI(AluI::Addi, A0, A0, 1),
            Ins::Branch(cc, A0, A1, 0), // backward
            Ins::Branch(cc, A0, A1, 1), // forward, over the filler
        ];
        for _ in 0..600 {
            prog.push(Ins::OpI(AluI::Addi, X0, X0, 0)); // 2400 bytes of filler
        }
        prog.push(Ins::At(1));
        prog.push(Ins::Jalr(X0, Ra, 0));
        check(&gas, &prog);
    }

    // JAL: forward/backward, x0 and ra, plus a Call through a Def symbol.
    let mut prog = vec![
        Ins::Def("helper".into()),
        Ins::OpI(AluI::Addi, A0, A0, 2),
        Ins::Jalr(X0, Ra, 0),
        Ins::At(0),
        Ins::Jal(X0, 0), // self
        Ins::Jal(Ra, 1), // forward
        Ins::Call("helper".into()),
    ];
    for _ in 0..300 {
        prog.push(Ins::OpI(AluI::Addi, X0, X0, 0));
    }
    prog.push(Ins::At(1));
    prog.push(Ins::Jal(X0, 0)); // far backward
    check(&gas, &prog);
}
