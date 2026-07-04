//! Run outcome types — `Halt`, `Fast`, `Report` (+ `Ty`) and their formatters.
use std::collections::HashMap;

/// The frozen cell ABI / report-schema version (`"abi"` in [`Report::to_json`]). Bump only
/// on a breaking change to the register/memory/capability contract or the JSON shape. See
/// `docs/09-cell80-abi.md`.
// v2: the 32-bit lane — `ED FE` traps 0x12 (MUL32) / 0x13 (DIVMOD32), and `u32` state
// fields (two little-endian slots) drivable/readable by name (`.cell` format v4 carries
// a width per state field). Additive, but a v2 cartridge that traps 0x12/0x13 needs a
// v2 host — an older host treats unknown trap ids as no-ops and would compute garbage.
// v3 (Phase S, `docs/11-machine-text.md` §3): the buffer manifest types — `bytes[N]`
// (a byte-packed `[u8; N]` state field: N raw bytes at the address) and `str[N]` (a
// length-prefixed UTF-8 buffer: u16 LE length + up to N bytes, valid by the §4
// contract). Additive: scalar fields are unchanged; `.cell` format v6 carries the
// capacity per buffer field. Buffer fields are *declared* in v3; the host byte-I/O
// surface (packing/promotion) arrives with Phase S3.
pub const ABI_VERSION: u32 = 3;

/// The first halt code of the **escalation band** (`0xFF00..=0xFFFF`): a `halt(code)`
/// in this range is not a failure but a structured *hand-off* — the cell declares the
/// request exceeded the kernel class (needs strings / floats / I/O / …), so an
/// orchestrator routes it up the ladder instead of retrying or reporting an error. The
/// band rides the existing `halt` trap, so it needs **no compiler surface**: any cell
/// can `halt(0xFF03)` today. See [`Halt::Escalate`] and `docs/09-cell80-abi.md`.
pub const ESCALATE_BASE: u16 = 0xFF00;

/// The named reasons of the escalation band (`ESCALATE_BASE + offset`). Codes in the
/// band beyond the named set decode as `"custom"` — the code itself is always carried.
pub const ESCALATE_REASONS: [(u16, &str); 7] = [
    (0xFF00, "unspecified"),
    (0xFF01, "needs_strings"),
    (0xFF02, "needs_floats"),
    (0xFF03, "needs_io"),
    (0xFF04, "needs_network"),
    (0xFF05, "needs_wider_math"),
    (0xFF06, "out_of_domain"),
];

/// Why a run stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Halt {
    /// The entry returned (clean).
    Returned,
    /// The program called `halt(code)` (Cell80 `ED FE` HALT) — an explicit stop.
    Halted(u16),
    /// The program called `halt(code)` with a code in the [`ESCALATE_BASE`] band — a
    /// structured escalation: *this request exceeds the kernel class; hand it to the
    /// next rung* (vs [`Halted`](Halt::Halted), an outcome, and the limit stops,
    /// failures). Decode the reason with [`escalate_reason`](Halt::escalate_reason).
    Escalate(u16),
    /// The T-state budget was reached first.
    CycleBudget,
    /// The `max_touched` memory ceiling was reached.
    MemoryLimit,
    /// A `/ 0` or `% 0` reached a divide trap (the default [`crate::DivByZero::Halt`]
    /// policy) — the run stopped rather than letting a garbage quotient flow onward.
    DivByZero,
}

impl Halt {
    fn as_str(self) -> &'static str {
        match self {
            Halt::Returned => "returned",
            Halt::Halted(_) => "halted",
            Halt::Escalate(_) => "escalate",
            Halt::CycleBudget => "cycle_budget",
            Halt::MemoryLimit => "memory_limit",
            Halt::DivByZero => "div_by_zero",
        }
    }

    /// The escalation reason name for an [`Escalate`](Halt::Escalate) code
    /// (`"custom"` for unnamed codes in the band), `None` for any other halt.
    pub fn escalate_reason(self) -> Option<&'static str> {
        match self {
            Halt::Escalate(c) => Some(
                ESCALATE_REASONS
                    .iter()
                    .find(|(code, _)| *code == c)
                    .map_or("custom", |(_, name)| name),
            ),
            _ => None,
        }
    }
}

/// A typed state-field kind: a scalar width for typed memory read-back, or (ABI v3)
/// a fixed-capacity buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ty {
    U8,
    U16,
    U32,
    /// `bytes[N]` — a byte-packed `[u8; N]` state field: `N` raw bytes at the field
    /// address. Declared in the manifest so a caller can read the envelope; scalar
    /// `set`/`get` paths reject it (the byte-buffer I/O surface is Phase S3).
    Bytes(u16),
    /// `str[N]` — a length-prefixed UTF-8 buffer (u16 LE length, then up to `N`
    /// bytes), valid by the host-boundary contract. Manifest annotation for the
    /// Phase S3 `str_out` promotion; no field lowers to it before then.
    Str(u16),
}

impl Ty {
    /// Parse `u8`/`u16`/`u32`, or the v3 buffer forms `bytes[N]`/`str[N]`.
    pub fn parse(s: &str) -> Result<Ty, String> {
        match s {
            "u8" => Ok(Ty::U8),
            "u16" => Ok(Ty::U16),
            "u32" => Ok(Ty::U32),
            other => {
                let buf = |prefix: &str| -> Option<Result<Ty, String>> {
                    let inner = other.strip_prefix(prefix)?.strip_suffix(']')?;
                    Some(match inner.parse::<u16>() {
                        Ok(n) if prefix.starts_with("bytes") => Ok(Ty::Bytes(n)),
                        Ok(n) => Ok(Ty::Str(n)),
                        Err(_) => Err(format!("bad capacity in `{other}`")),
                    })
                };
                buf("bytes[").or_else(|| buf("str[")).unwrap_or_else(|| {
                    Err(format!(
                        "unknown type `{other}` (want u8/u16/u32/bytes[N]/str[N])"
                    ))
                })
            }
        }
    }

    /// The one-byte wire code (the `.cell` manifest's `state_addrs` encoding).
    /// Buffer codes (`3`/`4`) are followed on the wire by a u16 capacity —
    /// format v6+ only (see `cartridge.rs`).
    pub fn code(self) -> u8 {
        match self {
            Ty::U16 => 0,
            Ty::U32 => 1,
            Ty::U8 => 2,
            Ty::Bytes(_) => 3,
            Ty::Str(_) => 4,
        }
    }

    /// Decode a [`code`](Ty::code) byte; buffer codes take the capacity that
    /// followed on the wire.
    pub fn from_code(c: u8, capacity: u16) -> Result<Ty, String> {
        match c {
            0 => Ok(Ty::U16),
            1 => Ok(Ty::U32),
            2 => Ok(Ty::U8),
            3 => Ok(Ty::Bytes(capacity)),
            4 => Ok(Ty::Str(capacity)),
            other => Err(format!("unknown state-field type code {other}")),
        }
    }

    /// `Some(N)` for the v3 buffer kinds; `None` for scalars.
    pub fn capacity(self) -> Option<u16> {
        match self {
            Ty::Bytes(n) | Ty::Str(n) => Some(n),
            _ => None,
        }
    }
}

impl std::fmt::Display for Ty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ty::U8 => write!(f, "u8"),
            Ty::U16 => write!(f, "u16"),
            Ty::U32 => write!(f, "u32"),
            Ty::Bytes(n) => write!(f, "bytes[{n}]"),
            Ty::Str(n) => write!(f, "str[{n}]"),
        }
    }
}

/// The lightweight outcome of a [`run_fast`](crate::Runner::run_fast): the result registers,
/// T-states, and halt reason — no allocations (no symbol map, size report, or memory
/// diff). For tight agent loops.
#[derive(Debug, Clone, Copy)]
pub struct Fast {
    /// The primary result in `HL`.
    pub result: u16,
    /// All three result registers `[HL, DE, BC]`.
    pub regs: [u16; 3],
    /// T-states elapsed. **See the caveat on [`Report::cycles`] — a deterministic *relative*
    /// cost, not authentic Z80 time; pair it with `trapped_ops` before using as a signal.**
    pub cycles: u64,
    /// Count of cost-bearing `ED FE` host traps (`mul`/`div`/fill) — see [`Report::trapped_ops`].
    pub trapped_ops: u64,
    pub halt: Halt,
}

/// The structured outcome of a [`run`](crate::run).
#[derive(Debug, Clone)]
pub struct Report {
    /// The entry function that was run, and its address.
    pub entry: String,
    pub entry_addr: u16,
    /// The primary result in `HL`.
    pub result: u16,
    /// All three result registers `[HL, DE, BC]` — a `-> (u16, u16, u16)` tuple return
    /// fills all three (`result` is `regs[0]`).
    pub regs: [u16; 3],
    /// Named typed values decoded from post-run memory (empty unless requested via
    /// [`Runner::read_named`](crate::Runner::read_named) / the CLI `--read`).
    pub reads: Vec<(String, u64)>,
    /// T-states elapsed, and the budget it ran under. **Caveat — not authentic Z80 time:**
    /// in Cell mode `*`/`/`/`%` and `[v; N]` fills are `ED FE` host traps serviced natively
    /// and charged a flat ~4 T-states, *not* the real software-routine cost. So `cycles` is
    /// a **deterministic relative cost metric** — correct for liveness (the budget) and
    /// replay, but it must **not** be read as hardware-fidelity time or used as an RL reward
    /// (that would reward shoving work into traps that read as "free"). Pair it with
    /// `trapped_ops` to make a faithful cost signal. See `docs/09-cell80-abi.md`.
    pub cycles: u64,
    pub budget: u64,
    /// How many cost-bearing `ED FE` host traps (`mul`/`div`/fill) the run executed — the
    /// honest companion to `cycles`. Each trap is charged a flat ~4 T-states, so a program
    /// with high `trapped_ops` did real work that `cycles` undercounts. A reward function
    /// should weight or refuse trap-heavy programs rather than treat low `cycles` as cheap.
    pub trapped_ops: u64,
    /// Did the entry return cleanly (`true`)? (Shorthand for `halt == Halt::Returned`.)
    pub returned: bool,
    /// Why the run stopped (returned / cycle budget / memory limit).
    pub halt: Halt,
    /// Total compiled code size, and the number of functions (incl. monomorphic
    /// instances + the appended runtime).
    pub code_bytes: usize,
    pub fn_count: usize,
    /// The symbol map (name → address), sorted by address.
    pub symbols: Vec<(String, u16)>,
    /// Contiguous RAM ranges written during the run, as `(start, end_inclusive)`.
    pub touched: Vec<(u16, u16)>,
    /// `(hits, lookups)` of this runner's memoization cache — `None` unless the cache was
    /// [enabled](crate::Runner::enable_cache). The cache serves the fast path; this is the
    /// hit-rate counter riding along on the rich report.
    pub cache_stats: Option<(u64, u64)>,
}

impl Report {
    /// A human-readable, aligned summary.
    pub fn to_human(&self) -> String {
        let halt = match self.halt {
            Halt::Returned => "returned".to_string(),
            Halt::Halted(c) => format!("halted (code {c})"),
            Halt::Escalate(c) => format!(
                "escalated ({}, code {c:#06x}) — request exceeds the kernel class",
                self.halt.escalate_reason().unwrap_or("custom")
            ),
            Halt::CycleBudget => format!("CYCLE BUDGET EXCEEDED (≥ {} T-states)", self.budget),
            Halt::MemoryLimit => "MEMORY LIMIT EXCEEDED".to_string(),
            Halt::DivByZero => "DIVIDE BY ZERO".to_string(),
        };
        let syms: Vec<String> = self
            .symbols
            .iter()
            .map(|(n, a)| format!("{n}@{a:#06x}"))
            .collect();
        let mem: Vec<String> = self
            .touched
            .iter()
            .map(|(s, e)| format!("{s:#06x}-{e:#06x} ({}B)", e - s + 1))
            .collect();
        let mem = if mem.is_empty() {
            "(none written)".to_string()
        } else {
            mem.join(", ")
        };
        let reads = if self.reads.is_empty() {
            String::new()
        } else {
            let r: Vec<String> = self.reads.iter().map(|(n, v)| format!("{n}={v}")).collect();
            format!("\nreads      {}", r.join(", "))
        };
        format!(
            "entry      {} @ {:#06x}\n\
             result     {} ({:#06x})\n\
             regs       HL={} DE={} BC={}\n\
             cycles     {} / {} T-states ({} trapped op(s) — see ABI note)\n\
             halt       {halt}\n\
             code       {} bytes, {} functions\n\
             symbols    {}\n\
             memory     {mem}{reads}",
            self.entry,
            self.entry_addr,
            self.result,
            self.result,
            self.regs[0],
            self.regs[1],
            self.regs[2],
            self.cycles,
            self.budget,
            self.trapped_ops,
            self.code_bytes,
            self.fn_count,
            syms.join(", "),
        )
    }

    /// A single-line JSON object (for machine/agent consumption).
    pub fn to_json(&self) -> String {
        let syms: Vec<String> = self
            .symbols
            .iter()
            .map(|(n, a)| format!("\"{n}\":{a}"))
            .collect();
        let mem: Vec<String> = self
            .touched
            .iter()
            .map(|(s, e)| format!("[{s},{e}]"))
            .collect();
        let reads: Vec<String> = self
            .reads
            .iter()
            .map(|(n, v)| format!("\"{n}\":{v}"))
            .collect();
        // `halt_code` only appears for an explicit `halt(code)`; an escalation carries
        // its machine-readable reason + raw code.
        let halt_code = match self.halt {
            Halt::Halted(c) => format!(",\"halt_code\":{c}"),
            Halt::Escalate(c) => format!(
                ",\"escalate\":\"{}\",\"escalate_code\":{c}",
                self.halt.escalate_reason().unwrap_or("custom")
            ),
            _ => String::new(),
        };
        // `cache` only appears when memoization is enabled on the runner.
        let cache = match self.cache_stats {
            Some((h, n)) => format!(",\"cache\":{{\"hits\":{h},\"lookups\":{n}}}"),
            None => String::new(),
        };
        format!(
            "{{\"abi\":{},\"entry\":\"{}\",\"entry_addr\":{},\"result\":{},\"regs\":[{},{},{}],\"cycles\":{},\
             \"trapped_ops\":{},\"budget\":{},\"halt\":\"{}\"{},\"code_bytes\":{},\"functions\":{},\
             \"symbols\":{{{}}},\"memory_touched\":[{}],\"reads\":{{{}}}{cache}}}",
            ABI_VERSION,
            self.entry,
            self.entry_addr,
            self.result,
            self.regs[0],
            self.regs[1],
            self.regs[2],
            self.cycles,
            self.trapped_ops,
            self.budget,
            self.halt.as_str(),
            halt_code,
            self.code_bytes,
            self.fn_count,
            syms.join(","),
            mem.join(","),
            reads.join(","),
        )
    }
}

pub(super) fn sorted_symbols(symbols: &HashMap<String, u16>) -> Vec<(String, u16)> {
    let mut v: Vec<(String, u16)> = symbols.iter().map(|(k, a)| (k.clone(), *a)).collect();
    v.sort_by_key(|(_, a)| *a);
    v
}

/// Coalesce a *sorted* list of distinct addresses into contiguous `(start, end)` ranges.
pub(super) fn coalesce(sorted: &[u16]) -> Vec<(u16, u16)> {
    let mut ranges: Vec<(u16, u16)> = Vec::new();
    for &a in sorted {
        match ranges.last_mut() {
            Some(last) if last.1.checked_add(1) == Some(a) => last.1 = a,
            _ => ranges.push((a, a)),
        }
    }
    ranges
}
