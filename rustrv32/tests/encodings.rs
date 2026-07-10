//! Encoding goldens: known-good RV32I(M) words pinned against the encoder. The
//! local, always-on half of B1's emission verification — the Sail model joins as
//! the CI adversary (linux-only job, spec §6 risk 2).

use rustrv32::{encode, Alu, AluI, Bcc, Ins, LoadW, Reg, StoreW};

fn one(i: Ins) -> u32 {
    let b = encode(&[i]).unwrap();
    assert_eq!(b.len(), 4);
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

#[test]
fn golden_words() {
    // Assembled reference encodings (RISC-V unprivileged spec formats).
    assert_eq!(one(Ins::OpI(AluI::Addi, Reg::X0, Reg::X0, 0)), 0x0000_0013); // nop
    assert_eq!(one(Ins::OpI(AluI::Addi, Reg::A0, Reg::A0, -1)), 0xFFF5_0513); // addi a0,a0,-1
    assert_eq!(one(Ins::Jalr(Reg::X0, Reg::Ra, 0)), 0x0000_8067); // ret
    assert_eq!(one(Ins::Ecall), 0x0000_0073);
    assert_eq!(one(Ins::Lui(Reg::T0, 0x12345)), 0x1234_52B7); // lui t0,0x12345
    assert_eq!(
        one(Ins::Op(Alu::Add, Reg::A0, Reg::A1, Reg::A2)),
        0x00C5_8533
    ); // add a0,a1,a2
    assert_eq!(
        one(Ins::Op(Alu::Mul, Reg::A0, Reg::A1, Reg::A2)),
        0x02C5_8533
    ); // mul a0,a1,a2
    assert_eq!(
        one(Ins::Op(Alu::Div, Reg::A0, Reg::A1, Reg::A2)),
        0x02C5_C533
    ); // div a0,a1,a2
    assert_eq!(one(Ins::OpI(AluI::Srai, Reg::A0, Reg::A0, 4)), 0x4045_5513); // srai a0,a0,4
    assert_eq!(
        one(Ins::Store(StoreW::Sw, Reg::A0, Reg::A1, 4)),
        0x00B5_2223
    ); // sw a1,4(a0)
    assert_eq!(one(Ins::Load(LoadW::Lw, Reg::A2, Reg::A0, 4)), 0x0045_2603); // lw a2,4(a0)
}

#[test]
fn branch_and_jump_displacements_resolve() {
    // beq a0,a1,+8 — branch over one instruction to a placed label.
    let code = encode(&[
        Ins::Branch(Bcc::Eq, Reg::A0, Reg::A1, 0),
        Ins::OpI(AluI::Addi, Reg::A0, Reg::A0, 1),
        Ins::At(0),
    ])
    .unwrap();
    assert_eq!(
        u32::from_le_bytes([code[0], code[1], code[2], code[3]]),
        0x00B5_0463
    );
    // jal x0, 0 — a self-jump encodes as the canonical 0x6F word.
    let code = encode(&[Ins::At(1), Ins::Jal(Reg::X0, 1)]).unwrap();
    assert_eq!(
        u32::from_le_bytes([code[0], code[1], code[2], code[3]]),
        0x0000_006F
    );
    // A backward branch: negative displacement, sign bits in the right fields.
    let code = encode(&[
        Ins::At(2),
        Ins::OpI(AluI::Addi, Reg::A0, Reg::A0, -1),
        Ins::Branch(Bcc::Ne, Reg::A0, Reg::X0, 2),
    ])
    .unwrap();
    // bne a0,x0,-4
    assert_eq!(
        u32::from_le_bytes([code[4], code[5], code[6], code[7]]),
        0xFE05_1EE3
    );
}

#[test]
fn range_errors_are_diagnostics_not_truncation() {
    assert!(encode(&[Ins::OpI(AluI::Addi, Reg::A0, Reg::A0, 2048)])
        .unwrap_err()
        .contains("12 bits"));
    assert!(encode(&[Ins::OpI(AluI::Slli, Reg::A0, Reg::A0, 32)])
        .unwrap_err()
        .contains("shift amount"));
    assert!(encode(&[Ins::Jal(Reg::X0, 9)])
        .unwrap_err()
        .contains("unplaced label"));
    assert!(encode(&[Ins::Lui(Reg::A0, 0x10_0000)])
        .unwrap_err()
        .contains("20 bits"));
}
