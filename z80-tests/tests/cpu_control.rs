//! Control-flow / non-opcode CPU paths the decode sweep doesn't reach:
//! the host-trap carry helpers, the one-instruction EI enable delay, and maskable
//! interrupt servicing (masked / IM 1 / IM 2). These are the bulk of `cpu.rs`'s
//! remaining uncovered lines.

use z80::Cpu;
use z80_tests::FlatBus;

#[test]
fn carry_helpers_round_trip() {
    let mut cpu = Cpu::new();
    assert!(!cpu.regs.carry());
    cpu.regs.set_carry(true);
    assert!(cpu.regs.carry());
    cpu.regs.set_carry(false);
    assert!(!cpu.regs.carry());
}

#[test]
fn ei_sets_iff_immediately_but_delays_interrupt_accept_by_one_instruction() {
    let mut bus = FlatBus::new();
    bus.load(0x0000, &[0xFB, 0x00]); // EI ; NOP
    let mut cpu = Cpu::new();

    cpu.step(&mut bus); // EI
    assert!(
        cpu.iff1 && cpu.iff2,
        "EI sets IFF1/IFF2 immediately (hardware behaviour)"
    );
    assert!(
        !cpu.interrupt(&mut bus),
        "but a maskable interrupt is still inhibited for the one instruction after EI"
    );

    cpu.step(&mut bus); // NOP — the inhibit window closes
    assert!(
        cpu.interrupt(&mut bus),
        "interrupt accepted after the instruction following EI"
    );
}

#[test]
fn interrupt_is_masked_when_disabled() {
    let mut bus = FlatBus::new();
    let mut cpu = Cpu::new();
    cpu.iff1 = false;
    assert!(
        !cpu.interrupt(&mut bus),
        "a masked interrupt is not serviced"
    );
}

#[test]
fn im1_interrupt_vectors_to_0x0038_and_pushes_pc() {
    let mut bus = FlatBus::new();
    let mut cpu = Cpu::new();
    cpu.iff1 = true;
    cpu.im = 1;
    cpu.regs.sp = 0xFFF0;
    cpu.regs.pc = 0x1234;

    assert!(cpu.interrupt(&mut bus));
    assert_eq!(cpu.regs.pc, 0x0038);
    assert!(!cpu.iff1 && !cpu.iff2, "servicing clears both flip-flops");
    // old PC was pushed (little-endian) to the stack.
    let lo = bus.mem[0xFFEE] as u16;
    let hi = bus.mem[0xFFEF] as u16;
    assert_eq!(lo | (hi << 8), 0x1234);
}

#[test]
fn im2_interrupt_vectors_through_the_i_table() {
    let mut bus = FlatBus::new();
    // I=0x00 → vector address 0x00FF; place the handler address 0x4321 there.
    bus.mem[0x00FF] = 0x21;
    bus.mem[0x0100] = 0x43;
    let mut cpu = Cpu::new();
    cpu.iff1 = true;
    cpu.im = 2;
    cpu.regs.i = 0x00;
    cpu.regs.sp = 0xFFF0;
    cpu.regs.pc = 0x1234;

    assert!(cpu.interrupt(&mut bus));
    assert_eq!(cpu.regs.pc, 0x4321, "IM 2 jumps to the table entry");
}

#[test]
fn interrupt_wakes_the_cpu_from_halt() {
    let mut bus = FlatBus::new();
    let mut cpu = Cpu::new();
    cpu.iff1 = true;
    cpu.im = 1;
    cpu.halted = true;
    assert!(cpu.interrupt(&mut bus));
    assert!(!cpu.halted, "an accepted interrupt clears HALT");
}
