//! Per-opcode correctness against the **SingleStepTests** suite
//! (<https://github.com/SingleStepTests/z80>): ~1000 cases per opcode, each an initial
//! CPU+RAM state, the expected final state after exactly one instruction, and the
//! cycle-by-cycle bus activity.
//!
//! The data is fetched on demand and git-ignored (it's large). This test **skips cleanly**
//! when it's absent, so a plain `cargo test` is green without it. To run for real:
//!
//! ```text
//! z80-tests/sst/fetch.sh            # download a representative subset into sst/v1/
//! z80-tests/sst/fetch.sh --all      # …or the whole instruction set
//! cargo test -p z80-tests --release single_step -- --nocapture
//! ```
//!
//! Knobs (env): `SST_DIR` overrides the data dir; `SST_MAX_CASES=N` caps cases per opcode;
//! `SST_NO_CYCLES=1` skips the T-state count check (state-only).
//!
//! Field notes (from the suite README): `wz` is MEMPTR; `q` is the flag-modification latch
//! used for the SCF/CCF X/Y quirk; `p` tracks "last instruction was LD A,I/R" (an
//! interrupt-edge-case flag this core doesn't model — irrelevant in a no-interrupt single
//! step, so it isn't compared).

use std::collections::VecDeque;
use std::path::PathBuf;

use serde_json::Value;
use z80::{Bus, Cpu};

/// A flat-RAM bus that also replays the test's port reads and records its port writes,
/// and accumulates T-states (so we can check cycle counts).
struct TestBus {
    mem: Vec<u8>,
    tstates: u64,
    port_in: VecDeque<u8>,
    port_out: Vec<(u16, u8)>,
}

impl TestBus {
    fn new() -> Self {
        Self { mem: vec![0; 0x1_0000], tstates: 0, port_in: VecDeque::new(), port_out: Vec::new() }
    }
}

impl Bus for TestBus {
    fn read(&mut self, addr: u16) -> u8 {
        self.mem[addr as usize]
    }
    fn write(&mut self, addr: u16, val: u8) {
        self.mem[addr as usize] = val;
    }
    fn input(&mut self, _port: u16) -> u8 {
        self.port_in.pop_front().unwrap_or(0xFF)
    }
    fn output(&mut self, port: u16, val: u8) {
        self.port_out.push((port, val));
    }
    fn contend(&mut self, _addr: u16, _cycles: u32) {}
    fn tick(&mut self, cycles: u32) {
        self.tstates += cycles as u64;
    }
}

fn u8f(v: &Value, k: &str) -> u8 {
    v[k].as_u64().unwrap_or_else(|| panic!("missing u8 field {k}")) as u8
}
fn u16f(v: &Value, k: &str) -> u16 {
    v[k].as_u64().unwrap_or_else(|| panic!("missing u16 field {k}")) as u16
}
fn boolf(v: &Value, k: &str) -> bool {
    v[k].as_u64().unwrap_or(0) != 0
}

/// Build the CPU + bus from a test's `initial` block.
fn setup(init: &Value, ports: Option<&Vec<Value>>) -> (Cpu, TestBus) {
    let mut cpu = Cpu::new();
    {
        let r = &mut cpu.regs;
        r.pc = u16f(init, "pc");
        r.sp = u16f(init, "sp");
        r.a = u8f(init, "a");
        r.f = u8f(init, "f");
        r.b = u8f(init, "b");
        r.c = u8f(init, "c");
        r.d = u8f(init, "d");
        r.e = u8f(init, "e");
        r.h = u8f(init, "h");
        r.l = u8f(init, "l");
        r.i = u8f(init, "i");
        r.r = u8f(init, "r");
        r.ix = u16f(init, "ix");
        r.iy = u16f(init, "iy");
        r.wz = u16f(init, "wz");
        let (af, bc, de, hl) =
            (u16f(init, "af_"), u16f(init, "bc_"), u16f(init, "de_"), u16f(init, "hl_"));
        r.a_ = (af >> 8) as u8;
        r.f_ = af as u8;
        r.b_ = (bc >> 8) as u8;
        r.c_ = bc as u8;
        r.d_ = (de >> 8) as u8;
        r.e_ = de as u8;
        r.h_ = (hl >> 8) as u8;
        r.l_ = hl as u8;
    }
    cpu.iff1 = boolf(init, "iff1");
    cpu.iff2 = boolf(init, "iff2");
    cpu.im = u8f(init, "im");
    cpu.ei_pending = boolf(init, "ei");
    // Incoming Q: step() shifts this into q_prev (what SCF/CCF read) at the start.
    cpu.q = u8f(init, "q");

    let mut bus = TestBus::new();
    for cell in init["ram"].as_array().unwrap() {
        bus.mem[cell[0].as_u64().unwrap() as usize] = cell[1].as_u64().unwrap() as u8;
    }
    // Queue the values the test expects IN to read, in order.
    if let Some(ports) = ports {
        for p in ports {
            if p[2].as_str() == Some("r") {
                bus.port_in.push_back(p[1].as_u64().unwrap() as u8);
            }
        }
    }
    (cpu, bus)
}

/// Compare the post-step CPU + bus to the test's `final` block; return a list of diffs.
fn diff(cpu: &Cpu, bus: &TestBus, fin: &Value, ports: Option<&Vec<Value>>) -> Vec<String> {
    let mut d = Vec::new();
    let r = &cpu.regs;
    let mut chk = |name: &str, got: u64, want: u64| {
        if got != want {
            d.push(format!("{name}: got {got:#x}, want {want:#x}"));
        }
    };
    chk("a", r.a as u64, u8f(fin, "a") as u64);
    chk("f", r.f as u64, u8f(fin, "f") as u64);
    chk("b", r.b as u64, u8f(fin, "b") as u64);
    chk("c", r.c as u64, u8f(fin, "c") as u64);
    chk("d", r.d as u64, u8f(fin, "d") as u64);
    chk("e", r.e as u64, u8f(fin, "e") as u64);
    chk("h", r.h as u64, u8f(fin, "h") as u64);
    chk("l", r.l as u64, u8f(fin, "l") as u64);
    chk("i", r.i as u64, u8f(fin, "i") as u64);
    chk("r", r.r as u64, u8f(fin, "r") as u64);
    chk("pc", r.pc as u64, u16f(fin, "pc") as u64);
    chk("sp", r.sp as u64, u16f(fin, "sp") as u64);
    chk("ix", r.ix as u64, u16f(fin, "ix") as u64);
    chk("iy", r.iy as u64, u16f(fin, "iy") as u64);
    chk("wz", r.wz as u64, u16f(fin, "wz") as u64);
    chk("af_", ((r.a_ as u64) << 8) | r.f_ as u64, u16f(fin, "af_") as u64);
    chk("bc_", ((r.b_ as u64) << 8) | r.c_ as u64, u16f(fin, "bc_") as u64);
    chk("de_", ((r.d_ as u64) << 8) | r.e_ as u64, u16f(fin, "de_") as u64);
    chk("hl_", ((r.h_ as u64) << 8) | r.l_ as u64, u16f(fin, "hl_") as u64);
    chk("iff1", cpu.iff1 as u64, boolf(fin, "iff1") as u64);
    chk("iff2", cpu.iff2 as u64, boolf(fin, "iff2") as u64);
    chk("im", cpu.im as u64, u8f(fin, "im") as u64);
    chk("q", cpu.q as u64, u8f(fin, "q") as u64);

    for cell in fin["ram"].as_array().unwrap() {
        let (addr, want) = (cell[0].as_u64().unwrap(), cell[1].as_u64().unwrap());
        let got = bus.mem[addr as usize] as u64;
        if got != want {
            d.push(format!("ram[{addr:#x}]: got {got:#x}, want {want:#x}"));
        }
    }
    // Port writes, in order.
    if let Some(ports) = ports {
        let want_out: Vec<(u16, u8)> = ports
            .iter()
            .filter(|p| p[2].as_str() == Some("w"))
            .map(|p| (p[0].as_u64().unwrap() as u16, p[1].as_u64().unwrap() as u8))
            .collect();
        if bus.port_out != want_out {
            d.push(format!("ports out: got {:x?}, want {want_out:x?}", bus.port_out));
        }
    }
    d
}

fn data_dir() -> Option<PathBuf> {
    if let Ok(d) = std::env::var("SST_DIR") {
        let p = PathBuf::from(d);
        if p.is_dir() {
            return Some(p);
        }
    }
    let def = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sst/v1");
    def.is_dir().then_some(def)
}

#[test]
fn single_step_tests() {
    let Some(dir) = data_dir() else {
        eprintln!(
            "SingleStepTests: no data — skipping. Fetch with `z80-tests/sst/fetch.sh` \
             (or set SST_DIR to a checkout's v1/)."
        );
        return;
    };
    let max_cases = std::env::var("SST_MAX_CASES").ok().and_then(|s| s.parse().ok()).unwrap_or(usize::MAX);
    let check_cycles = std::env::var("SST_NO_CYCLES").is_err();

    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no .json opcode files in {}", dir.display());

    let (mut total, mut failed) = (0usize, 0usize);
    let mut report: Vec<String> = Vec::new();
    let mut by_field: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut by_file: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();

    for f in &files {
        let cases: Value = serde_json::from_slice(&std::fs::read(f).unwrap())
            .unwrap_or_else(|e| panic!("parse {}: {e}", f.display()));
        let fname = f.file_name().unwrap().to_string_lossy();
        for t in cases.as_array().unwrap().iter().take(max_cases) {
            total += 1;
            let ports = t.get("ports").and_then(|p| p.as_array());
            let (mut cpu, mut bus) = setup(&t["initial"], ports);
            cpu.step(&mut bus);

            let mut diffs = diff(&cpu, &bus, &t["final"], ports);
            if check_cycles {
                let want = t["cycles"].as_array().unwrap().len() as u64;
                if bus.tstates != want {
                    diffs.push(format!("cycles: got {}, want {want}", bus.tstates));
                }
            }
            if !diffs.is_empty() {
                failed += 1;
                *by_file.entry(fname.to_string()).or_default() += 1;
                for d in &diffs {
                    let field = d.split(['[', ':']).next().unwrap_or(d).trim().to_string();
                    *by_field.entry(field).or_default() += 1;
                }
                if report.len() < 30 {
                    let name = t["name"].as_str().unwrap_or("?");
                    report.push(format!("  {fname} [{name}]: {}", diffs.join(", ")));
                }
            }
        }
    }

    eprintln!(
        "SingleStepTests: {}/{} cases passed across {} opcode file(s){}",
        total - failed,
        total,
        files.len(),
        if check_cycles { "" } else { " (state only)" }
    );
    if failed > 0 {
        let mut tally: Vec<_> = by_field.iter().collect();
        tally.sort_by(|a, b| b.1.cmp(a.1));
        eprintln!("failing fields (field: case-count):");
        for (field, n) in tally {
            eprintln!("  {field}: {n}");
        }
        let mut byf: Vec<_> = by_file.iter().collect();
        byf.sort_by(|a, b| b.1.cmp(a.1));
        eprintln!("failing opcodes (file: case-count):");
        for (f, n) in byf {
            eprintln!("  {f}: {n}");
        }
    }
    assert!(
        failed == 0,
        "{failed}/{total} SingleStepTests cases failed; first {} shown:\n{}",
        report.len(),
        report.join("\n")
    );
}
