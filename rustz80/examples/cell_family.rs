//! One cell, many bodies — the cell-family demo (Phase 5,
//! `docs/13-multi-target-spec.md` §0).
//!
//! A single restricted-Rust source compiles through the shared `cell80-core` IR
//! into independent artifacts, each executed on its own reference machine, all
//! answering identically:
//!
//! - **Z80 / Spectrum48** — authentic bytes on the cycle-exact Z80 core (real
//!   T-states: the certificate target, docs 13 §4);
//! - **RV32 / Hazard3 model** — the rustrv32 backend on the cycle-accounted
//!   RV32IM executor (table provisional until the RP2350 `mcycle` co-sign);
//! - **the reference IR interpreter** — the backend-independent meaning.
//!
//! The second half shows the family widening past backend zero: an `i32` cell
//! runs natively on RV32 and the interpreter, while rustz80 refuses it with the
//! pre-registered gate error (signed-32 lands with a backend that has it).
//!
//! Run: `cargo run -p rustz80 --example cell_family`

use rustz80::{compile_program_for, lower_program_full, PreludeConfig, Target};

/// The u16 cell: Euclid's gcd behind a `run` entry — a multi-`fn` program, so
/// the call convention crosses every backend too.
const GCD: &str = "
    fn gcd(a: u16, b: u16) -> u16 {
        let mut a = a;
        let mut b = b;
        while b != 0 {
            let t = a % b;
            a = b;
            b = t;
        }
        a
    }
    fn run() -> u16 { gcd(270u16, 192u16) * 1000u16 + gcd(1071u16, 462u16) }
";

/// The i32 cell: a deadband + clamp (the robo-dialect shape — signed sensor
/// spans, WS-C) that needs two's-complement ordering end to end.
const DEADBAND: &str = "
    fn deadband(x: i32, lim: i32) -> i32 {
        let mut y = x;
        if y > -100i32 {
            if y < 100i32 { y = 0i32; }
        }
        if y > lim { y = lim; }
        if y < 0i32 - lim { y = 0i32 - lim; }
        y
    }
    fn run() -> u16 {
        let a = deadband(-70000i32, 50000i32);
        let b = deadband(60i32, 50000i32);
        ((a == -50000i32) as u16) * 10u16 + ((b == 0i32) as u16)
    }
";

fn main() {
    println!("== one source, three bodies: gcd (u16) ==");
    let (hl, tstates) = run_z80(GCD, "run");
    println!("  Z80/Spectrum48   result={hl:5}   T-states={tstates} (authentic)");
    let (a0, cycles) = run_rv32(GCD, "run");
    println!("  RV32/Hazard3     result={a0:5}   cycles={cycles} (provisional table)");
    let ir = rustz80::interp_program(GCD, "run").expect("interp");
    println!(
        "  IR interpreter   result={:5}   (the reference meaning)",
        ir[0]
    );
    assert!(hl as u32 == a0 && ir[0] == hl, "the bodies must agree");
    println!("  agreement: all three identical\n");

    println!("== the family widens: deadband (i32) ==");
    let (a0, cycles) = run_rv32(DEADBAND, "run");
    println!("  RV32/Hazard3     result={a0:5}   cycles={cycles} (signed-32 native)");
    let ir = rustz80::interp_program(DEADBAND, "run").expect("interp");
    println!("  IR interpreter   result={:5}", ir[0]);
    assert!(a0 == ir[0] as u32 && a0 == 11, "the i32 bodies must agree");
    let Err(gate) = compile_program_for(DEADBAND, Target::Spectrum48) else {
        panic!("backend zero must refuse signed-32");
    };
    println!("  Z80/backend-zero refuses, as pre-registered:");
    println!("    {gate}");
}

/// Compile for the authentic Spectrum48 target and run on the cycle-exact Z80
/// core: a tiny trampoline `CALL`s the entry and `HALT`s (the harness shape).
fn run_z80(src: &str, entry: &str) -> (u16, u64) {
    struct Ram {
        mem: Vec<u8>,
        tstates: u64,
    }
    impl z80::Bus for Ram {
        fn read(&mut self, a: u16) -> u8 {
            self.mem[a as usize]
        }
        fn write(&mut self, a: u16, v: u8) {
            self.mem[a as usize] = v;
        }
        fn input(&mut self, _: u16) -> u8 {
            0xFF
        }
        fn output(&mut self, _: u16, _: u8) {}
        fn contend(&mut self, _: u16, _: u32) {}
        fn tick(&mut self, t: u32) {
            self.tstates += t as u64;
        }
        fn host_trap(&mut self, _: &mut z80::Regs) -> u32 {
            unreachable!("Spectrum48 images carry no traps")
        }
    }
    let prog = compile_program_for(src, Target::Spectrum48).expect("z80 compile");
    let mut bus = Ram {
        mem: vec![0; 0x1_0000],
        tstates: 0,
    };
    let org = rustz80::ORG as usize;
    bus.mem[org..org + prog.code.len()].copy_from_slice(&prog.code);
    let entry_addr = prog.symbols[entry];
    bus.mem[0x7000] = 0xCD; // CALL entry
    bus.mem[0x7001] = entry_addr as u8;
    bus.mem[0x7002] = (entry_addr >> 8) as u8;
    bus.mem[0x7003] = 0x76; // HALT
    let mut cpu = z80::Cpu::new();
    cpu.reset();
    cpu.regs.pc = 0x7000;
    cpu.regs.sp = 0xFFF0;
    while !cpu.halted {
        cpu.step(&mut bus);
    }
    (cpu.regs.hl(), bus.tstates)
}

/// Lower once, compile with the RV32 backend, run on the cycle-accounted executor.
fn run_rv32(src: &str, entry: &str) -> (u32, u64) {
    let file: syn::File = syn::parse_str(src).expect("parse");
    let lowered = lower_program_full(&file, &PreludeConfig::default()).expect("lower");
    let image = rustrv32::compile(&lowered.funcs, &lowered.const_data()).expect("rv32 compile");
    let (regs, cycles, stop, _) = rustrv32::run_cell(
        &image.code,
        &image.consts,
        image.symbols[entry],
        &[],
        &[],
        10_000_000,
    );
    assert_eq!(stop, rustrv32::Stop::Returned);
    (regs[0], cycles)
}
