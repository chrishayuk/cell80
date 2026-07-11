//! Executor semantics + determinism (WS-B/B2): the RISC-V M-extension corner
//! cases the spec pins (div-by-zero all-ones/dividend, MIN/-1 wrap), load
//! sign-extension, branch orderings, and the determinism fingerprint (rerun /
//! fresh instance ⇒ identical result *and* cycles). The gcd program is the
//! M1-shape smoke test: hand-assembled Euclid vs a host oracle.

use rustrv32::{encode, run_fn, Alu, AluI, Bcc, Ins, LoadW, Reg, StoreW};

const MEM: usize = 0x8000;
const FUEL: u64 = 100_000;

fn run1(ins: &[Ins], args: &[u32]) -> (u32, u64) {
    let code = encode(ins).unwrap();
    let (a0, cycles, stop) = run_fn(&code, args, MEM, FUEL);
    assert_eq!(stop, rustrv32::Stop::Returned, "unexpected stop");
    (a0, cycles)
}

fn ret() -> Ins {
    Ins::Jalr(Reg::X0, Reg::Ra, 0)
}

#[test]
fn m_extension_corner_semantics() {
    // div/rem by zero: all-ones quotient, dividend remainder — no trap.
    let div = [Ins::Op(Alu::Div, Reg::A0, Reg::A0, Reg::A1), ret()];
    assert_eq!(run1(&div, &[7, 0]).0, u32::MAX);
    let divu = [Ins::Op(Alu::Divu, Reg::A0, Reg::A0, Reg::A1), ret()];
    assert_eq!(run1(&divu, &[7, 0]).0, u32::MAX);
    let rem = [Ins::Op(Alu::Rem, Reg::A0, Reg::A0, Reg::A1), ret()];
    assert_eq!(run1(&rem, &[7, 0]).0, 7);
    // Signed overflow: MIN / -1 = MIN, MIN % -1 = 0 (wraps, no trap).
    assert_eq!(run1(&div, &[0x8000_0000, u32::MAX]).0, 0x8000_0000);
    assert_eq!(run1(&rem, &[0x8000_0000, u32::MAX]).0, 0);
    // Signed division truncates toward zero.
    assert_eq!(run1(&div, &[(-7i32) as u32, 2]).0, (-3i32) as u32);
    // The high-half multiplies.
    let mulh = [Ins::Op(Alu::Mulh, Reg::A0, Reg::A0, Reg::A1), ret()];
    assert_eq!(
        run1(&mulh, &[(-1i32) as u32, 2]).0,
        (-1i32) as u32 // (-1 × 2) >> 32 = -1
    );
    let mulhu = [Ins::Op(Alu::Mulhu, Reg::A0, Reg::A0, Reg::A1), ret()];
    assert_eq!(run1(&mulhu, &[0x8000_0000, 4]).0, 2);
}

#[test]
fn loads_extend_and_stores_narrow() {
    // sb/lb sign-extends; lbu zero-extends; sh/lh likewise at halfword width.
    let ins = [
        Ins::Store(StoreW::Sb, Reg::Sp, Reg::A0, -4),
        Ins::Load(LoadW::Lb, Reg::A1, Reg::Sp, -4),
        Ins::Load(LoadW::Lbu, Reg::A2, Reg::Sp, -4),
        Ins::Op(Alu::Sub, Reg::A0, Reg::A1, Reg::A2), // sext − zext
        ret(),
    ];
    // 0xFF: lb = -1, lbu = 255 → difference -256.
    assert_eq!(run1(&ins, &[0xFF]).0, (-256i32) as u32);
    let ins = [
        Ins::Store(StoreW::Sh, Reg::Sp, Reg::A0, -4),
        Ins::Load(LoadW::Lh, Reg::A1, Reg::Sp, -4),
        Ins::Load(LoadW::Lhu, Reg::A2, Reg::Sp, -4),
        Ins::Op(Alu::Xor, Reg::A0, Reg::A1, Reg::A2),
        ret(),
    ];
    // 0x8000: lh = 0xFFFF8000, lhu = 0x00008000 → xor = 0xFFFF0000.
    assert_eq!(run1(&ins, &[0x8000]).0, 0xFFFF_0000);
}

#[test]
fn branches_order_signed_and_unsigned() {
    // blt (signed): -1 < 1; bltu (unsigned): 0xFFFF_FFFF > 1.
    let signed = [
        Ins::Branch(Bcc::Lt, Reg::A0, Reg::A1, 0),
        Ins::OpI(AluI::Addi, Reg::A0, Reg::X0, 0),
        ret(),
        Ins::At(0),
        Ins::OpI(AluI::Addi, Reg::A0, Reg::X0, 1),
        ret(),
    ];
    assert_eq!(run1(&signed, &[(-1i32) as u32, 1]).0, 1);
    let unsigned = [
        Ins::Branch(Bcc::Ltu, Reg::A0, Reg::A1, 0),
        Ins::OpI(AluI::Addi, Reg::A0, Reg::X0, 0),
        ret(),
        Ins::At(0),
        Ins::OpI(AluI::Addi, Reg::A0, Reg::X0, 1),
        ret(),
    ];
    assert_eq!(run1(&unsigned, &[(-1i32) as u32, 1]).0, 0);
}

#[test]
fn x0_swallows_writes_and_reads_zero() {
    let ins = [
        Ins::OpI(AluI::Addi, Reg::X0, Reg::A0, 0), // "write" a0 into x0
        Ins::Op(Alu::Add, Reg::A0, Reg::X0, Reg::X0), // read it back: 0
        ret(),
    ];
    assert_eq!(run1(&ins, &[1234]).0, 0);
}

/// The gcd smoke test — Euclid by remainder, hand-assembled, vs a host oracle.
fn gcd_ins() -> Vec<Ins> {
    vec![
        Ins::At(0),
        Ins::Branch(Bcc::Eq, Reg::A1, Reg::X0, 1), // while a1 != 0
        Ins::Op(Alu::Remu, Reg::T0, Reg::A0, Reg::A1), // t0 = a0 % a1
        Ins::Op(Alu::Add, Reg::A0, Reg::A1, Reg::X0), // a0 = a1
        Ins::Op(Alu::Add, Reg::A1, Reg::T0, Reg::X0), // a1 = t0
        Ins::Jal(Reg::X0, 0),
        Ins::At(1),
        ret(),
    ]
}

#[test]
fn gcd_matches_the_host_oracle() {
    fn gcd(mut a: u32, mut b: u32) -> u32 {
        while b != 0 {
            (a, b) = (b, a % b);
        }
        a
    }
    for (a, b) in [(48, 18), (17, 5), (0, 9), (9, 0), (270, 192), (1, 1)] {
        assert_eq!(run1(&gcd_ins(), &[a, b]).0, gcd(a, b), "gcd({a},{b})");
    }
}

#[test]
fn determinism_rerun_and_fresh_instance() {
    // Same program, fresh executor: identical result and identical cycles —
    // the Snapshot discipline (result, cycles, stop) at RV32.
    let code = encode(&gcd_ins()).unwrap();
    let first = run_fn(&code, &[270, 192], MEM, FUEL);
    for _ in 0..3 {
        assert_eq!(run_fn(&code, &[270, 192], MEM, FUEL), first);
    }
    // Cycle accounting is exact, not approximate: pin the gcd(270,192) count so
    // any cycle-model change is a reviewed diff, not drift.
    assert_eq!(first.1, 160); // 4 × (1 + 34 + 1 + 1 + 2) + taken-beq 2 + ret 2
}

#[test]
fn faults_are_stops_not_panics() {
    // A wild store faults; fuel exhaustion is a stop.
    let ins = [Ins::Store(StoreW::Sw, Reg::X0, Reg::A0, -8), ret()];
    let code = encode(&ins).unwrap();
    assert_eq!(run_fn(&code, &[1], MEM, FUEL).2, rustrv32::Stop::Fault);
    let spin = encode(&[Ins::At(0), Ins::Jal(Reg::X0, 0)]).unwrap();
    assert_eq!(run_fn(&spin, &[], MEM, 100).2, rustrv32::Stop::Fuel);
}

/// Run raw instruction words (decoder arms the codegen never emits).
fn run_raw(words: &[u32], args: &[u32]) -> (u32, rustrv32::Stop) {
    let mut code = Vec::new();
    for w in words {
        code.extend_from_slice(&w.to_le_bytes());
    }
    let (a0, _, stop) = run_fn(&code, args, MEM, FUEL);
    (a0, stop)
}

const RET_W: u32 = 0x0000_8067; // jalr x0, 0(ra)

#[test]
fn decoder_arms_the_codegen_never_emits() {
    use rustrv32::Stop;
    // auipc a0, 0x1 — pc-relative: SRAM_BASE + 0x1000.
    let (v, stop) = run_raw(&[0x0000_1517, RET_W], &[]);
    assert_eq!(stop, Stop::Returned);
    assert_eq!(v, rustrv32::SRAM_BASE + 0x1000);
    // slti / sltiu / ori / xori immediates.
    let (v, _) = run_raw(&[0xFFF5_2513, RET_W], &[(-2i32) as u32]); // slti a0,a0,-1
    assert_eq!(v, 1);
    let (v, _) = run_raw(&[0xFFF5_3513, RET_W], &[5]); // sltiu a0,a0,-1 (unsigned max)
    assert_eq!(v, 1);
    let (v, _) = run_raw(&[0x0F05_6513, RET_W], &[0x01]); // ori a0,a0,0xF0
    assert_eq!(v, 0xF1);
    // R-type sll / srl (register amounts).
    let sll = encode(&[Ins::Op(Alu::Sll, Reg::A0, Reg::A0, Reg::A1), ret()]).unwrap();
    assert_eq!(run_fn(&sll, &[3, 4], MEM, FUEL).0, 48);
    let srl = encode(&[Ins::Op(Alu::Srl, Reg::A0, Reg::A0, Reg::A1), ret()]).unwrap();
    assert_eq!(run_fn(&srl, &[0x8000_0000, 31], MEM, FUEL).0, 1);
    // mulhsu: signed × unsigned high half.
    let mulhsu = encode(&[Ins::Op(Alu::Mulhsu, Reg::A0, Reg::A0, Reg::A1), ret()]).unwrap();
    assert_eq!(
        run_fn(&mulhsu, &[(-1i32) as u32, 2], MEM, FUEL).0,
        u32::MAX // (-1 × 2) >> 32 = -1
    );
    // ecall stops the run (the executor trap surface).
    let (v, stop) = run_raw(&[0x0000_0073, RET_W], &[42]);
    assert_eq!((v, stop), (42, Stop::Ecall));
}

#[test]
fn every_fault_class_stops_cleanly() {
    use rustrv32::Stop;
    // Misaligned pc: jalr clears bit 0, so bit 1 is the reachable misalignment.
    let jalr_odd = encode(&[
        Ins::Lui(Reg::T0, rustrv32::SRAM_BASE >> 12),
        Ins::Jalr(Reg::X0, Reg::T0, 2),
    ])
    .unwrap();
    assert_eq!(run_fn(&jalr_odd, &[], MEM, FUEL).2, Stop::Fault);
    // Fetch out of the window.
    let jalr_oob = encode(&[Ins::Jalr(Reg::X0, Reg::X0, 16)]).unwrap();
    assert_eq!(run_fn(&jalr_oob, &[], MEM, FUEL).2, Stop::Fault);
    // Misaligned data: lh/lw/sh/sw at odd (or 2-mod-4) addresses.
    for ins in [
        Ins::Load(LoadW::Lh, Reg::A0, Reg::Sp, -3),
        Ins::Load(LoadW::Lw, Reg::A0, Reg::Sp, -2),
        Ins::Store(StoreW::Sh, Reg::Sp, Reg::A0, -3),
        Ins::Store(StoreW::Sw, Reg::Sp, Reg::A0, -2),
    ] {
        let code = encode(&[ins, ret()]).unwrap();
        assert_eq!(run_fn(&code, &[1], MEM, FUEL).2, Stop::Fault, "no fault");
    }
    // Illegal encodings: all-zeroes, an unknown LOAD/STORE/BRANCH funct3, an
    // unknown OP funct7, and ebreak (unassigned SYSTEM word).
    for word in [
        0x0000_0000, // opcode 0
        0x0000_3503, // "ld" (f3=011) — RV64-only
        0x0000_3023, // "sd" (f3=011) — RV64-only
        0x0000_2063, // branch f3=010 — unassigned
        0x0A05_0533, // OP funct7=5 — unassigned
        0x0010_0073, // ebreak — unassigned here
    ] {
        let (_, stop) = run_raw(&[word, RET_W], &[]);
        assert_eq!(stop, rustrv32::Stop::Fault, "word {word:#010x}");
    }
}
