//! The shared differential harness: a flat RAM bus **with the Cell80 host traps
//! serviced** (so both compile targets run under one oracle), the compile→run helpers,
//! and the `check!` macro every module tests through.
//!
//! Every helper compiles and runs the source **twice** — once per [`rustz80::Target`]:
//! `Spectrum48` exercises the software `__mul16`/`__divmod16`/`__mul32`/`__divmod32`
//! routines, `Cell` exercises the `ED FE` trap path the cell VM actually ships. Both
//! must agree with each other *and* with the rustc oracle.

/// Both compile targets — every differential test runs the full matrix.
pub(crate) const TARGETS: [rustz80::Target; 2] =
    [rustz80::Target::Spectrum48, rustz80::Target::Cell];

/// A flat 64K RAM bus that services the Cell80 `ED FE` traps (mirroring the cell VM's
/// bus): MUL16/DIVMOD16, MUL32/DIVMOD32 (left operand in the two stack words), and
/// FILL16. Spectrum-target images never contain `ED FE`, so servicing is harmless there.
struct Ram {
    mem: Vec<u8>,
}

impl Ram {
    fn rd16(&self, a: u16) -> u16 {
        u16::from_le_bytes([self.mem[a as usize], self.mem[a.wrapping_add(1) as usize]])
    }
    fn wr16(&mut self, a: u16, v: u16) {
        self.mem[a as usize] = v as u8;
        self.mem[a.wrapping_add(1) as usize] = (v >> 8) as u8;
    }
    fn rd32(&self, a: u16) -> u32 {
        self.rd16(a) as u32 | (self.rd16(a.wrapping_add(2)) as u32) << 16
    }
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
    fn host_trap(&mut self, regs: &mut z80::Regs) -> u32 {
        match regs.a {
            0x10 => regs.set_hl(regs.bc().wrapping_mul(regs.de())),
            0x11 => {
                let (bc, de) = (regs.bc(), regs.de());
                match bc.checked_div(de) {
                    Some(q) => {
                        regs.set_hl(q);
                        regs.set_de(bc % de);
                    }
                    None => panic!("diff harness: divide by zero (rustc would panic too)"),
                }
            }
            0x12 => {
                let l = self.rd32(regs.sp);
                let r = regs.hl() as u32 | (regs.de() as u32) << 16;
                let p = l.wrapping_mul(r);
                regs.set_hl(p as u16);
                regs.set_de((p >> 16) as u16);
            }
            0x13 => {
                let l = self.rd32(regs.sp);
                let r = regs.hl() as u32 | (regs.de() as u32) << 16;
                let (q, rem) = match l.checked_div(r) {
                    Some(q) => (q, l % r),
                    None => panic!("diff harness: divide by zero (rustc would panic too)"),
                };
                regs.set_hl(q as u16);
                regs.set_de((q >> 16) as u16);
                let sp = regs.sp;
                self.wr16(sp, rem as u16);
                self.wr16(sp.wrapping_add(2), (rem >> 16) as u16);
            }
            0x20 => {
                let (mut addr, count, val) = (regs.hl(), regs.bc(), regs.de());
                for _ in 0..count {
                    self.wr16(addr, val);
                    addr = addr.wrapping_add(2);
                }
            }
            other => panic!("diff harness: unexpected trap id {other:#04x}"),
        }
        4
    }
}

/// Load `bytes` at `ORG`, `CALL` the trampoline target, run to the halt, and return the
/// final CPU + bus. The core loop shared by every helper below.
fn exec(bytes: &[u8], entry_addr: u16) -> (z80::Cpu, Ram) {
    exec_args(bytes, entry_addr, &[], &[])
}

/// [`exec`] with register arguments and pre-laid data blobs: `args` load into
/// `HL`/`DE`/`BC` (the parameter convention) before the `CALL`; each `(addr, bytes)`
/// in `data` is written to memory first — how `run_str` plants a string buffer.
fn exec_args(
    bytes: &[u8],
    entry_addr: u16,
    args: &[u16],
    data: &[(u16, &[u8])],
) -> (z80::Cpu, Ram) {
    let mut bus = Ram {
        mem: vec![0u8; 0x1_0000],
    };
    for (addr, blob) in data {
        bus.mem[*addr as usize..*addr as usize + blob.len()].copy_from_slice(blob);
    }
    // trampoline @ 0x7000:  [LD HL/DE/BC, arg]* ; CALL entry ; HALT
    const LD: [u8; 3] = [0x21, 0x11, 0x01];
    let mut p = 0x7000usize;
    for (i, &v) in args.iter().enumerate().take(3) {
        bus.mem[p] = LD[i];
        bus.mem[p + 1] = v as u8;
        bus.mem[p + 2] = (v >> 8) as u8;
        p += 3;
    }
    bus.mem[p] = 0xCD;
    bus.mem[p + 1] = entry_addr as u8;
    bus.mem[p + 2] = (entry_addr >> 8) as u8;
    bus.mem[p + 3] = 0x76;
    let org = rustz80::ORG as usize;
    bus.mem[org..org + bytes.len()].copy_from_slice(bytes);

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
    (cpu, bus)
}

/// Run compiled function bytes (entry at `ORG`) and return `HL`.
pub(crate) fn run(bytes: &[u8]) -> u16 {
    exec(bytes, rustz80::ORG).0.regs.hl()
}

/// Where `run_str` plants the input buffer — far above the code + scratch region.
const STR_INPUT: u16 = 0xB000;

/// Run compiled `fn f(s: &str) -> u16` bytes: pack `s` as a length-prefixed buffer
/// (u16 LE length, then the bytes — the Phase S wire format) at [`STR_INPUT`], pass
/// its address in `HL`, and return `HL` after the run.
pub(crate) fn run_str(bytes: &[u8], s: &str) -> u16 {
    let mut buf = Vec::with_capacity(s.len() + 2);
    buf.extend_from_slice(&(s.len() as u16).to_le_bytes());
    buf.extend_from_slice(s.as_bytes());
    exec_args(bytes, rustz80::ORG, &[STR_INPUT], &[(STR_INPUT, &buf)])
        .0
        .regs
        .hl()
}

/// Compile + run one block on **both targets** and assert they match the rustc oracle.
macro_rules! check {
    ($body:block) => {{
        #[allow(unused_assignments)]
        fn host() -> u16 $body
        let src = format!("fn f() -> u16 {}", stringify!($body));
        for target in crate::harness::TARGETS {
            let bytes = rustz80::compile_fn_for(&src, target)
                .unwrap_or_else(|e| panic!("compile failed ({target:?}): {e}\nsrc: {src}"));
            let got = crate::harness::run(&bytes);
            assert_eq!(
                got, host(),
                "rustz80 vs rustc diverged on {target:?}\nsrc: {src}\n  z80={got} host={}",
                host()
            );
        }
    }};
}

/// Compile + run one `fn f(s: &str) -> u16` body on **both targets**, over one or
/// more input strings, asserting each against the rustc oracle. The single-source
/// property of `check!`, for string kernels. The parameter name is passed first
/// (macro hygiene: the body's `s` must be a call-site binding):
/// `check_str!(s, { s.len() as u16 }, "", "hello");`
macro_rules! check_str {
    ($s:ident, $body:block, $($input:expr),+ $(,)?) => {{
        // The dialect's long-form comparisons (`s.len() == 0`, `c >= b'A'`) are the
        // constructs under test — not style to lint away.
        #[allow(unused_assignments, clippy::needless_range_loop, clippy::len_zero)]
        fn host($s: &str) -> u16 $body
        let src = format!("fn f({}: &str) -> u16 {}", stringify!($s), stringify!($body));
        for target in crate::harness::TARGETS {
            let bytes = rustz80::compile_fn_for(&src, target)
                .unwrap_or_else(|e| panic!("compile failed ({target:?}): {e}\nsrc: {src}"));
            $(
                let got = crate::harness::run_str(&bytes, $input);
                assert_eq!(
                    got,
                    host($input),
                    "rustz80 vs rustc diverged on {:?} for input {:?}\nsrc: {}\n  z80={} host={}",
                    target, $input, src, got, host($input)
                );
            )+
        }
    }};
}

/// Run a multi-function program from its `entry` symbol on **both targets**, assert the
/// targets agree, and return the result — the caller then asserts it against the oracle.
pub(crate) fn run_program(src: &str, entry: &str) -> u16 {
    both(src, entry, |cpu, _| cpu.regs.hl())
}

/// Like [`run_program`] but return the three result registers `[HL, DE, BC]` —
/// to inspect a tuple return's register layout.
pub(crate) fn run_program_regs(src: &str, entry: &str) -> [u16; 3] {
    both(src, entry, |cpu, _| {
        [cpu.regs.hl(), cpu.regs.de(), cpu.regs.bc()]
    })
}

/// Run a program for its memory effects and return the bus. Both targets run and the
/// buses must agree everywhere **except the code region** (`ORG..SCRATCH`), which
/// differs by construction (traps vs software routines) and is masked out.
pub(crate) fn run_to_memory(src: &str, entry: &str) -> Vec<u8> {
    both(src, entry, |_, bus| {
        let mut m = bus.mem.clone();
        m[rustz80::ORG as usize..0x9000].fill(0);
        m
    })
}

/// Compile `src` for both targets, run `entry`, extract a value from each run, assert
/// the two targets agree, and return it.
fn both<T: PartialEq + std::fmt::Debug>(
    src: &str,
    entry: &str,
    extract: impl Fn(&z80::Cpu, &Ram) -> T,
) -> T {
    let mut results = crate::harness::TARGETS.iter().map(|&target| {
        let prog = rustz80::compile_program_for(src, target)
            .unwrap_or_else(|e| panic!("compile failed ({target:?}): {e}"));
        let (cpu, bus) = exec(&prog.code, prog.symbols[entry]);
        extract(&cpu, &bus)
    });
    let spectrum = results.next().unwrap();
    let cell = results.next().unwrap();
    assert_eq!(spectrum, cell, "Spectrum48 vs Cell targets diverged");
    spectrum
}
