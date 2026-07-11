//! The runnable machine — `Runner`, its `CellBus`, the exec core, and `CellPool`.
use super::report::{coalesce, sorted_symbols};
use super::*;
use rustz80::{Program, ORG};

/// The bus the CPU steps against — borrows the [`Runner`]'s reusable buffers, counts
/// T-states, and records each *distinct* written address (for an O(touched) reset and
/// the report).
struct CellBus<'a> {
    mem: &'a mut [u8],
    seen: &'a mut [bool],
    touched: &'a mut Vec<u16>,
    cycles: u64,
    halt: Option<u16>, // set by the HALT trap (`halt(code)`)
    trapped_ops: u64,  // count of cost-bearing ED FE traps (mul/div/fill) this run
    div0_halts: bool,  // the `CellConfig::div_by_zero` policy (halt vs saturate)
    div0: bool,        // a divide trap saw a zero divisor under the halt policy
}

impl CellBus<'_> {
    /// Write a byte and record it as touched (so the next run resets it) — shared by the
    /// CPU's `write` and the fill traps.
    fn touch_write(&mut self, a: u16, v: u8) {
        self.mem[a as usize] = v;
        if !self.seen[a as usize] {
            self.seen[a as usize] = true;
            self.touched.push(a);
        }
    }

    /// Read the little-endian `u32` a 32-bit trap's left operand occupies at `sp`
    /// (two pushed words, low word on top).
    fn read32_stack(&self, sp: u16) -> u32 {
        let b = |i: u16| self.mem[sp.wrapping_add(i) as usize] as u32;
        b(0) | b(1) << 8 | b(2) << 16 | b(3) << 24
    }
}

impl z80::Bus for CellBus<'_> {
    fn read(&mut self, a: u16) -> u8 {
        self.mem[a as usize]
    }
    fn write(&mut self, a: u16, v: u8) {
        self.touch_write(a, v);
    }
    fn input(&mut self, _: u16) -> u8 {
        0xFF
    }
    fn output(&mut self, _: u16, _: u8) {}
    fn contend(&mut self, _: u16, _: u32) {}
    fn tick(&mut self, c: u32) {
        self.cycles += c as u64; // the single source of truth for elapsed time
    }
    /// Cell80 host intrinsics (`ED FE`, id in `A`). Matches `spectrum::host::math_traps`:
    /// `0x10` MUL16 (`HL = BC*DE`), `0x11` DIVMOD16 (`HL = BC/DE`, `DE = BC%DE`),
    /// and the 32-bit pair — `0x12` MUL32 / `0x13` DIVMOD32: the left operand in the
    /// two stack words (low word on top), the right in `HL:DE` (low:high), result in
    /// `HL:DE`; DIVMOD32 writes the remainder back into the stack words. Done
    /// host-native, so a `var*var` multiply/divide costs a few T-states instead of a
    /// software loop.
    fn host_trap(&mut self, regs: &mut z80::Regs) -> u32 {
        match regs.a {
            0x10 => {
                let p = regs.bc().wrapping_mul(regs.de());
                regs.set_hl(p);
                self.trapped_ops += 1;
            }
            0x11 => {
                let (bc, de) = (regs.bc(), regs.de());
                match bc.checked_div(de) {
                    Some(q) => {
                        regs.set_hl(q);
                        regs.set_de(bc % de);
                    }
                    // Divide by zero: halt the run (default — a garbage quotient must not
                    // flow onward), or saturate under the legacy opt-in policy.
                    None if self.div0_halts => self.div0 = true,
                    None => regs.set_hl(0xFFFF),
                }
                self.trapped_ops += 1;
            }
            0x12 => {
                // MUL32: HL:DE = l * r (mod 2^32).
                let l = self.read32_stack(regs.sp);
                let r = regs.hl() as u32 | (regs.de() as u32) << 16;
                let p = l.wrapping_mul(r);
                regs.set_hl(p as u16);
                regs.set_de((p >> 16) as u16);
                self.trapped_ops += 1;
            }
            0x13 => {
                // DIVMOD32: quotient → HL:DE, remainder → back into the stack words.
                let l = self.read32_stack(regs.sp);
                let r = regs.hl() as u32 | (regs.de() as u32) << 16;
                let (q, rem) = match l.checked_div(r) {
                    Some(q) => (q, l % r),
                    // Divide by zero: halt (default), or saturate like the Spectrum
                    // software sibling (q = 0xFFFF_FFFF, rem = l) under the opt-in.
                    None if self.div0_halts => {
                        self.div0 = true;
                        self.trapped_ops += 1;
                        return 4;
                    }
                    None => (u32::MAX, l),
                };
                regs.set_hl(q as u16);
                regs.set_de((q >> 16) as u16);
                self.touch_write(regs.sp, rem as u8);
                self.touch_write(regs.sp.wrapping_add(1), (rem >> 8) as u8);
                self.touch_write(regs.sp.wrapping_add(2), (rem >> 16) as u8);
                self.touch_write(regs.sp.wrapping_add(3), (rem >> 24) as u8);
                self.trapped_ops += 1;
            }
            0x20 => {
                // FILL16: BC slots (2-byte words) of DE at HL — array `[v; N]` init.
                let (mut addr, count, val) = (regs.hl(), regs.bc(), regs.de());
                for _ in 0..count {
                    self.touch_write(addr, val as u8);
                    self.touch_write(addr.wrapping_add(1), (val >> 8) as u8);
                    addr = addr.wrapping_add(2);
                }
                self.trapped_ops += 1;
            }
            0x30 => self.halt = Some(regs.hl()), // HALT: stop with status code HL (not a cost trap)
            _ => {}
        }
        4 // a fast hardware op (cell cycle accounting) — see the `trapped_ops` caveat
    }
}

/// The value-cell memo table: `(entry, args) → (outcome, imported?)`.
type ValueCache = std::collections::HashMap<(u16, Vec<u16>), (Fast, bool)>;
/// The state-cell memo table: `(entry, sorted input triples, read set) →
/// (outcome, named fields, imported?)`.
type StateCache = std::collections::HashMap<
    (u16, Vec<(u16, u8, u64)>, Vec<(u16, u8)>),
    (Fast, Vec<(String, u64)>, bool),
>;

/// A compiled cell, runnable many times. One 64 KiB bus is allocated up front and the
/// code loaded once; each [`run`](Runner::run) resets only the previous run's writes,
/// re-lays the argument trampoline, and steps — so reuse pays for the computation, not a
/// fresh 128 KiB alloc/zero.
pub struct Runner {
    prog: Program,
    cfg: CellConfig,
    mem: Vec<u8>,
    seen: Vec<bool>,   // was this address written this run? (dedup for `touched`)
    touched: Vec<u16>, // distinct addresses written by the last run
    /// The opt-in memoization cache (roadmap 3.3): every run starts from reset memory and
    /// the substrate is deterministic, so `(entry, args)` **fully determines** the outcome
    /// — a repeated scoring/verification call is a hash lookup, not a run. `None` until
    /// [`enable_cache`](Runner::enable_cache). Serves [`run_fast`](Runner::run_fast) only:
    /// the rich [`run`](Runner::run) path reports post-run memory (`touched`, `reads`),
    /// which a memoized result cannot faithfully reproduce.
    cache: Option<ValueCache>,
    /// The state-cell sibling of [`cache`](Runner::cache) (docs/12 §2 — the scoring
    /// workhorses are state cells): keyed by `(entry, sorted input triples, read
    /// set)`, storing the named post-run fields alongside the [`Fast`]. The read
    /// set is part of the key because a fact is only well-defined given what was
    /// read back. Enabled together with `cache`.
    state_cache: Option<StateCache>,
    cache_hits: u64,
    cache_lookups: u64,
    /// Hits served from **imported** facts (vs locally computed) — the provenance
    /// split the Act-3 screen wants (docs/12 §2). The `bool` on each cache value is
    /// the entry's origin.
    cache_hits_imported: u64,
    /// The content address facts are keyed by (docs/12 §2): a cartridge-backed load
    /// stamps the v5 artifact hash via [`set_artifact_hash`](Runner::set_artifact_hash);
    /// a bare program self-hashes its serialized image (config included — the image
    /// bytes carry the whole policy) at construction.
    artifact_hash: [u8; 32],
}

impl Runner {
    /// Instantiate a runnable machine from an already-[`compile`](CellProgram::compile)d
    /// program — **cheap**: allocate the bus and load the code, *no parse/compile*. The
    /// way to skip cold setup for a cached snippet (compile once → `Runner::new` many).
    pub fn new(program: &CellProgram) -> Self {
        let mut mem = vec![0u8; 0x1_0000];
        let org = ORG as usize;
        mem[org..org + program.prog.code.len()].copy_from_slice(&program.prog.code);
        if program.uses_kernel_bank() {
            // The resident kernel bank, placed like the code itself — outside the
            // touch-tracking bus, so it survives the per-run reset (sandboxed cells
            // can't write it: no raw memory, and stores go to scratch/state).
            let bank = rustz80::kernel_bank();
            let b = rustz80::BANK_ORG as usize;
            mem[b..b + bank.code.len()].copy_from_slice(&bank.code);
        }
        Runner {
            prog: program.prog.clone(),
            cfg: program.cfg.clone(),
            mem,
            seen: vec![false; 0x1_0000],
            touched: Vec::new(),
            cache: None,
            state_cache: None,
            cache_hits: 0,
            cache_lookups: 0,
            cache_hits_imported: 0,
            artifact_hash: self_hash(program),
        }
    }

    /// Opt in to memoization (see the field doc on [`Runner::cache`]): subsequent
    /// [`run_fast`](Runner::run_fast) calls consult a `(entry, args) → Fast` map before
    /// executing (and [`run_state_fast`](Runner::run_state_fast) its state-cell
    /// sibling). Only **budget-independent** outcomes are stored (a clean return, an
    /// explicit `halt(code)`, a div-by-zero — never a budget/memory-limit stop), and a
    /// hit requires the stored run to have fit strictly inside the caller's budget, so a
    /// cached answer is byte-for-byte what the run would have produced.
    pub fn enable_cache(&mut self) {
        if self.cache.is_none() {
            self.cache = Some(std::collections::HashMap::new());
        }
        if self.state_cache.is_none() {
            self.state_cache = Some(std::collections::HashMap::new());
        }
    }

    /// The content address this runner's facts are keyed by — the cartridge's v5
    /// artifact hash when loaded from one ([`set_artifact_hash`]), else the bare
    /// image's self-hash. Same hash ⇒ same machine ⇒ same facts, forever.
    pub fn artifact_hash(&self) -> [u8; 32] {
        self.artifact_hash
    }

    /// Stamp the v5 artifact hash on a cartridge-backed runner, so cached facts key
    /// on the shareable content address rather than the bare image self-hash.
    pub fn set_artifact_hash(&mut self, hash: [u8; 32]) {
        self.artifact_hash = hash;
    }

    /// `(hits, lookups)` since the cache was enabled — `None` if it never was. The
    /// hit-rate counter the memoization economics are measured by.
    pub fn cache_stats(&self) -> Option<(u64, u64)> {
        self.cache
            .as_ref()
            .map(|_| (self.cache_hits, self.cache_lookups))
    }

    /// The provenance split of the hits: `(locally computed, served from imported
    /// facts)` — the Act-3 screen's number (docs/12 §2). `None` until the cache is on.
    pub fn cache_split(&self) -> Option<(u64, u64)> {
        self.cache.as_ref().map(|_| {
            (
                self.cache_hits - self.cache_hits_imported,
                self.cache_hits_imported,
            )
        })
    }

    /// Every budget-independent cached outcome as a [`Fact`](crate::Fact) (docs/12):
    /// value entries by register args, state entries with fields named through
    /// `state_addrs`. Entries whose input addresses aren't name-mapped are skipped —
    /// facts are name-keyed, and a raw-address drive has no name to claim. Order is
    /// unspecified (the exporter sorts).
    pub(crate) fn cached_facts(
        &self,
        state_addrs: &[(String, u16, crate::Ty)],
    ) -> Vec<crate::Fact> {
        use crate::facts::FactInput;
        let mut out = Vec::new();
        // Reverse symbol map, smallest name winning ties deterministically.
        let mut names: Vec<(&String, &u16)> = self.prog.symbols.iter().collect();
        names.sort();
        let name_of = |addr: u16| -> Option<&str> {
            names
                .iter()
                .find(|(_, a)| **a == addr)
                .map(|(n, _)| n.as_str())
        };
        if let Some(c) = self.cache.as_ref() {
            for ((entry_addr, args), (f, _)) in c {
                let Some(entry) = name_of(*entry_addr) else {
                    continue;
                };
                out.push(crate::Fact {
                    artifact: self.artifact_hash,
                    entry: entry.to_string(),
                    input: FactInput::Args(args.clone()),
                    regs: f.regs,
                    cycles: f.cycles,
                    trapped_ops: f.trapped_ops,
                    halt: f.halt,
                    out: Vec::new(),
                });
            }
        }
        if let Some(c) = self.state_cache.as_ref() {
            for ((entry_addr, inputs, _reads), (f, state, _)) in c {
                let Some(entry) = name_of(*entry_addr) else {
                    continue;
                };
                // Name each input address; skip the entry if any is unnameable.
                let mut fields = Vec::with_capacity(inputs.len());
                let mut ok = true;
                for (addr, _, val) in inputs {
                    match state_addrs.iter().find(|(_, a, _)| a == addr) {
                        Some((n, _, _)) => fields.push((n.clone(), *val)),
                        None => {
                            ok = false;
                            break;
                        }
                    }
                }
                if !ok {
                    continue;
                }
                fields.sort();
                // Canonical form sorts `out` keys too; the importer restores the
                // declaration order from the artifact's own state_addrs.
                let mut sorted_out = state.clone();
                sorted_out.sort();
                out.push(crate::Fact {
                    artifact: self.artifact_hash,
                    entry: entry.to_string(),
                    input: FactInput::Fields(fields),
                    regs: f.regs,
                    cycles: f.cycles,
                    trapped_ops: f.trapped_ops,
                    halt: f.halt,
                    out: sorted_out,
                });
            }
        }
        out
    }

    /// Stamp an imported fact into the cache (marked `imported` for the provenance
    /// split). Returns `Ok(false)` when an identical entry already exists,
    /// `Ok(true)` when inserted, and `Err` (carrying the existing outcome) on a
    /// **collision with a differing outcome** — the caller decides it by execution
    /// (two contradictory facts cannot both be true of a deterministic machine).
    /// Existing local entries win upstream tie-breaks: they *are* execution results.
    pub(crate) fn insert_fact(
        &mut self,
        f: &crate::Fact,
        state_addrs: &[(String, u16, crate::Ty)],
    ) -> Result<bool, String> {
        use crate::facts::FactInput;
        let entry_addr = *self
            .prog
            .symbols
            .get(&f.entry)
            .ok_or_else(|| format!("no entry `{}` in this artifact", f.entry))?;
        let fast = Fast {
            result: f.regs[0],
            regs: f.regs,
            cycles: f.cycles,
            trapped_ops: f.trapped_ops,
            halt: f.halt,
        };
        match &f.input {
            FactInput::Args(args) => {
                let cache = self
                    .cache
                    .as_mut()
                    .ok_or("cache is not enabled on this runner")?;
                let key = (entry_addr, args.clone());
                if let Some((have, _)) = cache.get(&key) {
                    if same_fast(have, &fast) {
                        return Ok(false);
                    }
                    return Err(crate::facts::fact_outcome(
                        have.regs,
                        have.cycles,
                        have.trapped_ops,
                        have.halt,
                        &[],
                    ));
                }
                cache.insert(key, (fast, true));
                Ok(true)
            }
            FactInput::Fields(fields) => {
                let inputs = crate::facts::resolve_fields(fields, state_addrs)?;
                let mut key_in: Vec<(u16, u8, u64)> =
                    inputs.iter().map(|(a, t, v)| (*a, t.code(), *v)).collect();
                key_in.sort_unstable();
                // The host read-set convention: every scalar field, declaration order.
                let key_rd: Vec<(u16, u8)> =
                    state_addrs.iter().map(|(_, a, t)| (*a, t.code())).collect();
                let cache = self
                    .state_cache
                    .as_mut()
                    .ok_or("cache is not enabled on this runner")?;
                // Store the read-back in declaration order (what a live
                // `run_state_fast` returns), not the fact's canonical key order.
                let mut decl_out = Vec::with_capacity(f.out.len());
                for (name, _, _) in state_addrs {
                    if let Some((_, v)) = f.out.iter().find(|(k, _)| k == name) {
                        decl_out.push((name.clone(), *v));
                    }
                }
                let key = (entry_addr, key_in, key_rd);
                if let Some((have, have_out, _)) = cache.get(&key) {
                    let mut have_sorted = have_out.clone();
                    have_sorted.sort();
                    if same_fast(have, &fast) && have_sorted == f.out {
                        return Ok(false);
                    }
                    return Err(crate::facts::fact_outcome(
                        have.regs,
                        have.cycles,
                        have.trapped_ops,
                        have.halt,
                        have_out,
                    ));
                }
                cache.insert(key, (fast, decl_out, true));
                Ok(true)
            }
        }
    }

    /// Compile `src` (permissive) and instantiate — back-compat for trusted/game code.
    /// Untrusted cells should use [`compile_with_config`](Runner::compile_with_config).
    pub fn compile(src: &str) -> Result<Self, String> {
        Ok(Self::new(&CellProgram::compile(src)?))
    }

    /// Compile `src` under `cfg` and instantiate.
    pub fn compile_with_config(src: &str, cfg: CellConfig) -> Result<Self, String> {
        Ok(Self::new(&CellProgram::compile_with_config(src, cfg)?))
    }

    /// The compiled program (symbol map, code).
    pub fn program(&self) -> &Program {
        &self.prog
    }

    /// Run `entry` (or `run`/`main` if `None`) with `args` in the calling-convention
    /// registers (`HL`/`DE`/`BC`), bounded by `budget` T-states. Memory the previous
    /// run touched is zeroed first, so repeated runs start from the same clean state.
    pub fn run(
        &mut self,
        entry: Option<&str>,
        args: &[u16],
        budget: u64,
    ) -> Result<Report, String> {
        self.run_with_inputs(entry, args, &[], budget)
    }

    /// Like [`run`](Runner::run), but first writes typed `inputs` `(addr, ty, value)` into
    /// memory after the reset — so a cell whose state lives at a known base reads
    /// caller-supplied values (resolve field addresses with [`rustz80::struct_layout`]).
    pub fn run_with_inputs(
        &mut self,
        entry: Option<&str>,
        args: &[u16],
        inputs: &[(u16, Ty, u64)],
        budget: u64,
    ) -> Result<Report, String> {
        // Non-scalar fields can't ride the scalar input triple: an array field is
        // driven element-wise (`StateCell::set_array` / `run_state_values` expand it
        // to per-element scalar triples before this call); a `bytes[N]`/`str[N]`
        // buffer's byte-I/O surface is Phase S3. Reject before exec, which assumes
        // validated scalars.
        if let Some((addr, ty, _)) = inputs.iter().find(|(_, ty, _)| ty.capacity().is_some()) {
            return Err(format!(
                "input at {addr:#06x} is {ty} — not a scalar; arrays are driven \
                 element-wise through the array surface, byte buffers arrive with \
                 Phase S3"
            ));
        }
        let (entry, entry_addr) = self.resolve_entry(entry)?;
        let (regs, cycles, trapped_ops, halt) = self.exec(entry_addr, args, inputs, budget);
        // Observability: clone the symbol map + size report + coalesce the memory diff.
        // The hot path skips all of this — see `run_fast`.
        self.touched.sort_unstable();
        Ok(Report {
            entry,
            entry_addr,
            result: regs[0],
            regs,
            cycles,
            trapped_ops,
            budget,
            returned: halt == Halt::Returned,
            halt,
            code_bytes: self.prog.code.len(),
            fn_count: self.prog.size_report().len(),
            symbols: sorted_symbols(&self.prog.symbols),
            touched: coalesce(&self.touched),
            reads: Vec::new(),
            cache_stats: self.cache_stats(),
        })
    }

    /// The **hot path**: run `entry` and return just the result registers, cycles, and
    /// halt — *no* symbol-map clone, size report, or memory-diff (no per-call
    /// allocations). For tight agent loops over many candidates (see `run` for the rich
    /// [`Report`]).
    pub fn run_fast(
        &mut self,
        entry: Option<&str>,
        args: &[u16],
        budget: u64,
    ) -> Result<Fast, String> {
        let entry_addr = self.resolve_addr(entry)?;
        if self.cache.is_some() {
            self.cache_lookups += 1;
            let key = (entry_addr, args.to_vec());
            // A stored outcome replays only if it fit *strictly* inside this budget.
            // (Conservative: at cycles == budget the live run in fact completes —
            // the final instruction starts while cycles < budget — so equality is
            // a miss that re-executes to the same outcome, never a wrong answer.)
            if let Some((f, imported)) = self.cache.as_ref().and_then(|c| c.get(&key)) {
                if f.cycles < budget {
                    self.cache_hits += 1;
                    if *imported {
                        self.cache_hits_imported += 1;
                    }
                    return Ok(*f);
                }
            }
            let f = self.exec_fast(entry_addr, args, budget);
            // Budget/memory stops are budget- or config-relative; everything else is a
            // deterministic property of (entry, args) and caches forever.
            if !matches!(f.halt, Halt::CycleBudget | Halt::MemoryLimit) {
                self.cache
                    .as_mut()
                    .expect("checked above")
                    .insert(key, (f, false));
            }
            return Ok(f);
        }
        Ok(self.exec_fast(entry_addr, args, budget))
    }

    /// The cached **state-cell** hot path (docs/12 §2 — the delta that puts the
    /// scoring workhorses under the memo table): write typed `inputs` into the state
    /// at [`STATE_BASE`], run the entry (state base in `HL`, the state-cell
    /// convention), and read `reads` back — the whole outcome (named fields +
    /// [`Fast`]) memoized under `(entry, sorted inputs, read set)`. The same
    /// budget-independence and strict-replay rules as [`run_fast`](Runner::run_fast)
    /// apply, so a hit is byte-for-byte what the run would have produced.
    pub fn run_state_fast(
        &mut self,
        entry: Option<&str>,
        inputs: &[(u16, Ty, u64)],
        reads: &[(String, u16, Ty)],
        budget: u64,
    ) -> Result<(Fast, Vec<(String, u64)>), String> {
        // Non-scalar fields can't ride the scalar triple (same rule as
        // `run_with_inputs`); the memoized read-back is scalar-shaped too, so an
        // array-state cell takes the uncached `run_state_values` lane instead.
        if let Some((addr, ty, _)) = inputs.iter().find(|(_, ty, _)| ty.capacity().is_some()) {
            return Err(format!(
                "input at {addr:#06x} is {ty} — not a scalar; arrays are driven \
                 element-wise through the array surface, byte buffers arrive with \
                 Phase S3"
            ));
        }
        let entry_addr = self.resolve_addr(entry)?;
        if self.state_cache.is_some() {
            self.cache_lookups += 1;
            // Canonical key: inputs sorted by address (unique per field), plus the
            // read set — a fact is only well-defined given what was read back.
            let mut key_in: Vec<(u16, u8, u64)> =
                inputs.iter().map(|(a, t, v)| (*a, t.code(), *v)).collect();
            key_in.sort_unstable();
            let key_rd: Vec<(u16, u8)> = reads.iter().map(|(_, a, t)| (*a, t.code())).collect();
            let key = (entry_addr, key_in, key_rd);
            if let Some((f, state, imported)) = self.state_cache.as_ref().and_then(|c| c.get(&key))
            {
                if f.cycles < budget {
                    self.cache_hits += 1;
                    if *imported {
                        self.cache_hits_imported += 1;
                    }
                    return Ok((*f, state.clone()));
                }
            }
            let (regs, cycles, trapped_ops, halt) =
                self.exec(entry_addr, &[STATE_BASE], inputs, budget);
            let f = Fast {
                result: regs[0],
                regs,
                cycles,
                trapped_ops,
                halt,
            };
            let state = self.read_named(reads);
            if !matches!(f.halt, Halt::CycleBudget | Halt::MemoryLimit) {
                self.state_cache
                    .as_mut()
                    .expect("checked above")
                    .insert(key, (f, state.clone(), false));
            }
            return Ok((f, state));
        }
        let (regs, cycles, trapped_ops, halt) =
            self.exec(entry_addr, &[STATE_BASE], inputs, budget);
        let f = Fast {
            result: regs[0],
            regs,
            cycles,
            trapped_ops,
            halt,
        };
        let state = self.read_named(reads);
        Ok((f, state))
    }

    /// Run the same entry over many argument sets, reusing **all** setup — the "score N
    /// candidates" path. The entry is resolved once (no per-call name allocation/lookup).
    ///
    /// If the entry is **straight-line** over the opcode subset the compiler emits (no
    /// branches/calls/`halt`), it's decoded once and replayed by a stripped fast executor
    /// (no per-instruction fetch/contention/refresh/flag work) — several × faster. The
    /// cycle count is then input-independent, so it's taken from one authentic calibration
    /// run; results are still the real Z80 semantics (oracle-validated). Anything outside
    /// that subset transparently falls back to the authentic interpreter, per input.
    /// One [`Fast`] per input set, in order.
    pub fn run_many_fast(
        &mut self,
        entry: Option<&str>,
        arg_sets: &[&[u16]],
        budget: u64,
    ) -> Result<Vec<Fast>, String> {
        let entry_addr = self.resolve_addr(entry)?;
        if let Some(ops) = fast::decode(&self.prog.code, entry_addr) {
            // Calibrate the (input-independent) cycle count + confirm a clean return under
            // budget. If the cell doesn't return cleanly (shouldn't for straight-line), or
            // there are no inputs, fall through to the authentic path.
            if let Some(first) = arg_sets.first() {
                // Straight-line ⇒ cycles + trapped_ops are input-independent, so one
                // calibration run gives both for the whole batch.
                let (_, cycles, trapped_ops, halt) = self.exec(entry_addr, first, &[], budget);
                if halt == Halt::Returned {
                    let div0_halts = self.cfg.div_by_zero == DivByZero::Halt;
                    let mut out = Vec::with_capacity(arg_sets.len());
                    for args in arg_sets {
                        let (regs, div0) = fast::run(
                            &ops,
                            &mut self.mem,
                            &mut self.seen,
                            &mut self.touched,
                            args,
                            div0_halts,
                        );
                        out.push(if div0 {
                            // A divide-by-zero halts mid-run, breaking the input-independent
                            // cycle premise — take the authentic path for this input so the
                            // reported cycles/halt are the real ones.
                            self.exec_fast(entry_addr, args, budget)
                        } else {
                            Fast {
                                result: regs[0],
                                regs,
                                cycles,
                                trapped_ops,
                                halt: Halt::Returned,
                            }
                        });
                    }
                    return Ok(out);
                }
            }
        }
        // Fallback: the authentic interpreter, per input.
        Ok(arg_sets
            .iter()
            .map(|args| self.exec_fast(entry_addr, args, budget))
            .collect())
    }

    /// `exec` + pack a [`Fast`] — the shared body of `run_fast`/`run_many_fast`.
    fn exec_fast(&mut self, entry_addr: u16, args: &[u16], budget: u64) -> Fast {
        let (regs, cycles, trapped_ops, halt) = self.exec(entry_addr, args, &[], budget);
        Fast {
            result: regs[0],
            regs,
            cycles,
            trapped_ops,
            halt,
        }
    }

    /// Resolve just the entry **address** (default `run`, then `main`) — no name
    /// allocation, for the hot path. The named [`resolve_entry`](Self::resolve_entry) is
    /// for the `Report` path, which needs the name.
    fn resolve_addr(&self, entry: Option<&str>) -> Result<u16, String> {
        let name = match entry {
            Some(e) => e,
            None if self.prog.symbols.contains_key("run") => "run",
            None if self.prog.symbols.contains_key("main") => "main",
            None => return Err("no `run` or `main` entry — pass an explicit entry".into()),
        };
        self.prog.symbols.get(name).copied().ok_or_else(|| {
            let mut names: Vec<String> = self.prog.symbols.keys().cloned().collect();
            names.sort();
            format!("no entry `{name}`; available: {}", names.join(", "))
        })
    }

    /// Resolve the entry name + address (defaulting to `run`, then `main`).
    fn resolve_entry(&self, entry: Option<&str>) -> Result<(String, u16), String> {
        let entry = match entry {
            Some(e) => e.to_string(),
            None if self.prog.symbols.contains_key("run") => "run".to_string(),
            None if self.prog.symbols.contains_key("main") => "main".to_string(),
            None => return Err("no `run` or `main` entry — pass an explicit entry".into()),
        };
        let addr = *self.prog.symbols.get(&entry).ok_or_else(|| {
            let mut names: Vec<String> = self.prog.symbols.keys().cloned().collect();
            names.sort();
            format!("no entry `{entry}`; available: {}", names.join(", "))
        })?;
        Ok((entry, addr))
    }

    /// Reset (zero last run's writes + restore code), lay the trampoline + inputs, and
    /// step the CPU. Returns `(regs[HL,DE,BC], cycles, trapped_ops, halt)`. The
    /// allocation-free core shared by `run`/`run_fast`.
    fn exec(
        &mut self,
        entry_addr: u16,
        args: &[u16],
        inputs: &[(u16, Ty, u64)],
        budget: u64,
    ) -> ([u16; 3], u64, u64, Halt) {
        // Reset only the bytes the previous run wrote, then restore the code (in case it
        // was poked).
        for &a in &self.touched {
            self.mem[a as usize] = 0;
            self.seen[a as usize] = false;
        }
        self.touched.clear();
        let org = ORG as usize;
        self.mem[org..org + self.prog.code.len()].copy_from_slice(&self.prog.code);

        // Trampoline written straight to memory (no per-call Vec): load args into
        // HL/DE/BC, CALL the entry, HALT on return.
        const LD: [u8; 3] = [0x21, 0x11, 0x01];
        let mut p = TRAMPOLINE as usize;
        for (i, &v) in args.iter().enumerate().take(3) {
            self.mem[p] = LD[i];
            self.mem[p + 1] = v as u8;
            self.mem[p + 2] = (v >> 8) as u8;
            p += 3;
        }
        self.mem[p] = 0xCD; // CALL entry
        self.mem[p + 1] = entry_addr as u8;
        self.mem[p + 2] = (entry_addr >> 8) as u8;
        self.mem[p + 3] = 0x76; // HALT

        // Typed inputs (after the reset, so they survive it; marked touched so the next
        // run cleans them). Little-endian, low byte first.
        for &(addr, ty, val) in inputs {
            let bytes = match ty {
                Ty::U8 => 1,
                Ty::U16 => 2,
                Ty::U32 | Ty::F32 => 4, // f32 rides as its raw binary32 bits
                // Validated out by `run_with_inputs` (arrays are pre-expanded to
                // per-element scalars; buffers wait for the S3 byte-I/O surface).
                Ty::Bytes(_) | Ty::Str(_) | Ty::Array(..) => {
                    unreachable!("non-scalar input reached exec")
                }
            };
            for i in 0..bytes {
                let a = addr.wrapping_add(i as u16) as usize;
                self.mem[a] = (val >> (8 * i)) as u8;
                if !self.seen[a] {
                    self.seen[a] = true;
                    self.touched.push(a as u16);
                }
            }
        }

        let max_touched = self.cfg.max_touched;
        let mut bus = CellBus {
            mem: &mut self.mem,
            seen: &mut self.seen,
            touched: &mut self.touched,
            cycles: 0,
            halt: None,
            trapped_ops: 0,
            div0_halts: self.cfg.div_by_zero == DivByZero::Halt,
            div0: false,
        };
        let mut cpu = z80::Cpu::new();
        cpu.reset();
        cpu.regs.pc = TRAMPOLINE;
        cpu.regs.sp = SP_TOP;
        let mut mem_limit = false;
        while !cpu.halted && bus.cycles < budget {
            cpu.step(&mut bus);
            if bus.halt.is_some() || bus.div0 {
                break; // `halt(code)` or a divide-by-zero — stop right after the trap
            }
            if matches!(max_touched, Some(m) if bus.touched.len() > m) {
                mem_limit = true;
                break;
            }
        }
        let halt = if bus.div0 {
            Halt::DivByZero
        } else if let Some(code) = bus.halt {
            // The escalation band: a structured "this exceeds the kernel class" hand-off,
            // not an outcome — see `ESCALATE_BASE`.
            if code >= crate::ESCALATE_BASE {
                Halt::Escalate(code)
            } else {
                Halt::Halted(code)
            }
        } else if cpu.halted {
            Halt::Returned
        } else if mem_limit {
            Halt::MemoryLimit
        } else {
            Halt::CycleBudget
        };
        (
            [cpu.regs.hl(), cpu.regs.de(), cpu.regs.bc()],
            bus.cycles,
            bus.trapped_ops,
            halt,
        )
    }

    /// Read a byte from the cell's memory *after a run* (the bus stays live until the
    /// next [`run`](Runner::run) resets it).
    pub fn peek_u8(&self, addr: u16) -> u8 {
        self.mem[addr as usize]
    }
    /// Read a little-endian `u16` (one slot).
    pub fn peek_u16(&self, addr: u16) -> u16 {
        u16::from_le_bytes([
            self.mem[addr as usize],
            self.mem[addr.wrapping_add(1) as usize],
        ])
    }
    /// Read a `u32` (two slots: low word at `addr`, high word at `addr + 2`).
    pub fn peek_u32(&self, addr: u16) -> u32 {
        self.peek_u16(addr) as u32 | (self.peek_u16(addr.wrapping_add(2)) as u32) << 16
    }
    /// Decode named, typed values from post-run memory — the typed state read-back. The
    /// `(name, addr, ty)` layout is the caller's (e.g. from a state-struct symbol map);
    /// this turns it into `(name, value)` pairs read off the live bus.
    pub fn read_named(&self, fields: &[(String, u16, Ty)]) -> Vec<(String, u64)> {
        fields
            .iter()
            .filter_map(|(name, addr, ty)| {
                let v = match ty {
                    Ty::U8 => self.peek_u8(*addr) as u64,
                    Ty::U16 => self.peek_u16(*addr) as u64,
                    // f32 reads back as its raw bits — the host converts with
                    // `f32::from_bits`; the type keeps the repr from blurring.
                    Ty::U32 | Ty::F32 => self.peek_u32(*addr) as u64,
                    // A buffer/array field has no scalar reading — skipped rather
                    // than misreported; arrays read whole via `read_named_values`,
                    // the byte read-back surface is Phase S3.
                    Ty::Bytes(_) | Ty::Str(_) | Ty::Array(..) => return None,
                };
                Some((name.clone(), v))
            })
            .collect()
    }

    /// Decode named, typed values from post-run memory **including array fields** —
    /// the value-envelope read-back ([`FieldValue`]). Scalars read as
    /// [`read_named`](Runner::read_named); a `u16[N]`/`u32[N]` field reads its whole
    /// declared envelope, element values in order. Buffer fields (`bytes[N]`/
    /// `str[N]`) are still skipped (Phase S3).
    pub fn read_named_values(&self, fields: &[(String, u16, Ty)]) -> Vec<(String, FieldValue)> {
        fields
            .iter()
            .filter_map(|(name, addr, ty)| {
                let v = match ty {
                    Ty::U8 => FieldValue::Scalar(self.peek_u8(*addr) as u64),
                    Ty::U16 => FieldValue::Scalar(self.peek_u16(*addr) as u64),
                    Ty::U32 | Ty::F32 => FieldValue::Scalar(self.peek_u32(*addr) as u64),
                    Ty::Array(elem, len) => FieldValue::Array(
                        (0..*len)
                            .map(|i| {
                                let a = addr + i * elem.bytes();
                                match elem {
                                    crate::ArrayElem::U16 => self.peek_u16(a) as u64,
                                    crate::ArrayElem::U32 => self.peek_u32(a) as u64,
                                }
                            })
                            .collect(),
                    ),
                    Ty::Bytes(_) | Ty::Str(_) => return None,
                };
                Some((name.clone(), v))
            })
            .collect()
    }

    /// Re-point this runner at `program`, **reusing the allocated 64 KiB bus** (for
    /// [`CellPool`]). Clears the previous run's writes and the previous program's code so
    /// there's no cross-program leakage, then loads the new code — paying only O(code), not
    /// a fresh 128 KiB alloc/zero.
    fn reset_for(&mut self, program: &CellProgram) {
        for &a in &self.touched {
            self.mem[a as usize] = 0;
            self.seen[a as usize] = false;
        }
        self.touched.clear();
        let org = ORG as usize;
        for b in self.mem[org..org + self.prog.code.len()].iter_mut() {
            *b = 0; // the old program's code (the new one may be shorter)
        }
        self.prog = program.prog.clone();
        self.cfg = program.cfg.clone();
        self.mem[org..org + self.prog.code.len()].copy_from_slice(&self.prog.code);
        // A banked program needs the resident kernel bank exactly like `Runner::new`
        // stamps it — a recycled bus may never have carried it (born under a non-bank
        // cell), and running into zeroed 0xC000 is a cycle-budget runaway, not an
        // error. The bank lives outside touch-tracking, so stamping is idempotent
        // and survives per-run resets; a stale bank left for a non-bank successor
        // is unreachable (sandboxed cells can't address it).
        if program.uses_kernel_bank() {
            let bank = rustz80::kernel_bank();
            let b = rustz80::BANK_ORG as usize;
            self.mem[b..b + bank.code.len()].copy_from_slice(&bank.code);
        }
        // Memoized results belong to the *previous* program — drop them (and the
        // counters: stats are per-program, or the hit-rate lies across a pool reuse).
        if let Some(c) = self.cache.as_mut() {
            c.clear();
            self.cache_hits = 0;
            self.cache_lookups = 0;
            self.cache_hits_imported = 0;
        }
        if let Some(c) = self.state_cache.as_mut() {
            c.clear();
        }
        // The content address follows the program; a cartridge-backed reload
        // re-stamps via `set_artifact_hash` after this.
        self.artifact_hash = self_hash(program);
    }
}

/// Field-for-field equality of two [`Fast`] outcomes (no `PartialEq` on `Fast` —
/// this is the one comparison site, and it should stay explicit).
fn same_fast(a: &Fast, b: &Fast) -> bool {
    a.regs == b.regs && a.cycles == b.cycles && a.trapped_ops == b.trapped_ops && a.halt == b.halt
}

/// The content address of a **bare** (cartridge-less) program: SHA-256 over its
/// serialized image — which carries the code, symbols, *and* the whole `CellConfig`
/// (capabilities, ceilings, div-by-zero policy), so no outcome-affecting knob
/// escapes the hash. Distinct from a cartridge's v5 artifact hash (that one also
/// covers the manifest); facts verify against whichever machine produced them.
fn self_hash(program: &CellProgram) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(program.to_bytes());
    h.finalize().into()
}

/// A pool of reusable 64 KiB buses. Acquiring a cell for *any* program recycles an idle bus
/// instead of allocating + zeroing a fresh 128 KiB (the ~1 µs `Runner::new` cost the
/// lifecycle bench isolates) — paying only to load the code. For "spawn many short-lived
/// cells" / "instantiate N candidate tools concurrently" patterns: [`acquire`](Self::acquire)
/// a runner, run it, [`release`](Self::release) it back. The pool grows to the high-water
/// mark of live cells.
#[derive(Default)]
pub struct CellPool {
    idle: Vec<Runner>,
}

impl CellPool {
    /// An empty pool (allocates buses lazily, on the first [`acquire`](Self::acquire)).
    pub fn new() -> Self {
        Self::default()
    }

    /// A runner loaded with `program` — recycling an idle bus if one is free (no 128 KiB
    /// alloc), else allocating one. Return it with [`release`](Self::release).
    pub fn acquire(&mut self, program: &CellProgram) -> Runner {
        match self.idle.pop() {
            Some(mut r) => {
                r.reset_for(program);
                r
            }
            None => Runner::new(program),
        }
    }

    /// Return a runner to the pool so its bus can be reused by a later acquire.
    pub fn release(&mut self, runner: Runner) {
        self.idle.push(runner);
    }

    /// How many buses are idle (reusable without allocation).
    pub fn idle_count(&self) -> usize {
        self.idle.len()
    }
}

/// One-shot convenience: compile `src` and run `entry` once (see [`Runner`] for
/// compile-once/run-many).
pub fn run(src: &str, entry: Option<&str>, args: &[u16], budget: u64) -> Result<Report, String> {
    Runner::compile(src)?.run(entry, args, budget)
}
