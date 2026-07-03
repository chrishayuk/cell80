"""The repair-rate eval — is a rejected cell **plus the diagnostic** enough for a
one-shot fix?

This is how Phase 1.2 (the diagnostic rewrite) is *measured* rather than hoped: each
dataset row is a cell source that the compiler rejects, tagged with its diagnostic
class and the intended behavior (as I/O examples). The model gets exactly one shot —
the broken source and the compiler's error text, no tools, no retries — and the repair
counts only if the fixed source **compiles and reproduces the examples** (a fix that
compiles but breaks the semantics is a miss).

Per-class repair@1 is the report: a class whose diagnostic doesn't carry enough signal
for a one-shot fix is a diagnostic that needs rewriting. Record a baseline before
changing error text; regressions show up as a class dropping.

The *steering* below is held fixed (like the adoption eval): the deliberately thin
system prompt gives the dialect one breath of context — the repair signal must come
from the **error message**, because that's the thing 1.2 claims to have improved.

Gated: nothing imports `openai` or `cell80_py` until `run_repair` is called.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field

from .agent import AgentConfig, make_client
from .datasets import load_jsonl

# ── steering (HOLD THIS FIXED; the measured variable is the compiler error) ────────
SYSTEM_PROMPT = (
    "You repair small functions written in a restricted Rust dialect (a real Rust "
    "subset: u8/u16/u32/i16/bool, arithmetic, comparisons, if/match — including as "
    "values — while/for loops, at most 3 fn parameters; no strings, floats, closures, "
    "recursion, or Result/Option). You will get a rejected source and the compiler's "
    "error. Follow the error's guidance. Reply with ONLY the corrected source in a "
    "```rust code block — no commentary."
)

USER_PROMPT = (
    "This cell was rejected by the compiler.\n\n"
    "Intended behavior: {intent}\n\n"
    "SOURCE:\n```rust\n{src}\n```\n\n"
    "COMPILER ERROR:\n{error}\n\n"
    "Fix the source. Reply with ONLY the corrected Rust in a ```rust block."
)

_CODE_RE = re.compile(r"```(?:rust)?\s*\n(.*?)```", re.DOTALL)

DEFAULT_CYCLES = 2_000_000


@dataclass
class RepairResult:
    id: str
    klass: str
    error: str
    repaired_src: str | None = None
    compiled: bool = False
    correct: bool = False
    note: str = ""


@dataclass
class ClassStats:
    n: int = 0
    compiled: int = 0
    correct: int = 0

    @property
    def repair_at_1(self) -> float:
        return self.correct / self.n if self.n else 0.0


@dataclass
class RepairReport:
    model: str
    results: list[RepairResult] = field(default_factory=list)

    @property
    def by_class(self) -> dict[str, ClassStats]:
        out: dict[str, ClassStats] = {}
        for r in self.results:
            s = out.setdefault(r.klass, ClassStats())
            s.n += 1
            s.compiled += int(r.compiled)
            s.correct += int(r.correct)
        return out

    @property
    def overall(self) -> ClassStats:
        s = ClassStats()
        for r in self.results:
            s.n += 1
            s.compiled += int(r.compiled)
            s.correct += int(r.correct)
        return s

    def as_dict(self) -> dict:
        return {
            "model": self.model,
            "overall": {
                "n": self.overall.n,
                "compiled": self.overall.compiled,
                "repair_at_1": self.overall.repair_at_1,
            },
            "by_class": {
                k: {"n": s.n, "compiled": s.compiled, "repair_at_1": s.repair_at_1}
                for k, s in sorted(self.by_class.items())
            },
            "results": [
                {
                    "id": r.id,
                    "class": r.klass,
                    "compiled": r.compiled,
                    "correct": r.correct,
                    "note": r.note,
                }
                for r in self.results
            ],
        }


def try_compile(src: str) -> str | None:
    """Compile `src` as a sandboxed cell; `None` on success, the diagnostic on failure."""
    import cell80_py

    host = cell80_py.CellHost()
    try:
        host.add_source("probe", src)
        return None
    except ValueError as e:
        return str(e)


def run_examples(src: str, examples: list) -> bool:
    """Compile + run `src` on every `[args, expected]` example (positional value cell)."""
    import cell80_py

    host = cell80_py.CellHost()
    try:
        host.add_source("probe", src)
        h = host.load("probe")
        for args, expected in examples:
            rep = host.run(h, [int(a) for a in args], DEFAULT_CYCLES)
            if rep["halt"] != "returned" or int(rep["result"]) != int(expected):
                return False
        return True
    except ValueError:
        return False


def extract_source(reply: str | None) -> str | None:
    """The repaired source: the last ```rust block, or the raw reply if it looks bare."""
    if not reply:
        return None
    blocks = _CODE_RE.findall(reply)
    if blocks:
        return blocks[-1].strip()
    return reply.strip() if reply.strip().startswith("fn ") else None


def run_repair(
    dataset: str,
    model: str | None = None,
    client=None,
    cfg: AgentConfig | None = None,
) -> RepairReport:
    """One-shot repair over every dataset row. `client` is injectable for offline tests."""
    cfg = cfg or AgentConfig.from_env(model)
    client = client or make_client(cfg)
    report = RepairReport(model=cfg.model)

    for row in load_jsonl(dataset):
        error = try_compile(row["src"])
        res = RepairResult(id=row["id"], klass=row["klass"], error=error or "")
        if error is None:
            # The dialect grew and this row now compiles — the probe retires.
            res.note = "row compiles unrepaired (dialect grew?) — skipped"
            report.results.append(res)
            continue
        prompt = USER_PROMPT.format(intent=row["intent"], src=row["src"], error=error)
        try:
            resp = client.chat.completions.create(
                model=cfg.model,
                temperature=cfg.temperature,
                messages=[
                    {"role": "system", "content": SYSTEM_PROMPT},
                    {"role": "user", "content": prompt},
                ],
            )
            reply = resp.choices[0].message.content
        except Exception as e:  # endpoint/network problems are per-row data, not crashes
            res.note = f"endpoint error: {e}"
            report.results.append(res)
            continue
        fixed = extract_source(reply)
        if fixed is None:
            res.note = "no code block in the reply"
            report.results.append(res)
            continue
        res.repaired_src = fixed
        res.compiled = try_compile(fixed) is None
        if res.compiled:
            res.correct = run_examples(fixed, row["examples"])
            if not res.correct:
                res.note = "compiles but breaks the intended behavior"
        else:
            res.note = "repaired source still rejected"
        report.results.append(res)
    return report
