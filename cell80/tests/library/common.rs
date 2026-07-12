//! Shared host-oracle helpers for every pack file under `cell80/tests/library/` — split
//! out of the former monolithic `cell80/tests/library.rs` (2026-07-07) so each pack's
//! tests don't have to redeclare them.

use cell80::{Runner, DEFAULT_CYCLES};
use std::path::PathBuf;

/// Read a library cell's source by id (`cells/<pack>/<id>.rs`, searched recursively —
/// cells live in pack subdirectories).
pub fn cell_src(id: &str) -> String {
    let cells_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cells");
    let p = cell80::find_cell_file(&cells_dir, id).unwrap_or_else(|e| panic!("{e}"));
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

/// Compile + run a cell on `args`, returning its `HL` result.
pub fn run_cell(id: &str, args: &[u16]) -> u16 {
    let mut r = Runner::compile(&cell_src(id)).unwrap_or_else(|e| panic!("compile {id}: {e}"));
    r.run(None, args, DEFAULT_CYCLES)
        .unwrap_or_else(|e| panic!("run {id}: {e}"))
        .result
}

/// A banked-compile counterpart to `StateCell::bind`, needed for cells that compose
/// **two or more distinct** heavy F2 kernel bodies (the trig pack's `tan_f32`/`cot_f32`:
/// fsin + fcos; `asin_f32`/`acos_f32`: fsqrt + fatan2). `StateCell::bind` always goes
/// through `Runner::compile`, which is hardcoded to the *unbanked* `CellProgram::compile`
/// (inlines every kernel the entry reaches, no `//! kernel_bank` header support) -- fine
/// for a single kernel (measured directly: a lone fsin body is 8112/8192 bytes even under
/// the sandboxed cap, per `sin_f32`'s own authoring notes) but two-or-more distinct kernel
/// bodies together don't just miss the sandboxed cap, they overrun the *physical*
/// code+locals ceiling before the state region at `STATE_BASE` (0xB000) -- a hard
/// architecture wall, not a configurable one, discovered by actually attempting the bind
/// (the standing "verify by attempting a bind, don't assume either way" rule). Every cell
/// that needs this already declares `kernel_bank: on` and compiles clean through the real
/// sandboxed cartridge path (`cell80 index --json`, which *does* honor the header) --
/// this helper exists only to exercise them the same way for the host-oracle harness,
/// which has no cartridge/header parsing of its own. (`excel_xirr` hits the identical
/// two-kernel-body wall — fln + fexp — but its own test in `excel-financial.rs` already
/// had a fitting precedent, `run_fin`/`run_fin_budget` over `CellHost::run_state_values`
/// with `kernel_bank: true`, so it uses that instead of this helper.)
pub struct BankedCell {
    runner: cell80::Runner,
    fields: std::collections::HashMap<String, (u16, cell80::Ty)>,
    entry: String,
    pending: Vec<(u16, cell80::Ty, u64)>,
}

impl BankedCell {
    pub fn bind(src: &str, entry: &str) -> Self {
        let prog =
            cell80::CellProgram::compile_with_config_banked(src, cell80::CellConfig::permissive())
                .unwrap_or_else(|e| panic!("banked-bind {entry}: {e}"));
        let runner = cell80::Runner::new(&prog);
        let fields = cell80::state_field_addrs(src, entry)
            .unwrap_or_else(|e| panic!("state_field_addrs {entry}: {e}"))
            .into_iter()
            .map(|(name, addr, ty)| (name, (addr, ty)))
            .collect();
        BankedCell {
            runner,
            fields,
            entry: entry.to_string(),
            pending: Vec::new(),
        }
    }

    pub fn set(&mut self, name: &str, value: u64) {
        let &(addr, ty) = self
            .fields
            .get(name)
            .unwrap_or_else(|| panic!("no field `{name}`"));
        self.pending.push((addr, ty, value));
    }

    pub fn run(&mut self, budget: u64) -> cell80::Report {
        let pending = std::mem::take(&mut self.pending);
        self.runner
            .run_with_inputs(Some(&self.entry), &[cell80::STATE_BASE], &pending, budget)
            .unwrap_or_else(|e| panic!("run {}: {e}", self.entry))
    }

    pub fn get(&self, name: &str) -> u64 {
        let &(addr, ty) = self
            .fields
            .get(name)
            .unwrap_or_else(|| panic!("no field `{name}`"));
        match ty {
            cell80::Ty::U8 => self.runner.peek_u8(addr) as u64,
            cell80::Ty::U16 => self.runner.peek_u16(addr) as u64,
            cell80::Ty::U32 | cell80::Ty::F32 => self.runner.peek_u32(addr) as u64,
            cell80::Ty::Bytes(_) | cell80::Ty::Str(_) | cell80::Ty::Array(..) => {
                panic!("field `{name}` ({ty}) has no scalar peek")
            }
        }
    }
}
