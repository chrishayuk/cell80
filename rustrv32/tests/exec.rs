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
