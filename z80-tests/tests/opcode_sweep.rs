//! Decoder coverage sweep.
//!
//! Drive **every** base opcode and every CB / ED / DD / FD / DDCB / FDCB sub-opcode
//! through `step()` once on a flat 64K bus, and assert only that it neither panics nor
//! corrupts the harness. This is a *smoke* over the decode match arms — there are no
//! result assertions here (correctness is the targeted tests in `lib.rs` and, later, the
//! SingleStepTests/ZEX milestone). Its job is to prove every decode arm is reachable and
//! safe, which the hand-written tests don't do exhaustively.

use z80::Cpu;
use z80_tests::FlatBus;

/// Load `bytes` at 0x0000 and execute exactly one instruction from a known register
/// state where the memory/index paths land in valid RAM (HL/IX/IY → 0x4000, SP high).
fn step_once(bytes: &[u8]) {
    let mut bus = FlatBus::new();
    bus.load(0x0000, bytes);
    let mut cpu = Cpu::new();
    cpu.regs.sp = 0xfff0;
    cpu.regs.h = 0x40; // HL = 0x4000
    cpu.regs.l = 0x00;
    cpu.regs.ix = 0x4000;
    cpu.regs.iy = 0x4000;
    // The check is simply that this returns: `step()` must decode and execute the
    // instruction without panicking or hanging. (PC isn't asserted — RET/JP (HL) off a
    // zeroed stack legitimately land at 0x0000.)
    cpu.step(&mut bus);
}

#[test]
fn base_opcodes_all_decode_and_step() {
    for op in 0u16..=0xFF {
        // Trailing operand bytes cover immediate / displacement forms.
        step_once(&[op as u8, 0x12, 0x34, 0x56]);
    }
}

#[test]
fn cb_prefixed_all_decode_and_step() {
    for sub in 0u16..=0xFF {
        step_once(&[0xCB, sub as u8]);
    }
}

#[test]
fn ed_prefixed_all_decode_and_step() {
    for sub in 0u16..=0xFF {
        step_once(&[0xED, sub as u8, 0x12, 0x34]);
    }
}

#[test]
fn ix_iy_prefixed_all_decode_and_step() {
    for &prefix in &[0xDDu8, 0xFD] {
        for sub in 0u16..=0xFF {
            if sub == 0xCB {
                // DDCB / FDCB: prefix, CB, displacement, op.
                for cbop in 0u16..=0xFF {
                    step_once(&[prefix, 0xCB, 0x02, cbop as u8]);
                }
            } else {
                step_once(&[prefix, sub as u8, 0x12, 0x34]);
            }
        }
    }
}
