"""Agent-authored-example retrieval — the end-to-end lane the oracle numbers don't cover.

Checkpoints 21/22 measure ORACLE-EQUIPPED retrieval: examples derived from the target
cell, i.e. the router's discrimination capacity given correct probes. This eval measures
the deployed workflow instead:

    natural-language request
        → a model authors 1–3 input→output examples (never shown the target cell)
        → fused retrieval (behaviour ranks, text breaks ties)
        → selected cell

and tracks the quantities that decide whether the example lane is real end-to-end:

* **validity** — does the *expected* cell actually reproduce the authored examples?
  (The model must compute outputs itself; a wrong output is a wrong test.)
* **informativeness** — how many library cells co-match the examples (the behavioural
  equivalence class the probes actually pin down)?
* **retrieval** — fused P@1 with the authored examples, next to plain-text P@1 and the
  oracle-equipped P@1 on the same cases.
* **the dangerous failure** — `false_unique`: the examples pin a SINGLETON behavioural
  class that is not the expected cell (confidently wrong, usually from an invalid
  example another cell happens to satisfy). `false_unique_rate` should be ~0; ambiguity
  (expected inside a >1 class) is honest and recoverable, false uniqueness is not.

Population honesty: only cases whose expected cell is a **value cell of arity 1–3** are
authorable schema-free — a state cell's field names aren't knowable from the request
alone (the deployed flow for those is search → inspect → author field examples; that
two-step lane is registered as the extension). The report carries the population
fraction so the headline can't quietly cherry-pick.

The model is told the arity: that is caller-side intent (you know how many arguments
the function you want takes), not target-cell leakage.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from .agent import AgentConfig, make_client
from .datasets import load_jsonl
from .library import open_library
from .metrics import Aggregate, aggregate, best_rank
from .potion import _extract_json
from .retrieval import _acceptable

SYSTEM_PROMPT = (
    "You write test examples for a described integer function. Reply with JSON only, "
    'no prose: {"examples": [{"in": [..], "out": N}, ...]} with 1 to 3 examples. '
    "All inputs and outputs are unsigned 16-bit integers (0..65535). Only use inputs "
    "whose output you can compute EXACTLY from the description. Prefer inputs that "
    "distinguish this function from similar ones (unequal arguments where order might "
    "matter, values where off-by-one behaviour shows)."
)


def author_prompt(query: str, arity: int) -> str:
    args = "argument" if arity == 1 else "arguments"
    return (
        f"The function: {query}\n"
        f"It takes {arity} unsigned 16-bit integer {args}.\n"
        f"Give 1-3 input→output examples as JSON."
    )


def parse_examples_reply(reply: str | None, arity: int) -> list[tuple[list[int], int]] | None:
    """The model's reply → validated `(inputs, out)` pairs, or None if unusable.
    Well-formed means: parseable JSON, 1–3 examples, correct arity, u16 range."""
    data = _extract_json(reply)
    if not data or not isinstance(data.get("examples"), list):
        return None
    out: list[tuple[list[int], int]] = []
    for e in data["examples"][:3]:
        if not isinstance(e, dict):
            return None
        ins, want = e.get("in"), e.get("out")
        if (
            not isinstance(ins, list)
            or len(ins) != arity
            or not all(isinstance(v, int) and 0 <= v <= 0xFFFF for v in ins)
            or not isinstance(want, int)
            or not 0 <= want <= 0xFFFF
        ):
            return None
        out.append(([int(v) for v in ins], int(want)))
    return out or None


class BehaviourTable:
    """Hit counts for authored examples over every register-driveable value cell —
    the behavioural equivalence machinery. Warm handles, one load per cell."""

    def __init__(self, lib) -> None:
        self.lib = lib
        self.cells: list[tuple[str, int]] = []  # (id, arity)
        for m in lib.list():
            if m.get("state"):
                continue
            params = m.get("params") or []
            if 1 <= len(params) <= 3:
                self.cells.append((m["id"], len(params)))
        self.cells.sort()
        self._handles: dict[str, int] = {}

    def _run(self, cid: str, args: list[int]) -> int | None:
        h = self._handles.get(cid)
        if h is None:
            h = self.lib.host.load(cid)
            self._handles[cid] = h
        try:
            rep = self.lib.host.run(h, list(args))
            return int(rep["result"]) if rep.get("halt") == "returned" else None
        except ValueError:
            return None

    def close(self) -> None:
        for h in self._handles.values():
            try:
                self.lib.host.unload(h)
            except ValueError:
                pass
        self._handles.clear()

    def equivalence(
        self, examples: list[tuple[list[int], int]], arity: int
    ) -> tuple[list[str], int]:
        """(top behavioural class, max hit count): the cells sharing the best
        `(hits, arity-match)` key — mirroring the fused ranking's behavioural tiers.
        Empty class means no cell reproduces anything."""
        best_key: tuple[int, int] | None = None
        classes: dict[tuple[int, int], list[str]] = {}
        for cid, cell_arity in self.cells:
            hits = sum(
                1 for args, want in examples if self._run(cid, args) == want
            )
            if hits == 0:
                continue
            key = (hits, 1 if cell_arity == arity else 0)
            classes.setdefault(key, []).append(cid)
            if best_key is None or key > best_key:
                best_key = key
        if best_key is None:
            return [], 0
        return sorted(classes[best_key]), best_key[0]


@dataclass
class AuthoredCase:
    case_id: str
    query: str
    expected: list[str]
    category: str
    arity: int
    well_formed: bool
    examples: list[tuple[list[int], int]] = field(default_factory=list)
    valid: bool = False  # expected cell reproduces EVERY authored example
    top_class: list[str] = field(default_factory=list)
    plain_rank: int | None = None
    oracle_rank: int | None = None  # fused with the sidecar's oracle examples
    authored_rank: int | None = None  # fused with the model's examples

    @property
    def expected_in_top_class(self) -> bool:
        return any(e in self.top_class for e in self.expected)

    @property
    def false_unique(self) -> bool:
        return len(self.top_class) == 1 and not self.expected_in_top_class

    @property
    def ambiguous(self) -> bool:
        return len(self.top_class) > 1 and self.expected_in_top_class

    def as_dict(self) -> dict:
        return {
            "case_id": self.case_id,
            "category": self.category,
            "well_formed": self.well_formed,
            "examples": [{"in": i, "out": o} for i, o in self.examples],
            "valid": self.valid,
            "top_class": self.top_class,
            "expected_in_top_class": self.expected_in_top_class,
            "false_unique": self.false_unique,
            "plain_rank": self.plain_rank,
            "oracle_rank": self.oracle_rank,
            "authored_rank": self.authored_rank,
        }


@dataclass
class AuthoredReport:
    model: str
    library: str
    k: int
    population: int  # authorable cases (value cells, arity 1-3) in the dataset slice
    total_cases: int  # all cases in the dataset slice, for the population fraction
    cases: list[AuthoredCase] = field(default_factory=list)

    def _ranks(self, attr: str, cat: str | None = None) -> Aggregate:
        rows = [c for c in self.cases if cat is None or c.category == cat]
        return aggregate([getattr(c, attr) for c in rows])

    def rate(self, pred, cat: str | None = None) -> float:
        rows = [c for c in self.cases if cat is None or c.category == cat]
        return sum(1 for c in rows if pred(c)) / len(rows) if rows else 0.0

    def as_dict(self) -> dict:
        cats = sorted({c.category for c in self.cases})

        def split(cat: str | None) -> dict:
            return {
                "n": len([c for c in self.cases if cat is None or c.category == cat]),
                "well_formed": round(self.rate(lambda c: c.well_formed, cat), 4),
                "valid": round(self.rate(lambda c: c.valid, cat), 4),
                "false_unique_rate": round(self.rate(lambda c: c.false_unique, cat), 4),
                "ambiguity_rate": round(self.rate(lambda c: c.ambiguous, cat), 4),
                "plain": self._ranks("plain_rank", cat).as_dict(),
                "oracle": self._ranks("oracle_rank", cat).as_dict(),
                "authored": self._ranks("authored_rank", cat).as_dict(),
                # The dangerous correlation: invalid examples AND a wrong top-1.
                "correlated_failure": round(
                    self.rate(lambda c: not c.valid and c.authored_rank != 1, cat), 4
                ),
            }

        return {
            "eval": "authored-examples",
            "model": self.model,
            "library": self.library,
            "k": self.k,
            "population": self.population,
            "total_cases": self.total_cases,
            "population_fraction": round(self.population / self.total_cases, 4)
            if self.total_cases
            else 0.0,
            "overall": split(None),
            "by_category": {c: split(c) for c in cats},
            "cases": [c.as_dict() for c in self.cases],
        }


def _ask(client, cfg, query: str, arity: int) -> str | None:
    try:
        resp = client.chat.completions.create(
            model=cfg.model,
            temperature=cfg.temperature,
            messages=[
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": author_prompt(query, arity)},
            ],
        )
        return resp.choices[0].message.content
    except Exception:
        return None  # endpoint errors are per-case data (an unusable authoring turn)


def run_authored(
    dataset: str = "retrieval",
    examples: str = "retrieval-examples",
    library_dir: str | None = None,
    model: str | None = None,
    client=None,
    cfg: AgentConfig | None = None,
    k: int = 5,
    max_cases: int | None = None,
    category: str | None = None,
) -> AuthoredReport:
    """The full loop over every authorable case (value cells, arity 1–3). `client`
    is injectable for offline tests; `examples` names the oracle sidecar used for
    the per-case oracle comparison column."""
    cfg = cfg or AgentConfig.from_env(model)
    client = client or make_client(cfg)
    lib = open_library(library_dir)
    sidecar = {row["id"]: row for row in load_jsonl(examples)}
    table = BehaviourTable(lib)

    cases = load_jsonl(dataset)
    if category:
        cases = [c for c in cases if c.get("category") == category]
    total = len(cases)

    report = AuthoredReport(
        model=cfg.model, library=str(library_dir or "seed"), k=k,
        population=0, total_cases=total,
    )
    arities = {m["id"]: len(m.get("params") or []) for m in lib.list() if not m.get("state")}
    try:
        for case in cases:
            expected = _acceptable(case)
            arity = arities.get(expected[0])
            if arity is None or not 1 <= arity <= 3:
                continue  # not schema-free-authorable; counted via population fraction
            report.population += 1
            if max_cases is not None and len(report.cases) >= max_cases:
                continue  # keep counting the population, stop asking the model
            query = case["query"]
            case_id = str(case.get("id", query))

            reply = _ask(client, cfg, query, arity)
            parsed = parse_examples_reply(reply, arity)
            row = AuthoredCase(
                case_id=case_id, query=query, expected=expected,
                category=case.get("category", "uncategorized"),
                arity=arity, well_formed=parsed is not None,
                examples=parsed or [],
            )
            row.plain_rank = best_rank(
                [m["id"] for m in lib.search(query, k)], set(expected)
            )
            oracle = sidecar.get(case_id)
            if oracle:
                row.oracle_rank = best_rank(
                    [m["id"] for m in lib.search(query, k, examples=oracle["examples"])],
                    set(expected),
                )
            if parsed:
                row.valid = all(
                    table._run(expected[0], args) == want for args, want in parsed
                )
                row.top_class, _ = table.equivalence(parsed, arity)
                fused = lib.search(
                    query, k, examples=[{"in": i, "out": o} for i, o in parsed]
                )
                row.authored_rank = best_rank([m["id"] for m in fused], set(expected))
            report.cases.append(row)
    finally:
        table.close()
    return report
