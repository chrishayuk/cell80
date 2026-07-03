"""Tier 3 — behavioural disambiguation of the **escalated residue**.

The margin gate (tier 2) answers what the text tiers can answer safely and escalates
the rest. This module is what the escalation *carries*: cells are deterministic and
microsecond-cheap, so before a brain sees the candidates, we **execute them** on
inputs chosen to make them disagree, and attach the resulting I/O table as evidence.
Same-shape siblings (`min`/`max`, `gcd`/`lcm`) — invisible to every text signal,
measured — separate in one probe row: on `(3, 7)` one returns 3, the other 7.

Two consumers:

* **Requests carrying I/O examples** (the ladder's rung 1): `match_examples` filters
  the candidate set to the cells that reproduce them — pure execution, no model.
* **Text-only requests**: `probe_table` builds the discriminating-evidence table for
  the top-k, and the A/B eval (`run_disambiguation`) measures the thing that matters:
  does a model shown *behaviour* pick the right cell more often than one shown
  manifests alone? That lift is tier 3's number.

v1 probes **value cells** (positional params); state cells need input/output field
annotations to probe safely — that arrives with the ladder's register-time metadata
authoring, and is noted as such rather than guessed at.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from .agent import AgentConfig, make_client
from .library import open_library

DEFAULT_CYCLES = 2_000_000

# A deterministic input battery per arity: small/adjacent pairs (order matters for
# min/max, sub, pow), zeros (identity/absorbing edges), boundary values (saturation,
# wrap), and coprime/composite pairs (gcd/lcm family). Deliberately diverse — the
# selector below keeps only the rows that actually discriminate.
BATTERY: dict[int, list[list[int]]] = {
    1: [[0], [1], [2], [7], [16], [100], [255], [256], [1000], [65535]],
    2: [
        [3, 7],
        [7, 3],
        [10, 3],
        [0, 5],
        [5, 0],
        [1, 1],
        [4, 6],
        [8, 9],
        [12, 8],
        [100, 7],
        [255, 2],
        [65535, 2],
        [2, 10],
        [9, 4],
    ],
    3: [
        [3, 7, 5],
        [10, 0, 20],
        [5, 5, 5],
        [1, 2, 3],
        [100, 10, 50],
        [0, 0, 1],
        [65535, 1, 2],
        [7, 3, 11],
    ],
}
MAX_PROBES = 4  # evidence rows kept per candidate set


@dataclass
class ProbeTable:
    """Discriminating evidence for one candidate set: `outputs[cell_id] = [out, …]`
    aligned with `probes` (inputs where at least two candidates disagree)."""

    probes: list[list[int]] = field(default_factory=list)
    outputs: dict[str, list[int | None]] = field(default_factory=dict)
    skipped: list[str] = field(default_factory=list)  # state cells / arity misfits

    def render(self) -> str:
        if not self.probes:
            return "(no discriminating probes available)"
        lines = ["outputs on sample inputs (deterministic, actually executed):"]
        args = "  ".join(f"{tuple(p)!s:<14}" for p in self.probes)
        lines.append(f"  {'cell':<18}{args}")
        for cid, outs in self.outputs.items():
            row = "  ".join(f"{('?' if o is None else o)!s:<14}" for o in outs)
            lines.append(f"  {cid:<18}{row}")
        if self.skipped:
            lines.append(f"  (not probed: {', '.join(self.skipped)})")
        return "\n".join(lines)


def _run_cell(lib, handle: int, args: list[int]) -> int | None:
    try:
        rep = lib.host.run(handle, args, DEFAULT_CYCLES)
        return int(rep["result"]) if rep["halt"] == "returned" else None
    except ValueError:
        return None


def probe_table(lib, cell_ids: list[str]) -> ProbeTable:
    """Execute the candidates on the arity battery and keep the [`MAX_PROBES`] inputs
    that discriminate hardest (most distinct outputs across candidates, disagreements
    first). Value cells only in v1 — state cells are listed as unprobed."""
    t = ProbeTable()
    arities: dict[str, int] = {}
    for cid in cell_ids:
        m = lib.inspect(cid)
        if m is None:
            t.skipped.append(cid)
            continue
        if m.get("state"):
            t.skipped.append(cid)  # state cell — needs field annotations (see module doc)
            continue
        arities[cid] = len(m.get("params", []))
    if not arities:
        return t
    # Probe at the majority arity; misfits are listed, not guessed at.
    arity = max(set(arities.values()), key=lambda a: sum(v == a for v in arities.values()))
    probed = [c for c, a in arities.items() if a == arity]
    t.skipped += [c for c, a in arities.items() if a != arity]
    battery = BATTERY.get(arity, [])
    if not battery or len(probed) < 2:
        return t

    handles = {c: lib.host.load(c) for c in probed}
    try:
        rows = []  # (distinct_count, probe, {cell: out})
        for probe in battery:
            outs = {c: _run_cell(lib, handles[c], probe) for c in probed}
            distinct = len(set(outs.values()))
            if distinct > 1:
                rows.append((distinct, probe, outs))
        rows.sort(key=lambda r: -r[0])
        for _, probe, outs in rows[:MAX_PROBES]:
            t.probes.append(probe)
            for c in probed:
                t.outputs.setdefault(c, []).append(outs[c])
    finally:
        for h in handles.values():
            lib.host.unload(h)
    return t


def match_examples(
    lib, cell_ids: list[str], examples: list[tuple[list[int], int]]
) -> list[str]:
    """Rung 1 scoped to a candidate set: the cells that reproduce **every** example —
    pure execution, no model. An empty result means the behaviour isn't in the set."""
    out = []
    for cid in cell_ids:
        m = lib.inspect(cid)
        if m is None or m.get("state"):
            continue
        h = lib.host.load(cid)
        try:
            if all(_run_cell(lib, h, list(a)) == e for a, e in examples):
                out.append(cid)
        finally:
            lib.host.unload(h)
    return out


# ── the A/B: does behaviour beat manifests for the escalated residue? ──────────────

SYSTEM_PROMPT = (
    "You pick the single best tool ('cell') for a request from a short candidate "
    "list. Reply with exactly one line: CELL: <id>."
)

def _manifest_lines(lib, ids: list[str]) -> str:
    lines = []
    for cid in ids:
        m = lib.inspect(cid) or {}
        tags = ",".join(m.get("tags", []))
        lines.append(f"- {cid}: {m.get('summary', '')} [{tags}]")
    return "\n".join(lines)


@dataclass
class Pick:
    query: str
    category: str
    expected: list[str]
    candidates: list[str]
    manifests_pick: str | None = None
    evidence_pick: str | None = None


@dataclass
class DisambiguationReport:
    model: str
    theta: float
    embed_model: str
    picks: list[Pick] = field(default_factory=list)

    def split(self, category: str) -> dict:
        rows = [p for p in self.picks if p.category == category]
        n = len(rows)
        return {
            "n": n,
            "manifests_only": sum(p.manifests_pick in p.expected for p in rows) / n
            if n
            else 0.0,
            "with_probes": sum(p.evidence_pick in p.expected for p in rows) / n
            if n
            else 0.0,
        }

    def as_dict(self) -> dict:
        cats = []
        for p in self.picks:
            if p.category not in cats:
                cats.append(p.category)
        return {
            "model": self.model,
            "theta": self.theta,
            "embed_model": self.embed_model,
            "splits": {c: self.split(c) for c in cats},
        }


def _ask(client, cfg, prompt: str) -> str | None:
    try:
        resp = client.chat.completions.create(
            model=cfg.model,
            temperature=cfg.temperature,
            messages=[
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": prompt},
            ],
        )
        text = resp.choices[0].message.content or ""
    except Exception:
        return None
    for line in reversed(text.strip().splitlines()):
        if "CELL:" in line.upper():
            return line.split(":", 1)[1].strip().strip("`").split()[0]
    return None


def run_disambiguation(
    dataset: str = "retrieval",
    library_dir: str | None = None,
    model: str | None = None,
    embed_model: str | None = None,
    k: int = 5,
    client=None,
    cfg: AgentConfig | None = None,
) -> DisambiguationReport:
    """The tier-3 A/B over the **escalated residue**: run the tiers, take every query
    the gate declined (with the right answer present in the top-k, so the pick is
    possible), and ask the model twice — manifests only, then manifests + the executed
    probe table. The lift is tier 3's contribution."""
    from . import tiers as _t

    emb = embed_model or _t.RECOMMENDED_EMBED_MODEL
    report_t = _t.run_tiers(dataset=dataset, library_dir=library_dir, embed_model=emb, k=k)
    lib = open_library(library_dir)
    cfg = cfg or AgentConfig.from_env(model)
    client = client or make_client(cfg)
    out = DisambiguationReport(model=cfg.model, theta=report_t.theta, embed_model=emb)

    for d in report_t.decisions:
        if d.answered(report_t.theta):
            continue  # tier 2 already answered it
        ids = [cid for _, cid in d.top[:k]]
        if not any(e in ids for e in d.expected):
            continue  # unpickable — a retrieval miss, not a disambiguation case
        pick = Pick(d.query, d.category, d.expected, ids)
        base = f"Request: {d.query}\n\nCandidates:\n{_manifest_lines(lib, ids)}"
        pick.manifests_pick = _ask(client, cfg, base)
        evidence = probe_table(lib, ids)
        pick.evidence_pick = _ask(client, cfg, f"{base}\n\n{evidence.render()}")
        out.picks.append(pick)
    return out
