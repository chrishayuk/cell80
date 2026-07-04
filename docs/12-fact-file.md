# The fact file — an exportable, spot-checkable memo table (3.3+)

*Status: specified, not started. Follows shipped 3.3 (the in-process cache) and rides
shipped 3.1 (the v5 artifact hash). Two deliverables: the **key upgrade** (image-hash
keyed, state-cell coverage) and the **file** (export / import-with-spot-check). The
design goal is stated once and enforced everywhere: **boring on purpose** — a fact
file must be readable in a text editor, diffable, greppable, and falsifiable by
execution. Trust comes from re-running, never from the file.*

## What a fact is

One line, one claim:

> Cell `<artifact_hash>`, entry `<name>`, given `<args>`, produces `<outcome>` at
> `<cost>` — forever.

The claim is eternal because every outcome-affecting knob lives *inside* the hash:
the image, the manifest, the capability policy, and — this is why 0.3 carried the
div-by-zero flag in the image — the halt policy. If a future config knob can change
an outcome, it must be hashed or it must not exist. That invariant is the spec's
load-bearing wall; it gets its own test (§7).

*Invariant audit (2026-07-04, at spec time): the wall holds today. All five
`CellConfig` fields serialize into the image bytes (`program.rs` `to_bytes`, flags +
ceilings), the image is inside `artifact_hash`, `Runner::new`/`load` copy `cfg` from
the program with no divergence path (the field is private, no setter), memory is
reset O(touched) per exec with the code restored, and the bus is closed —
`inport` reads a constant `0xFF`, `output` is a no-op, no clock, no RNG. The only
outcome-affecting knob outside the image is the cycle budget — which is exactly the
one the format excludes by construction (§1).*

## 1. File format — JSON Lines, one canonical form

Extension `.facts` (JSONL content). Line 1 is the header; every other line is a fact.

```
{"facts":1,"lib":"cell80","producer":"chris@m3max","created":"2026-07-04T14:02:11Z","count":10000}
{"a":"sha256:9f3c…e21","e":"run","args":[300,400],"r":[57600,0,0],"cy":412,"tr":2,"h":"ok"}
{"a":"sha256:9f3c…e21","e":"run","f":{"wx":3,"wy":5,"x":17,"y":40},"r":[0,0,0],"cy":1893,"tr":4,"h":"ok","out":{"dist":250000}}
{"a":"sha256:b7d1…044","e":"run","args":[7,0],"r":[0,0,0],"cy":96,"tr":1,"h":"div_by_zero"}
```

- **`a`** — the v5 artifact hash (manifest + image). Never an entry address:
  addresses are image-internal; the hash *is* the image.
- **`e`** — entry symbol by name (resolved against the artifact's own symbols at
  import; same hash ⇒ same layout, the name is for human eyes).
- **`args`** (value cells) *or* **`f`** (state cells): named fields, **keys sorted
  lexicographically** — the one canonicalization rule. `u32` fields carry full
  values. `out` carries named output fields for state cells.
- **`r`/`cy`/`tr`/`h`** — the `Fast` surface: `[HL, DE, BC]`, cycles, trapped_ops,
  halt (`ok` / `div_by_zero` / `escalate:<code>` / …).
- **Never stored:** `h = "cycle_budget"`. Budget-dependent outcomes aren't facts —
  the shipped 3.3 invariant, carried into the format: an importer **rejects the
  line** (not the file) on sight.

Canonical form (sorted keys, no whitespace, fixed field order) exists so `sort -u`
merges fact files and `diff` means something — not for hashing. There are no
per-line MACs and no Merkle anything, deliberately: **a fact's integrity is checked
by executing it**, which is cheaper than designing crypto and impossible to get
wrong the same way. The optional ed25519 signature (3.1's keypair, over the file's
SHA-256, in a trailing `{"sig":…}` line) is **attribution, not verification** — it
answers "who published this," never "is this true." The spec says so in the type
name: `Attribution`, not `Proof`.

## 2. The key upgrade (in-process, precedes the file)

- Cache key becomes `(artifact_hash, entry, canonical_args)` — `Runner` gets the
  hash at construction from the cartridge (a plain program without a cartridge
  hashes its own image + config on the spot, so `Runner::compile` paths stay
  cached).
- **`run_state_fast`** — the delta-two fix. State-cell inputs are already
  deterministic key material (`(name, ty, value)` triples, sorted); outcome = the
  named output fields + `Fast`. This is the coverage that matters: the scoring
  workhorses are all state cells. The rich `run()` stays uncached, as shipped —
  post-run memory is not fakeable and we don't fake it.
- `Report.cache` grows one counter: hits served from **imported** facts vs locally
  computed — provenance is one u64, and the Act-3 screen wants the split.

## 3. Import — the spot-check is the product

```rust
pub struct ImportPolicy {
    /// Fraction of accepted lines re-executed on import (default 0.01, min 1 line).
    pub verify_fraction: f64,
    /// On a failed verification: reject the whole file (default) or quarantine lines.
    pub on_failure: FailFile | QuarantineLines,
    /// Cycle ceiling per verification run (default: the fact's own `cy` + 1 —
    /// a true fact replays inside its own recorded cost, by determinism).
    pub verify_budget: VerifyBudget,
}
pub struct ImportReport {
    pub read: u64, pub accepted: u64,
    pub rejected_unknown_artifact: u64, pub rejected_budget_halt: u64,
    pub verified: u64,
    pub failures: Vec<FactFailure>,   // line no, key, expected vs re-executed
}
```

Decisions, each with its reason:

- **Unknown artifact ⇒ reject the line.** A fact about a cell you don't hold is
  unfalsifiable *to you* — importing it would be trust, and this file carries none.
  (Corollary: a fact file travels well next to the `.cell`s it speaks about; the
  CLI gets `cell80 facts export --with-cells` for exactly that.)
- **Sampling is seeded locally, never derived from file content.** If the producer
  can predict which lines get checked, they tamper elsewhere. The RNG is the
  importer's; tests may seed it, adversaries may not.
- **Default `FailFile`.** One caught lie removes the unverified remainder's
  standing — a producer who fabricated one line fabricated an unknown number.
  `QuarantineLines` exists for salvage workflows and says so in its docs.
- **Verification is budgeted by the fact's own claim.** `cy + 1` as the ceiling is
  itself a check: a true fact, by determinism, replays in exactly `cy` — a fact
  that *runs long* is false even if the result matches. Cost claims are claims.
- **Key collision with a differing outcome — file vs file, or file vs local — is
  never a merge conflict.** Two contradictory facts cannot both be true of a
  deterministic machine: execute the key, keep the truth, report the loser as a
  `FactFailure`. Contradictions are *decidable* here; the importer decides them.

## 4. Surfaces

- **Rust:** `CellHost::export_facts(w) -> ExportStats`,
  `CellHost::import_facts(r, policy) -> ImportReport`.
- **CLI:** `cell80 facts export [--sign KEY] > lib.facts`,
  `cell80 facts import lib.facts [--verify-fraction 0.01] [--quarantine]`,
  `cell80 facts verify lib.facts --all` (the audit verb: re-execute every line,
  exit nonzero on any failure — CI-able).
- **MCP:** `cell_facts_import` / `cell_facts_stats` — the agent-visible face;
  import returns the `ImportReport` as data so an agent can *read* "9,999 verified
  claims accepted, 1 falsified" and act on it.

## 5. The demo mapping (Act 3, beat for beat)

Host A: `cell80 facts export > a.facts` after the Act-1 workload — the file opens
in a pager on camera; it is visibly just claims. Host B:
`cell80 facts import a.facts` → report shows accepted/verified counts and the
imported-vs-local hit split as the workload re-runs instantly. The tamper beat:
`sed` one digit in one line, re-import → `FailFile`, the report names the line, the
key, and the expected-vs-re-executed values. Closing frame is §3's `ImportReport`,
verbatim. Nothing in the demo is a special code path — every beat is the default
behaviour of the shipped verbs.

## 6. Deliberately out

Per-line signatures/MACs (execution is the checker) · Merkle trees / transparency
logs (a registry-scale concern for a registry-scale day) · TTLs or invalidation of
any kind (a fact file has nothing to invalidate — a changed cell is a different
`a`) · compression in-format (`zstd` the file in transit if you like; the format
stays text) · cross-version fact migration (facts about a v4-loaded image are
facts about *that* hash; there is nothing to migrate).

## 7. DoD

Key upgrade: cache keyed by artifact hash; `run_state_fast` lands with the
state-cell adoption tasks re-run cached (the hit-rate on the scoring family is the
number to publish). File: round-trip (export → import ⇒ 100% hits, zero
re-execution beyond the sample); the tamper test (result, cost, *and* halt
mutations each caught, file failed, line named); the contradiction test (two files,
same key, different outcome ⇒ execution decides); the predictability test (a
producer given the importer's code but not its seed cannot place a tamper that
survives 100 trials at 1% sampling); the invariant test — a grep-able assertion
that no `CellConfig` field outside the hashed image can alter any cached outcome
(the wall in §"What a fact is"; see the audit note there for the current evidence).
Bench: import throughput and per-fact verify cost in the table next to 3.3's 46 ns
hit.

## The one-sentence version

A fact file is a text file of claims that carry their own price, checkable by
running them; the importer samples unpredictably, fails loudly, decides
contradictions by execution, and trusts nothing — which is why anyone can trust it.
