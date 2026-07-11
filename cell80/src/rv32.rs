//! The RV32 machine-body runner (WS-E3): the sibling of [`crate::Runner`] behind
//! the same cartridge product surface. One manifest drives either body — the
//! family slot ABI + the shared 64 KiB window map (docs 13 §WS-E2) mean
//! `state_addrs` and entry names carry over unchanged; only the executor differs.
//!
//! The cycle numbers it reports carry the executor's caveat verbatim: the table
//! is provisional until the RP2350 `mcycle` co-sign (B4).

use crate::cartridge::{Cartridge, Rv32Body, RV32_TARGET};

/// One run's outcome on the RV32 executor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rv32Report {
    /// The result register (`a0`; a u16 entry's value is zero-extended).
    pub result: u32,
    /// All three result registers (`a0..a2` — tuple returns).
    pub regs: [u32; 3],
    /// The honest cycle count (provisional Hazard3 table — see the module doc).
    pub cycles: u64,
}

/// A loaded RV32 body: compile once (elsewhere), run many. Mirrors the Z80
/// [`crate::Runner`] boundary: loading a cartridge with a different machine body
/// is a typed refusal naming both sides.
pub struct Rv32Runner {
    body: Rv32Body,
    entry_off: u32,
}

impl Rv32Runner {
    pub fn load(cart: &Cartridge) -> Result<Self, String> {
        if cart.manifest.target != RV32_TARGET {
            return Err(format!(
                "this cartridge carries a `{}` machine body — Rv32Runner hosts \
                 `{RV32_TARGET}` bodies (use `Runner`/`CellHost` for z80-cell)",
                cart.manifest.target
            ));
        }
        let body = cart.rv32()?.clone();
        let entry_off = *body
            .image
            .symbols
            .get(&cart.manifest.entry)
            .ok_or_else(|| format!("no entry `{}` in the rv32 image", cart.manifest.entry))?;
        Ok(Rv32Runner { body, entry_off })
    }

    /// Run the entry: `args` ride `a0..a2` (u16 values zero-extended), `data`
    /// pairs plant into the 64 KiB window before the run (the typed-state I/O
    /// convention at its window addresses), `fuel` bounds instruction count
    /// (the deterministic liveness guard).
    pub fn run(
        &self,
        args: &[u32],
        data: &[(u16, &[u8])],
        fuel: u64,
    ) -> Result<Rv32Report, String> {
        let (regs, cycles, stop, _window) = rustrv32::run_cell(
            &self.body.image.code,
            &self.body.image.consts,
            self.entry_off,
            args,
            data,
            fuel,
        );
        match stop {
            rustrv32::Stop::Returned => Ok(Rv32Report {
                result: regs[0],
                regs,
                cycles,
            }),
            rustrv32::Stop::Ecall => Ok(Rv32Report {
                // `halt(code)` — the code rides a0, the Z80 halt convention.
                result: regs[0],
                regs,
                cycles,
            }),
            rustrv32::Stop::Fuel => Err("rv32: fuel budget exhausted".into()),
            rustrv32::Stop::Fault => Err("rv32: memory/alignment fault".into()),
        }
    }
}
