//! The **fact file** (docs/12) — an exportable, spot-checkable memo table.
//!
//! One line, one claim: *cell `<artifact_hash>`, entry `<name>`, given `<input>`,
//! produces `<outcome>` at `<cost>` — forever.* The file is boring on purpose:
//! JSONL, canonical field order (so `sort -u` merges and `diff` means something),
//! no per-line crypto — **a fact's integrity is checked by executing it**. The
//! importer samples unpredictably (its own seed, never file content), fails the
//! whole file on a caught lie by default, and decides key collisions by running
//! the key: two contradictory facts cannot both be true of a deterministic machine.

use super::{Fast, Halt, Ty};
use std::collections::HashMap;
use std::io::{BufRead, Write};

/// One fact: a claim about a single run of a single artifact.
#[derive(Debug, Clone, PartialEq)]
pub struct Fact {
    /// The content address (a cartridge's v5 artifact hash, or a bare image's
    /// self-hash). Never an entry address — addresses are image-internal.
    pub artifact: [u8; 32],
    /// The entry symbol, by name (resolved against the artifact's own symbols at
    /// import — same hash ⇒ same layout; the name is for human eyes).
    pub entry: String,
    /// Value-cell register args, or state-cell named fields (keys sorted — the one
    /// canonicalization rule).
    pub input: FactInput,
    /// The `Fast` surface: `[HL, DE, BC]`.
    pub regs: [u16; 3],
    pub cycles: u64,
    pub trapped_ops: u64,
    /// Never `CycleBudget`/`MemoryLimit` — budget-dependent outcomes aren't facts.
    pub halt: Halt,
    /// Named output fields (state facts; empty for value facts).
    pub out: Vec<(String, u64)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FactInput {
    Args(Vec<u16>),
    /// Named state fields, sorted by name.
    Fields(Vec<(String, u64)>),
}

impl Fact {
    /// The canonical JSONL line (fixed field order, no whitespace). Emitting is
    /// hand-rolled so the order is the *spec's* order, not an alphabetized map.
    pub fn to_line(&self) -> String {
        let mut s = String::with_capacity(128);
        s.push_str("{\"a\":\"sha256:");
        s.push_str(&hex(&self.artifact));
        s.push_str("\",\"e\":");
        s.push_str(&serde_json::Value::from(self.entry.as_str()).to_string());
        match &self.input {
            FactInput::Args(args) => {
                s.push_str(",\"args\":[");
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        s.push(',');
                    }
                    s.push_str(&a.to_string());
                }
                s.push(']');
            }
            FactInput::Fields(fields) => {
                s.push_str(",\"f\":{");
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        s.push(',');
                    }
                    s.push_str(&serde_json::Value::from(k.as_str()).to_string());
                    s.push(':');
                    s.push_str(&v.to_string());
                }
                s.push('}');
            }
        }
        s.push_str(&format!(
            ",\"r\":[{},{},{}],\"cy\":{},\"tr\":{},\"h\":\"{}\"",
            self.regs[0],
            self.regs[1],
            self.regs[2],
            self.cycles,
            self.trapped_ops,
            halt_str(self.halt),
        ));
        if !self.out.is_empty() {
            s.push_str(",\"out\":{");
            for (i, (k, v)) in self.out.iter().enumerate() {
                if i > 0 {
                    s.push(',');
                }
                s.push_str(&serde_json::Value::from(k.as_str()).to_string());
                s.push(':');
                s.push_str(&v.to_string());
            }
            s.push('}');
        }
        s.push('}');
        s
    }

    /// Parse one fact line (any key order — parsing is `serde_json`, only emitting
    /// is order-fixed). `Err` carries what's wrong with the line.
    pub fn from_line(line: &str) -> Result<Fact, String> {
        let v: serde_json::Value =
            serde_json::from_str(line).map_err(|e| format!("not JSON: {e}"))?;
        let obj = v.as_object().ok_or("not a JSON object")?;
        let a = obj
            .get("a")
            .and_then(|x| x.as_str())
            .ok_or("missing artifact `a`")?;
        let artifact = unhex(
            a.strip_prefix("sha256:")
                .ok_or("`a` must be sha256:<hex>")?,
        )?;
        let entry = obj
            .get("e")
            .and_then(|x| x.as_str())
            .ok_or("missing entry `e`")?
            .to_string();
        let input = if let Some(args) = obj.get("args") {
            let args = args.as_array().ok_or("`args` must be an array")?;
            FactInput::Args(
                args.iter()
                    .map(|x| {
                        x.as_u64()
                            .filter(|v| *v <= u16::MAX as u64)
                            .map(|v| v as u16)
                            .ok_or_else(|| "bad arg (want u16)".to_string())
                    })
                    .collect::<Result<_, _>>()?,
            )
        } else if let Some(f) = obj.get("f") {
            let f = f.as_object().ok_or("`f` must be an object")?;
            let mut fields: Vec<(String, u64)> = f
                .iter()
                .map(|(k, x)| {
                    x.as_u64()
                        .map(|v| (k.clone(), v))
                        .ok_or_else(|| format!("bad field `{k}` (want u64)"))
                })
                .collect::<Result<_, _>>()?;
            fields.sort();
            FactInput::Fields(fields)
        } else {
            return Err("missing input (`args` or `f`)".into());
        };
        let r = obj
            .get("r")
            .and_then(|x| x.as_array())
            .filter(|a| a.len() == 3)
            .ok_or("missing/short regs `r`")?;
        let mut regs = [0u16; 3];
        for (i, x) in r.iter().enumerate() {
            regs[i] = x
                .as_u64()
                .filter(|v| *v <= u16::MAX as u64)
                .ok_or("bad reg (want u16)")? as u16;
        }
        let cycles = obj
            .get("cy")
            .and_then(|x| x.as_u64())
            .ok_or("missing `cy`")?;
        let trapped_ops = obj
            .get("tr")
            .and_then(|x| x.as_u64())
            .ok_or("missing `tr`")?;
        let halt = parse_halt(
            obj.get("h")
                .and_then(|x| x.as_str())
                .ok_or("missing halt `h`")?,
        )?;
        let out = match obj.get("out") {
            Some(o) => {
                let o = o.as_object().ok_or("`out` must be an object")?;
                let mut out: Vec<(String, u64)> = o
                    .iter()
                    .map(|(k, x)| {
                        x.as_u64()
                            .map(|v| (k.clone(), v))
                            .ok_or_else(|| format!("bad out field `{k}`"))
                    })
                    .collect::<Result<_, _>>()?;
                out.sort();
                out
            }
            None => Vec::new(),
        };
        Ok(Fact {
            artifact,
            entry,
            input,
            regs,
            cycles,
            trapped_ops,
            halt,
            out,
        })
    }
}

/// The `h` field encoding. `CycleBudget`/`MemoryLimit` never encode — budget- and
/// config-relative stops aren't facts (an importer rejects such a line on sight).
fn halt_str(h: Halt) -> String {
    match h {
        Halt::Returned => "ok".into(),
        Halt::Halted(c) => format!("halt:{c}"),
        Halt::Escalate(c) => format!("escalate:{c}"),
        Halt::DivByZero => "div_by_zero".into(),
        Halt::CycleBudget | Halt::MemoryLimit => {
            unreachable!("budget-dependent outcomes are never exported")
        }
    }
}

fn parse_halt(s: &str) -> Result<Halt, String> {
    if s == "ok" {
        return Ok(Halt::Returned);
    }
    if s == "div_by_zero" {
        return Ok(Halt::DivByZero);
    }
    if let Some(c) = s.strip_prefix("halt:") {
        return Ok(Halt::Halted(c.parse().map_err(|_| "bad halt code")?));
    }
    if let Some(c) = s.strip_prefix("escalate:") {
        return Ok(Halt::Escalate(c.parse().map_err(|_| "bad escalate code")?));
    }
    if s == "cycle_budget" || s == "memory_limit" {
        return Err("budget-dependent outcome — not a fact".into());
    }
    Err(format!("unknown halt `{s}`"))
}

pub(crate) fn hex(b: &[u8; 32]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn unhex(s: &str) -> Result<[u8; 32], String> {
    if s.len() != 64 {
        return Err("artifact hash must be 64 hex chars".into());
    }
    let mut out = [0u8; 32];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).map_err(|_| "bad hex")?;
    }
    Ok(out)
}

/// Import policy (docs/12 §3): how much to spot-check and what a caught lie does.
#[derive(Debug, Clone)]
pub struct ImportPolicy {
    /// Fraction of accepted lines re-executed on import (default 0.01, min 1 line).
    pub verify_fraction: f64,
    /// On a failed verification: quarantine just the failing lines (salvage mode)
    /// instead of rejecting the whole file (`false`, the default — one caught lie
    /// removes the unverified remainder's standing).
    pub quarantine: bool,
    /// Sampling seed — the importer's own (tests may set it; a producer must not
    /// be able to predict it, so `None` draws from local entropy).
    pub seed: Option<u64>,
    /// Verify without importing (the `facts verify` audit verb).
    pub dry_run: bool,
}

impl Default for ImportPolicy {
    fn default() -> Self {
        ImportPolicy {
            verify_fraction: 0.01,
            quarantine: false,
            seed: None,
            dry_run: false,
        }
    }
}

/// What an import did — the Act-3 closing frame, as data.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImportReport {
    pub read: u64,
    pub accepted: u64,
    pub rejected_unknown_artifact: u64,
    pub rejected_budget_halt: u64,
    pub rejected_malformed: u64,
    pub verified: u64,
    /// Verifications that caught a lie: `(line number, key, expected vs re-executed)`.
    pub failures: Vec<FactFailure>,
    /// Whether the file was rejected wholesale (`FailFile`, the default policy).
    pub file_failed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FactFailure {
    pub line: u64,
    pub key: String,
    pub expected: String,
    pub got: String,
}

impl ImportReport {
    /// The human rendering — the demo's closing frame.
    pub fn to_human(&self) -> String {
        let mut s = format!(
            "read {} · accepted {} · verified {}",
            self.read, self.accepted, self.verified
        );
        if self.rejected_unknown_artifact + self.rejected_budget_halt + self.rejected_malformed > 0
        {
            s.push_str(&format!(
                " · rejected {} (unknown artifact {}, budget halt {}, malformed {})",
                self.rejected_unknown_artifact
                    + self.rejected_budget_halt
                    + self.rejected_malformed,
                self.rejected_unknown_artifact,
                self.rejected_budget_halt,
                self.rejected_malformed
            ));
        }
        if self.failures.is_empty() {
            s.push_str(" · no lies caught");
        } else {
            s.push_str(&format!(
                " · {} LIE(S) CAUGHT{}",
                self.failures.len(),
                if self.file_failed {
                    " — FILE REJECTED"
                } else {
                    " — quarantined"
                }
            ));
            for f in &self.failures {
                s.push_str(&format!(
                    "\n  line {}: {} — claimed {} / re-executed {}",
                    f.line, f.key, f.expected, f.got
                ));
            }
        }
        s
    }

    pub fn to_json(&self) -> String {
        let failures: Vec<String> = self
            .failures
            .iter()
            .map(|f| {
                format!(
                    "{{\"line\":{},\"key\":{},\"expected\":{},\"got\":{}}}",
                    f.line,
                    serde_json::Value::from(f.key.as_str()),
                    serde_json::Value::from(f.expected.as_str()),
                    serde_json::Value::from(f.got.as_str()),
                )
            })
            .collect();
        format!(
            "{{\"read\":{},\"accepted\":{},\"rejected_unknown_artifact\":{},\
             \"rejected_budget_halt\":{},\"rejected_malformed\":{},\"verified\":{},\
             \"file_failed\":{},\"failures\":[{}]}}",
            self.read,
            self.accepted,
            self.rejected_unknown_artifact,
            self.rejected_budget_halt,
            self.rejected_malformed,
            self.verified,
            self.file_failed,
            failures.join(",")
        )
    }
}

/// Write the header line + every fact, canonical form. `count` is stamped up front,
/// so the caller passes the facts as a slice.
pub(crate) fn write_facts(
    w: &mut impl Write,
    producer: &str,
    facts: &[Fact],
) -> Result<(), String> {
    writeln!(
        w,
        "{{\"facts\":1,\"lib\":\"cell80\",\"producer\":{},\"created\":{},\"count\":{}}}",
        serde_json::Value::from(producer),
        unix_now(),
        facts.len()
    )
    .map_err(|e| e.to_string())?;
    for f in facts {
        writeln!(w, "{}", f.to_line()).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Read a fact file: `(facts, per-line rejects)` — the header line(s) are skipped,
/// malformed/budget-halt lines are collected as rejects `(line no, why, budgetish)`.
#[allow(clippy::type_complexity)]
pub(crate) fn read_facts(
    r: impl BufRead,
) -> Result<(Vec<(u64, Fact)>, Vec<(u64, String, bool)>), String> {
    let mut facts = Vec::new();
    let mut rejects = Vec::new();
    for (i, line) in r.lines().enumerate() {
        let n = i as u64 + 1;
        let line = line.map_err(|e| format!("line {n}: {e}"))?;
        let t = line.trim();
        if t.is_empty() || t.starts_with("{\"facts\":") {
            continue; // header / blank
        }
        match Fact::from_line(t) {
            Ok(f) => facts.push((n, f)),
            Err(why) => {
                let budgetish = why.contains("budget-dependent");
                rejects.push((n, why, budgetish));
            }
        }
    }
    Ok((facts, rejects))
}

/// A tiny xorshift64* — the importer's own sampling randomness. Deliberately not
/// derived from file content (a producer who can predict the sample tampers
/// elsewhere); tests seed it, adversaries can't.
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: Option<u64>) -> Rng {
        let s = seed.unwrap_or_else(|| {
            use std::time::{SystemTime, UNIX_EPOCH};
            let t = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0x9E37_79B9_7F4A_7C15);
            // Mix in an allocation address so two imports in the same nanosecond
            // (or a mocked clock) still diverge.
            let probe = Box::new(0u8);
            t ^ (&*probe as *const u8 as u64)
        });
        Rng(s | 1)
    }
    pub(crate) fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform in `[0, 1)` (53-bit).
    pub(crate) fn f64(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn unix_now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// A human key for failure messages: `entry(input)`.
pub(crate) fn fact_key(f: &Fact) -> String {
    match &f.input {
        FactInput::Args(a) => format!(
            "{}({})",
            f.entry,
            a.iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ),
        FactInput::Fields(fs) => format!(
            "{}({})",
            f.entry,
            fs.iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

/// The outcome half of a fact, for expected-vs-got messages.
pub(crate) fn fact_outcome(
    regs: [u16; 3],
    cycles: u64,
    tr: u64,
    halt: Halt,
    out: &[(String, u64)],
) -> String {
    let mut s = format!(
        "r=[{},{},{}] cy={} tr={} h={}",
        regs[0],
        regs[1],
        regs[2],
        cycles,
        tr,
        halt_str_safe(halt)
    );
    if !out.is_empty() {
        s.push_str(" out={");
        for (i, (k, v)) in out.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&format!("{k}={v}"));
        }
        s.push('}');
    }
    s
}

/// Like [`halt_str`] but total — verification can *observe* a budget halt (a fact
/// that runs long is a lie), it just never exports one.
fn halt_str_safe(h: Halt) -> String {
    match h {
        Halt::CycleBudget => "cycle_budget".into(),
        Halt::MemoryLimit => "memory_limit".into(),
        other => halt_str(other),
    }
}

/// Resolve a fact's named fields against an artifact's `state_addrs`.
pub(crate) fn resolve_fields(
    fields: &[(String, u64)],
    state_addrs: &[(String, u16, Ty)],
) -> Result<Vec<(u16, Ty, u64)>, String> {
    fields
        .iter()
        .map(|(name, val)| {
            state_addrs
                .iter()
                .find(|(n, _, _)| n == name)
                .map(|(_, a, t)| (*a, *t, *val))
                .ok_or_else(|| format!("no state field `{name}`"))
        })
        .collect()
}

/// The artifact-hash → id map over a catalog (the "do I hold this cell?" check).
pub(crate) fn hash_index<'a>(
    carts: impl Iterator<Item = (&'a String, &'a super::Cartridge)>,
) -> HashMap<[u8; 32], String> {
    carts
        .map(|(id, c)| (c.artifact_hash(), id.clone()))
        .collect()
}

// ── the fact-file verbs on the host (docs/12 §3–§4) ────────────────────────────
//
// The model/parse/emit halves live above; the *verbs* live here too (rather than in
// `host.rs`) so the whole fact surface is one module. Same crate, same `CellHost`.

use super::CellHost;

impl CellHost {
    /// Export every budget-independent cached outcome across the loaded runners as a
    /// `.facts` file (docs/12 §1): a header line, then one canonical JSONL claim per
    /// fact, sorted (so `sort -u` merges and re-exports are stable). Returns the
    /// fact count.
    pub fn export_facts(&self, w: &mut impl std::io::Write, producer: &str) -> Result<u64, String> {
        let mut facts = Vec::new();
        for l in self.live.iter().flatten() {
            facts.extend(l.runner.cached_facts(&l.state_addrs));
        }
        let mut lines: Vec<(String, Fact)> = facts.into_iter().map(|f| (f.to_line(), f)).collect();
        lines.sort_by(|a, b| a.0.cmp(&b.0));
        lines.dedup_by(|a, b| a.0 == b.0);
        let facts: Vec<Fact> = lines.into_iter().map(|(_, f)| f).collect();
        crate::facts::write_facts(w, producer, &facts)?;
        Ok(facts.len() as u64)
    }

    /// Import a `.facts` file (docs/12 §3) — **the spot-check is the product**:
    ///
    /// - a fact about an artifact this catalog doesn't hold is rejected (it would
    ///   be unfalsifiable *to us*), as is any budget-dependent or malformed line;
    /// - a locally-seeded sample (`verify_fraction`, min 1) is **re-executed**, each
    ///   fact under its own claimed cost + 1 — a fact that runs long is a lie even
    ///   if the result matches;
    /// - one caught lie fails the whole file by default (`quarantine` salvages the
    ///   verified remainder instead);
    /// - key collisions with differing outcomes are decided **by execution** — the
    ///   existing entry of a warm runner already *is* an execution result, so it
    ///   wins and the newcomer is reported as a failure.
    ///
    /// Accepted facts stamp into already-loaded runners now and stage for future
    /// [`load`](Self::load)s. `dry_run` verifies without importing (`facts verify`).
    pub fn import_facts(
        &mut self,
        r: impl std::io::BufRead,
        policy: &ImportPolicy,
    ) -> Result<ImportReport, String> {
        let (parsed, rejects) = crate::facts::read_facts(r)?;
        let mut rep = ImportReport {
            read: (parsed.len() + rejects.len()) as u64,
            ..Default::default()
        };
        for (_, _, budgetish) in &rejects {
            if *budgetish {
                rep.rejected_budget_halt += 1;
            } else {
                rep.rejected_malformed += 1;
            }
        }
        // Unknown artifact ⇒ reject the line (this file carries zero trust).
        let by_hash = crate::facts::hash_index(self.catalog.iter());
        let mut accepted: Vec<(u64, Fact, String)> = Vec::new();
        for (line, f) in parsed {
            match by_hash.get(&f.artifact) {
                Some(id) => accepted.push((line, f, id.clone())),
                None => rep.rejected_unknown_artifact += 1,
            }
        }
        rep.accepted = accepted.len() as u64;

        // Contradiction pass (file-internal): two lines with the same key and
        // different outcomes cannot both be true of a deterministic machine —
        // execute the key, keep the truth, report the losers. Decidable, decided.
        {
            let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
            for (i, (_, f, _)) in accepted.iter().enumerate() {
                let k = format!(
                    "{}/{}",
                    crate::facts::hex(&f.artifact),
                    crate::facts::fact_key(f)
                );
                groups.entry(k).or_default().push(i);
            }
            let mut drop: std::collections::HashSet<usize> = std::collections::HashSet::new();
            for idxs in groups.values() {
                if idxs.len() < 2 {
                    continue;
                }
                let differing = idxs.windows(2).any(|w| {
                    let a = &accepted[w[0]].1;
                    let b = &accepted[w[1]].1;
                    (a.regs, a.cycles, a.trapped_ops, a.halt, &a.out)
                        != (b.regs, b.cycles, b.trapped_ops, b.halt, &b.out)
                });
                if !differing {
                    // Exact duplicates: keep one, silently drop the rest.
                    for &i in &idxs[1..] {
                        drop.insert(i);
                    }
                    continue;
                }
                for &i in idxs {
                    let (line, f, id) = &accepted[i];
                    rep.verified += 1;
                    if let Some(failure) = self.verify_fact(f, &id.clone(), *line)? {
                        rep.failures.push(failure);
                        drop.insert(i);
                    }
                }
            }
            let mut keep = Vec::with_capacity(accepted.len());
            for (i, item) in accepted.into_iter().enumerate() {
                if !drop.contains(&i) {
                    keep.push(item);
                }
            }
            accepted = keep;
        }

        // The unpredictable sample: the importer's own seed, never file content.
        let mut rng = crate::facts::Rng::new(policy.seed);
        let mut sample: Vec<usize> = (0..accepted.len())
            .filter(|_| rng.f64() < policy.verify_fraction)
            .collect();
        if sample.is_empty() && !accepted.is_empty() && policy.verify_fraction > 0.0 {
            sample.push((rng.next() % accepted.len() as u64) as usize); // min 1
        }
        for &i in &sample {
            let (line, f, id) = &accepted[i];
            if let Some(failure) = self.verify_fact(f, id, *line)? {
                rep.failures.push(failure);
            }
            rep.verified += 1;
        }
        if !rep.failures.is_empty() && !policy.quarantine {
            // FailFile: one lie removes the unverified remainder's standing.
            rep.file_failed = true;
            return Ok(rep);
        }
        if policy.dry_run {
            return Ok(rep);
        }
        let failed_lines: std::collections::HashSet<u64> =
            rep.failures.iter().map(|f| f.line).collect();
        for (line, f, _) in accepted {
            if failed_lines.contains(&line) {
                continue; // quarantined
            }
            // Already-loaded runners with this artifact take the fact now; a
            // collision with a differing outcome is decided by the warm runner's
            // own (execution-produced) entry — the newcomer loses, loudly.
            for l in self.live.iter_mut().flatten() {
                if l.runner.artifact_hash() == f.artifact {
                    if let Err(have) = l.runner.insert_fact(&f, &l.state_addrs) {
                        rep.failures.push(FactFailure {
                            line,
                            key: crate::facts::fact_key(&f),
                            expected: crate::facts::fact_outcome(
                                f.regs,
                                f.cycles,
                                f.trapped_ops,
                                f.halt,
                                &f.out,
                            ),
                            got: have,
                        });
                    }
                }
            }
            // Stage — a contradiction against an *earlier import's* staged fact is
            // decided by execution too: the newcomer must prove itself to displace.
            let same_key = |a: &Fact, b: &Fact| {
                a.entry == b.entry && a.input == b.input && a.artifact == b.artifact
            };
            let existing = self
                .imported
                .get(&f.artifact)
                .and_then(|v| v.iter().position(|old| same_key(old, &f)));
            match existing {
                Some(pos) if self.imported[&f.artifact][pos] == f => {} // duplicate
                Some(pos) => {
                    let id = by_hash.get(&f.artifact).expect("accepted ⇒ known").clone();
                    rep.verified += 1;
                    match self.verify_fact(&f, &id, line)? {
                        None => {
                            // The newcomer holds — the staged claim was the lie.
                            let old = self
                                .imported
                                .get_mut(&f.artifact)
                                .expect("existing pos")
                                .remove(pos);
                            rep.failures.push(FactFailure {
                                line,
                                key: crate::facts::fact_key(&old),
                                expected: crate::facts::fact_outcome(
                                    old.regs,
                                    old.cycles,
                                    old.trapped_ops,
                                    old.halt,
                                    &old.out,
                                ),
                                got: crate::facts::fact_outcome(
                                    f.regs,
                                    f.cycles,
                                    f.trapped_ops,
                                    f.halt,
                                    &f.out,
                                ),
                            });
                            self.imported.get_mut(&f.artifact).expect("entry").push(f);
                        }
                        Some(failure) => rep.failures.push(failure), // newcomer lied
                    }
                }
                None => self.imported.entry(f.artifact).or_default().push(f),
            }
        }
        Ok(rep)
    }

    /// Re-execute **every** line of a fact file against this catalog — the CI-able
    /// audit verb (`cell80 facts verify --all`). Nothing is imported.
    pub fn verify_facts(&mut self, r: impl std::io::BufRead) -> Result<ImportReport, String> {
        self.import_facts(
            r,
            &ImportPolicy {
                verify_fraction: 1.0,
                dry_run: true,
                quarantine: true, // report every failure, not just the first batch
                seed: Some(0),    // fraction 1.0 — the seed is irrelevant
            },
        )
    }

    /// Re-execute one fact and compare: `None` = it holds, `Some` = a caught lie.
    /// The budget is the fact's own claim + 1 — by determinism a true fact replays
    /// in exactly its recorded cycles, so running long falsifies the cost claim
    /// even when the result matches.
    fn verify_fact(
        &mut self,
        f: &Fact,
        id: &str,
        line: u64,
    ) -> Result<Option<FactFailure>, String> {
        let h = self.load(id)?;
        let budget = f.cycles + 1;
        let result = (|| -> Result<Option<FactFailure>, String> {
            let (got, got_out): (Fast, Vec<(String, u64)>) = match &f.input {
                crate::facts::FactInput::Args(args) => {
                    (self.run_fast(h, args, budget)?, Vec::new())
                }
                crate::facts::FactInput::Fields(fields) => {
                    let l = self.loaded(h)?;
                    let addrs = l.state_addrs.clone();
                    let entry = l.entry.clone();
                    let inputs = crate::facts::resolve_fields(fields, &addrs)?;
                    let reads: Vec<(String, u16, Ty)> = addrs.clone();
                    l.runner
                        .run_state_fast(Some(&entry), &inputs, &reads, budget)?
                }
            };
            let out_holds = f
                .out
                .iter()
                .all(|(k, v)| got_out.iter().any(|(gk, gv)| gk == k && gv == v));
            let holds = got.regs == f.regs
                && got.cycles == f.cycles
                && got.trapped_ops == f.trapped_ops
                && got.halt == f.halt
                && out_holds;
            Ok(if holds {
                None
            } else {
                Some(FactFailure {
                    line,
                    key: crate::facts::fact_key(f),
                    expected: crate::facts::fact_outcome(
                        f.regs,
                        f.cycles,
                        f.trapped_ops,
                        f.halt,
                        &f.out,
                    ),
                    got: crate::facts::fact_outcome(
                        got.regs,
                        got.cycles,
                        got.trapped_ops,
                        got.halt,
                        &got_out,
                    ),
                })
            })
        })();
        self.unload(h)?;
        result
    }
}
