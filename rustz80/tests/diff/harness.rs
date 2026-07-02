//! The shared differential harness: a flat RAM bus, the compile→run helpers,
//! and the `check!` macro every module tests through.

/// A flat 64K RAM bus — enough to run a compiled function.
struct Ram {
    mem: Vec<u8>,
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
    fn tick(&mut self, _: u32) {}
}

/// Load `bytes` at `ORG`, `CALL` it from a trampoline that `HALT`s on return,
/// run to the halt, and return `HL`.
pub(crate) fn run(bytes: &[u8]) -> u16 {
    let mut bus = Ram {
        mem: vec![0u8; 0x1_0000],
    };
    let org = rustz80::ORG;
    // trampoline @ 0x7000:  CALL org ; HALT
    bus.mem[0x7000] = 0xCD;
    bus.mem[0x7001] = org as u8;
    bus.mem[0x7002] = (org >> 8) as u8;
    bus.mem[0x7003] = 0x76;
    bus.mem[org as usize..org as usize + bytes.len()].copy_from_slice(bytes);

    let mut cpu = z80::Cpu::new();
    cpu.reset();
    cpu.regs.pc = 0x7000;
    cpu.regs.sp = 0xFFF0;
    for _ in 0..1_000_000 {
        if cpu.halted {
            break;
        }
        cpu.step(&mut bus);
    }
    assert!(cpu.halted, "function did not return");
    cpu.regs.hl()
}

/// Compile + run one block both ways and assert they match.
macro_rules! check {
    ($body:block) => {{
        #[allow(unused_assignments)]
        fn host() -> u16 $body
        let src = format!("fn f() -> u16 {}", stringify!($body));
        let bytes = rustz80::compile_fn(&src).unwrap_or_else(|e| panic!("compile failed: {e}\nsrc: {src}"));
        let got = run(&bytes);
        assert_eq!(got, host(), "rustz80 vs rustc diverged\nsrc: {src}\n  z80={got} host={}", host());
    }};
}

/// Run a multi-function program from its `entry` symbol.
pub(crate) fn run_program(prog: &rustz80::Program, entry: &str) -> u16 {
    let mut bus = Ram {
        mem: vec![0u8; 0x1_0000],
    };
    let target = prog.symbols[entry];
    bus.mem[0x7000] = 0xCD;
    bus.mem[0x7001] = target as u8;
    bus.mem[0x7002] = (target >> 8) as u8;
    bus.mem[0x7003] = 0x76;
    let org = rustz80::ORG as usize;
    bus.mem[org..org + prog.code.len()].copy_from_slice(&prog.code);
    let mut cpu = z80::Cpu::new();
    cpu.reset();
    cpu.regs.pc = 0x7000;
    cpu.regs.sp = 0xFFF0;
    for _ in 0..1_000_000 {
        if cpu.halted {
            break;
        }
        cpu.step(&mut bus);
    }
    assert!(cpu.halted, "program did not return");
    cpu.regs.hl()
}

/// Like [`run_program`] but return the first three result registers `[HL, DE, BC]` —
/// to inspect a tuple return's register layout.
pub(crate) fn run_program_regs(prog: &rustz80::Program, entry: &str) -> [u16; 3] {
    let mut bus = Ram {
        mem: vec![0u8; 0x1_0000],
    };
    let target = prog.symbols[entry];
    bus.mem[0x7000] = 0xCD;
    bus.mem[0x7001] = target as u8;
    bus.mem[0x7002] = (target >> 8) as u8;
    bus.mem[0x7003] = 0x76;
    let org = rustz80::ORG as usize;
    bus.mem[org..org + prog.code.len()].copy_from_slice(&prog.code);
    let mut cpu = z80::Cpu::new();
    cpu.reset();
    cpu.regs.pc = 0x7000;
    cpu.regs.sp = 0xFFF0;
    for _ in 0..1_000_000 {
        if cpu.halted {
            break;
        }
        cpu.step(&mut bus);
    }
    assert!(cpu.halted, "program did not return");
    [cpu.regs.hl(), cpu.regs.de(), cpu.regs.bc()]
}

/// Run a no-result program (entry `run`) on a 64K RAM bus and return the bus.
pub(crate) fn run_to_memory(prog: &rustz80::Program, entry: &str) -> Vec<u8> {
    let mut bus = Ram {
        mem: vec![0u8; 0x1_0000],
    };
    let target = prog.symbols[entry];
    bus.mem[0x7000] = 0xCD;
    bus.mem[0x7001] = target as u8;
    bus.mem[0x7002] = (target >> 8) as u8;
    bus.mem[0x7003] = 0x76;
    let org = rustz80::ORG as usize;
    bus.mem[org..org + prog.code.len()].copy_from_slice(&prog.code);
    let mut cpu = z80::Cpu::new();
    cpu.reset();
    cpu.regs.pc = 0x7000;
    cpu.regs.sp = 0xFFF0;
    for _ in 0..1_000_000 {
        if cpu.halted {
            break;
        }
        cpu.step(&mut bus);
    }
    assert!(cpu.halted, "program did not return");
    bus.mem
}
