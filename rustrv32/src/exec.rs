//! The RV32IM reference executor (WS-B/B2): fetch–decode–execute over a flat
//! memory image, cycle-accounted. Semantics follow the RISC-V unprivileged spec
//! exactly — division by zero returns all-ones (quotient) / the dividend
//! (remainder) with **no trap**, `MIN / -1` wraps, shifts take the low five bits,
//! `x0` reads zero and swallows writes.
//!
//! **The cycle table is provisional** (`CYCLES_*` below): Hazard3-shaped
//! (single-cycle ALU/loads/stores on zero-wait SRAM, a taken-branch/jump fetch
//! bubble), but the numbers are placeholders until the B4 `mcycle` co-sign on
//! RP2350 silicon qualifies them — the spec treats any divergence there as a
//! filed finding, not a shrug. Bounded cells forbid `div` (data-dependent timing,
//! docs 13 §2.2.5); the executor charges its documented worst case so a certified
//! bound is never optimistic.

/// Provisional per-class cycle costs (see the module doc — placeholders until the
/// B4 silicon co-sign).
const CYCLES_ALU: u64 = 1;
const CYCLES_MEM: u64 = 1;
const CYCLES_BRANCH_NOT_TAKEN: u64 = 1;
const CYCLES_BRANCH_TAKEN: u64 = 2;
const CYCLES_JUMP: u64 = 2;
const CYCLES_MUL: u64 = 1;
/// Hazard3's iterative divider is data-dependent; the executor charges the
/// documented worst case so certified bounds are conservative.
const CYCLES_DIV: u64 = 34;

/// Why a run stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    /// `pc` reached the sentinel return address — the entry returned.
    Returned,
    /// An `ecall` executed (the executor trap surface; `a7` selects, by
    /// convention — unassigned ids stop the run).
    Ecall,
    /// The fuel budget ran out (the deterministic liveness guard).
    Fuel,
    /// A fetch/load/store left the memory image, or a fetch was misaligned.
    Fault,
}

/// The executor: registers, `pc`, a flat memory window at [`Rv32::base`], and the
/// cycle accumulator. Deterministic by construction — no host state reaches it.
pub struct Rv32 {
    pub regs: [u32; 32],
    pub pc: u32,
    /// The memory window's base address (RP2350 SRAM by convention).
    pub base: u32,
    pub mem: Vec<u8>,
    pub cycles: u64,
}

/// The RP2350 SRAM base — where images load by convention.
pub const SRAM_BASE: u32 = 0x2000_0000;

/// The sentinel return address: an entry called with `ra = RETURN_SENTINEL`
/// stops the run cleanly when it returns (the harness convention — like the
/// Z80 trampoline's `HALT`).
pub const RETURN_SENTINEL: u32 = 0xFFFF_FFF0;

impl Rv32 {
    /// A fresh executor with `size` bytes of zeroed memory at [`SRAM_BASE`].
    pub fn new(size: usize) -> Self {
        Rv32 {
            regs: [0; 32],
            pc: SRAM_BASE,
            base: SRAM_BASE,
            mem: vec![0; size],
            cycles: 0,
        }
    }

    /// Lay `bytes` at absolute address `at`.
    pub fn load(&mut self, at: u32, bytes: &[u8]) {
        let off = (at - self.base) as usize;
        self.mem[off..off + bytes.len()].copy_from_slice(bytes);
    }

    fn rd_reg(&self, r: u32) -> u32 {
        self.regs[r as usize]
    }
    fn wr_reg(&mut self, r: u32, v: u32) {
        if r != 0 {
            self.regs[r as usize] = v;
        }
    }

    fn mem_off(&self, addr: u32, len: u32) -> Option<usize> {
        let off = addr.wrapping_sub(self.base) as usize;
        (off + len as usize <= self.mem.len()).then_some(off)
    }

    fn rd_mem(&self, addr: u32, len: u32) -> Option<u32> {
        let off = self.mem_off(addr, len)?;
        let mut v = 0u32;
        for i in (0..len as usize).rev() {
            v = (v << 8) | self.mem[off + i] as u32;
        }
        Some(v)
    }

    fn wr_mem(&mut self, addr: u32, len: u32, v: u32) -> bool {
        let Some(off) = self.mem_off(addr, len) else {
            return false;
        };
        for i in 0..len as usize {
            self.mem[off + i] = (v >> (8 * i)) as u8;
        }
        true
    }

    /// Execute one instruction. `None` = still running; `Some` = stopped.
    pub fn step(&mut self) -> Option<Stop> {
        if self.pc == RETURN_SENTINEL {
            return Some(Stop::Returned);
        }
        if self.pc & 3 != 0 {
            return Some(Stop::Fault);
        }
        let Some(word) = self.rd_mem(self.pc, 4) else {
            return Some(Stop::Fault);
        };
        let opcode = word & 0x7F;
        let rd = (word >> 7) & 0x1F;
        let f3 = (word >> 12) & 0x7;
        let rs1 = (word >> 15) & 0x1F;
        let rs2 = (word >> 20) & 0x1F;
        let f7 = word >> 25;
        let imm_i = (word as i32) >> 20; // sign-extended I-immediate
        let next = self.pc.wrapping_add(4);

        match opcode {
            // LUI / AUIPC
            0b0110111 => {
                self.wr_reg(rd, word & 0xFFFF_F000);
                self.cycles += CYCLES_ALU;
            }
            0b0010111 => {
                self.wr_reg(rd, self.pc.wrapping_add(word & 0xFFFF_F000));
                self.cycles += CYCLES_ALU;
            }
            // OP-IMM
            0b0010011 => {
                let a = self.rd_reg(rs1);
                let v = match f3 {
                    0b000 => a.wrapping_add(imm_i as u32),
                    0b010 => ((a as i32) < imm_i) as u32,
                    0b011 => (a < imm_i as u32) as u32,
                    0b100 => a ^ imm_i as u32,
                    0b110 => a | imm_i as u32,
                    0b111 => a & imm_i as u32,
                    0b001 => a << (rs2 & 0x1F),
                    0b101 if f7 == 0b0100000 => ((a as i32) >> (rs2 & 0x1F)) as u32,
                    0b101 => a >> (rs2 & 0x1F),
                    _ => unreachable!(),
                };
                self.wr_reg(rd, v);
                self.cycles += CYCLES_ALU;
            }
            // OP (R-type, incl. M)
            0b0110011 => {
                let (a, b) = (self.rd_reg(rs1), self.rd_reg(rs2));
                let (v, cost) = match (f7, f3) {
                    (0, 0b000) => (a.wrapping_add(b), CYCLES_ALU),
                    (0b0100000, 0b000) => (a.wrapping_sub(b), CYCLES_ALU),
                    (0, 0b001) => (a << (b & 0x1F), CYCLES_ALU),
                    (0, 0b010) => ((((a as i32) < (b as i32)) as u32), CYCLES_ALU),
                    (0, 0b011) => (((a < b) as u32), CYCLES_ALU),
                    (0, 0b100) => (a ^ b, CYCLES_ALU),
                    (0, 0b101) => (a >> (b & 0x1F), CYCLES_ALU),
                    (0b0100000, 0b101) => (((a as i32) >> (b & 0x1F)) as u32, CYCLES_ALU),
                    (0, 0b110) => (a | b, CYCLES_ALU),
                    (0, 0b111) => (a & b, CYCLES_ALU),
                    // M extension — spec semantics exactly (div-by-zero: all-ones /
                    // dividend, no trap; MIN/-1 wraps).
                    (1, 0b000) => (a.wrapping_mul(b), CYCLES_MUL),
                    (1, 0b001) => (
                        ((a as i32 as i64).wrapping_mul(b as i32 as i64) >> 32) as u32,
                        CYCLES_MUL,
                    ),
                    (1, 0b010) => (
                        ((a as i32 as i64).wrapping_mul(b as u64 as i64) >> 32) as u32,
                        CYCLES_MUL,
                    ),
                    (1, 0b011) => ((((a as u64) * (b as u64)) >> 32) as u32, CYCLES_MUL),
                    (1, 0b100) => (
                        if b == 0 {
                            u32::MAX
                        } else {
                            (a as i32).wrapping_div(b as i32) as u32
                        },
                        CYCLES_DIV,
                    ),
                    (1, 0b101) => (a.checked_div(b).unwrap_or(u32::MAX), CYCLES_DIV),
                    (1, 0b110) => (
                        if b == 0 {
                            a
                        } else {
                            (a as i32).wrapping_rem(b as i32) as u32
                        },
                        CYCLES_DIV,
                    ),
                    (1, 0b111) => (a.checked_rem(b).unwrap_or(a), CYCLES_DIV),
                    _ => return Some(Stop::Fault),
                };
                self.wr_reg(rd, v);
                self.cycles += cost;
            }
            // LOAD
            0b0000011 => {
                let addr = self.rd_reg(rs1).wrapping_add(imm_i as u32);
                let v = match f3 {
                    0b000 => self.rd_mem(addr, 1).map(|v| v as u8 as i8 as i32 as u32),
                    0b001 => self.rd_mem(addr, 2).map(|v| v as u16 as i16 as i32 as u32),
                    0b010 => self.rd_mem(addr, 4),
                    0b100 => self.rd_mem(addr, 1),
                    0b101 => self.rd_mem(addr, 2),
                    _ => return Some(Stop::Fault),
                };
                let Some(v) = v else {
                    return Some(Stop::Fault);
                };
                self.wr_reg(rd, v);
                self.cycles += CYCLES_MEM;
            }
            // STORE
            0b0100011 => {
                let imm = ((word >> 25) << 5 | rd) as i32;
                let imm = (imm << 20) >> 20; // sign-extend 12 bits
                let addr = self.rd_reg(rs1).wrapping_add(imm as u32);
                let len = match f3 {
                    0b000 => 1,
                    0b001 => 2,
                    0b010 => 4,
                    _ => return Some(Stop::Fault),
                };
                if !self.wr_mem(addr, len, self.rd_reg(rs2)) {
                    return Some(Stop::Fault);
                }
                self.cycles += CYCLES_MEM;
            }
            // BRANCH
            0b1100011 => {
                let (a, b) = (self.rd_reg(rs1), self.rd_reg(rs2));
                let taken = match f3 {
                    0b000 => a == b,
                    0b001 => a != b,
                    0b100 => (a as i32) < (b as i32),
                    0b101 => (a as i32) >= (b as i32),
                    0b110 => a < b,
                    0b111 => a >= b,
                    _ => return Some(Stop::Fault),
                };
                if taken {
                    let d = (((word >> 31) & 1) << 12)
                        | (((word >> 7) & 1) << 11)
                        | (((word >> 25) & 0x3F) << 5)
                        | (((word >> 8) & 0xF) << 1);
                    let d = ((d as i32) << 19) >> 19; // sign-extend 13 bits
                    self.pc = self.pc.wrapping_add(d as u32);
                    self.cycles += CYCLES_BRANCH_TAKEN;
                    return None;
                }
                self.cycles += CYCLES_BRANCH_NOT_TAKEN;
            }
            // JAL
            0b1101111 => {
                let d = (((word >> 31) & 1) << 20)
                    | (((word >> 12) & 0xFF) << 12)
                    | (((word >> 20) & 1) << 11)
                    | (((word >> 21) & 0x3FF) << 1);
                let d = ((d as i32) << 11) >> 11; // sign-extend 21 bits
                self.wr_reg(rd, next);
                self.pc = self.pc.wrapping_add(d as u32);
                self.cycles += CYCLES_JUMP;
                return None;
            }
            // JALR
            0b1100111 => {
                let target = self.rd_reg(rs1).wrapping_add(imm_i as u32) & !1;
                self.wr_reg(rd, next);
                self.pc = target;
                self.cycles += CYCLES_JUMP;
                return None;
            }
            // SYSTEM: ecall
            0b1110011 if word == 0x0000_0073 => {
                self.cycles += CYCLES_ALU;
                self.pc = next;
                return Some(Stop::Ecall);
            }
            _ => return Some(Stop::Fault),
        }
        self.pc = next;
        None
    }

    /// Run until a stop, bounded by `fuel` instructions (the liveness guard).
    pub fn run(&mut self, fuel: u64) -> Stop {
        for _ in 0..fuel {
            if let Some(stop) = self.step() {
                return stop;
            }
        }
        Stop::Fuel
    }
}

/// Load `code` at [`SRAM_BASE`], call it as a function — `a0..a2` = `args`,
/// `sp` at the top of memory, `ra` = the sentinel — and run to a stop. Returns
/// `(a0, cycles, stop)`: the result register, the honest cycle count, and why
/// the run ended. The B3 harness leg drives compiled cells through this.
pub fn run_fn(code: &[u8], args: &[u32], mem_size: usize, fuel: u64) -> (u32, u64, Stop) {
    let mut cpu = Rv32::new(mem_size);
    cpu.load(SRAM_BASE, code);
    for (i, &v) in args.iter().enumerate().take(3) {
        cpu.regs[10 + i] = v; // a0..a2
    }
    cpu.regs[1] = RETURN_SENTINEL; // ra
    cpu.regs[2] = SRAM_BASE + mem_size as u32; // sp (grows down)
    let stop = cpu.run(fuel);
    (cpu.regs[10], cpu.cycles, stop)
}
