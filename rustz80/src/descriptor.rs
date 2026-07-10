//! The target descriptor (Phase 5 WS-A/A1 — `docs/13-multi-target-spec.md` §2.1):
//! every per-target compilation parameter codegen used to read implicitly, in one
//! place. The rule the multi-target spec pre-registers: **no backend may read a
//! property of another backend — anything two backends need lives here.**
//! `Spectrum48` and `Cell` are the first two instances (same ISA, different
//! arithmetic strategy), proving the mechanism on backend zero before any new ISA
//! exists.

use crate::codegen::Target;

/// How `*`/`/`/`%` and `[v; N]` fills lower. `Software`: the appended micro-runtime
/// (authentic — the output runs anywhere, real ROM included). `HostTrap`: the
/// `ED FE` host trap, serviced natively by the cell bus (a NOP on real hardware,
/// so it never contaminates authentic output).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithStrategy {
    Software,
    HostTrap,
}

/// Per-target compilation parameters. Addresses are the target's native pointer
/// width (u16 while every backend is Z80; widening is WS-A/A2's problem, not A1's).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetDescriptor {
    /// Default code origin ([`crate::ORG`] for both Z80 targets).
    pub org: u16,
    /// Default locals scratch base: slot `i` lives at `scratch + i*2`. Only a
    /// *default* — program entries relocate scratch above the code when it
    /// outgrows the classic window.
    pub scratch: u16,
    /// The ceiling above code + locals for whole-program entries: the Cell VM
    /// lays state structs at `0xB000` (`cell80`'s `STATE_BASE`), so code + locals
    /// must stay below it; the Spectrum whole-program path just needs stack
    /// headroom.
    pub ceiling: u16,
    /// mul/div/fill lowering strategy (see [`ArithStrategy`]).
    pub arith: ArithStrategy,
}

pub const SPECTRUM48: TargetDescriptor = TargetDescriptor {
    org: crate::ORG,
    scratch: 0x9000,
    ceiling: 0xF000,
    arith: ArithStrategy::Software,
};

pub const CELL: TargetDescriptor = TargetDescriptor {
    org: crate::ORG,
    scratch: 0x9000,
    ceiling: 0xB000,
    arith: ArithStrategy::HostTrap,
};

impl Target {
    /// The target's descriptor — codegen reads per-target parameters through
    /// this, never through a `match` on the target itself.
    pub fn descriptor(self) -> &'static TargetDescriptor {
        match self {
            Target::Spectrum48 => &SPECTRUM48,
            Target::Cell => &CELL,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The descriptor values are ABI: `scratch` is where every historical image
    /// keeps its locals, `ceiling` guards `STATE_BASE`, and `arith` decides the
    /// emitted bytes. A change here is a hash-family break — deliberate or wrong.
    #[test]
    fn descriptor_values_are_pinned() {
        let s = Target::Spectrum48.descriptor();
        assert_eq!(
            (s.org, s.scratch, s.ceiling, s.arith),
            (0x8000, 0x9000, 0xF000, ArithStrategy::Software)
        );
        let c = Target::Cell.descriptor();
        assert_eq!(
            (c.org, c.scratch, c.ceiling, c.arith),
            (0x8000, 0x9000, 0xB000, ArithStrategy::HostTrap)
        );
    }
}
