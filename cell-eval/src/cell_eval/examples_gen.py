"""Generate the example-equipped retrieval sidecar (WS-F / F2).

For each `retrieval.jsonl` case, derive up to [`MAX_EXAMPLES`] I/O examples a
competent user could plausibly type, by executing the case's expected cell on a
fixed human-typable input battery and greedily keeping the inputs that eliminate
the most same-shape co-matching siblings. The sidecar
(`datasets/retrieval-examples.jsonl`) is keyed by case id — `retrieval.jsonl`
itself is never edited (the CI canary discipline).

**Circularity, stated plainly:** the expected cell reproduces its own examples
*by construction*, so an equipped hit is not "does expected match" — the honest
findings are (a) which same-shape siblings ALSO reproduce every example (each
row records the survivors in `co_match`), and (b) whether the fused ranker
(behaviour first, text order breaking ties) still puts the expected cell at #1.
The plausibility cap — a fixed small battery, ≤3 examples, no adversarial
mining of the u16 cube — keeps the examples honest: nothing a user couldn't
write from knowing the behaviour they want.

Sibling model: candidates that could co-match are the cells the fused matcher
could actually confuse. For value-form examples that is **every value cell**
regardless of its own arity — the VM probes uniformly (extra register args are
ignored, missing ones are 0), so `midrange3` genuinely reproduces `(9,4)→4` as
`(9,4,0)`; the greedy selector must see that to defend against it. For
field-form examples it is the state cells with the *same field-name layout* (a
field example drives fields by name, so a same-count sibling with different
names errors out and never co-matches; the real collision class is same-layout
families like `smag_*`). State cells register-probed by a value example can in
principle return coincidentally; that residue is accepted and left to the text
tiebreak rather than modeled here. Status-flag families need `expect`
(post-run field values): `smag_add`/`smag_sub` both return 1 on every valid
input — the answer lives in `mag`/`neg`. `expect` is emitted only when the
return alone leaves co-matchers, so examples stay as simple as an honest user
would write them.

Deterministic by construction: fixed batteries, sorted iteration, greedy ties
broken by battery order, no timestamps — re-running must be diff-clean.
Cells with non-scalar-driveable surfaces (f32 / byte-buffer fields, arity 0 or
>3 value cells) are recorded as skipped, not guessed at.
"""

from __future__ import annotations

import json
import pathlib
from dataclasses import dataclass, field

from .datasets import DATASETS_DIR, load_jsonl
from .library import open_library
from .tier3 import BATTERY

MAX_EXAMPLES = 3
# Types a REGISTER example can drive (value cells; buffer I/O arrives with
# Phase S3). Signed types are register-encodable — `min_i16` reads the same u16
# battery as `min`, which is exactly why they collide and must be in the pool.
_SCALAR_TYPES = {"u8", "u16", "u32", "i8", "i16", "i32"}
# State cells additionally drive f32 fields (Finance80): cells are deterministic
# softfloat, so an f32 example matches BIT-EXACTLY — the sidecar carries the bit
# pattern (u64), while the information content is an ordinary decimal a user
# would type (rate=0.05); surfaces convert. Value-cell f32 params stay excluded
# (the register path is u16).
_STATE_TYPES = _SCALAR_TYPES | {"f32"}

# The state-cell input pool: 0/1-heavy so validity-limited flag fields
# (`neg_a`/`neg_b` must be 0|1 or the cell escalates) land legal values often,
# with small distinct magnitudes so arithmetic families actually disagree.
_STATE_POOL = [3, 0, 7, 1, 9, 0, 4, 1, 12, 0, 100, 1, 2, 0, 5, 1]
# The f32 field pool: plausible finance-shaped decimals (rates, prices, flags'
# neighbours), kept small and distinct so families disagree; 0.0 for identity
# edges. Stored as bits at row-build time.
_FLOAT_POOL = [0.05, 100.0, 0.5, 1.0, 0.0, 2.0, 10.0, 0.25, 1.5, 3.0]


def _f32_bits(v: float) -> int:
    import struct

    return struct.unpack("<I", struct.pack("<f", v))[0]


def _state_rows(types: list[str]) -> list[tuple[int, ...]]:
    """Deterministic candidate input rows for a state cell, typed per field:
    integer fields rotate the 0/1-heavy pool (byte-identical to the pre-f32
    generator for all-integer layouts), f32 fields rotate the decimal pool as
    bit patterns."""
    k = len(types)
    flt = [i for i, t in enumerate(types) if t == "f32"]

    def val(i: int, j: int) -> int:
        if types[i] == "f32":
            # Index by position among the float fields so layouts stay stable.
            fi = flt.index(i)
            return _f32_bits(_FLOAT_POOL[(fi + j) % len(_FLOAT_POOL)])
        return _STATE_POOL[(i + j) % len(_STATE_POOL)]

    def flat(i: int, intval: int, fltval: float) -> int:
        return _f32_bits(fltval) if types[i] == "f32" else intval

    rows: list[tuple[int, ...]] = []
    for j in range(12):
        rows.append(tuple(val(i, j) for i in range(k)))
    rows.append(tuple(flat(i, 0, 0.0) for i in range(k)))
    rows.append(tuple(flat(i, 1, 1.0) for i in range(k)))
    rows.append(tuple(flat(i, i % 2, 0.5) for i in range(k)))
    rows.append(tuple(flat(i, (i + 1) % 2, 0.05) for i in range(k)))
    rows.append(tuple(flat(i, 9 if i % 2 == 0 else 0, 100.0) for i in range(k)))
    rows.append(tuple(flat(i, 4 if i % 2 == 0 else 1, 2.0) for i in range(k)))
    rows.append(tuple(flat(i, 2 if i % 2 == 0 else 0, 10.0) for i in range(k)))
    seen: set[tuple[int, ...]] = set()
    out = []
    for r in rows:
        if r not in seen:
            seen.add(r)
            out.append(r)
    return out


@dataclass
class GenStats:
    equipped: int = 0
    unequipped: dict[str, int] = field(default_factory=dict)  # reason -> count
    with_expect: int = 0
    ambiguous: int = 0  # equipped rows with residual co_match

    def skip(self, reason: str) -> None:
        self.unequipped[reason] = self.unequipped.get(reason, 0) + 1


def _clean(rep: dict) -> bool:
    return rep.get("halt") == "returned"


class _Tables:
    """Memoized execution tables per confusable group, built lazily.

    Value group key: ("v", arity) — battery rows run positionally.
    State group key: ("s", field-name tuple) — rows assigned to fields by
    declaration order, output-field slots zeroed (an omitted field is 0 at run
    time, so the emitted example only names the fields a user would set).
    """

    def __init__(self, lib) -> None:
        self.lib = lib
        self._members: dict[tuple, list[str]] = {}
        self._value_cells: list[str] = []  # every register-driveable value cell
        self._value_outs: dict[int, dict[str, list[int | None]]] = {}
        self._state_runs: dict[tuple, dict[str, list[tuple[int, dict] | None]]] = {}
        self._state_out_fields: dict[tuple, list[str]] = {}
        self._state_inputs: dict[tuple, list[dict[str, int]]] = {}
        for m in lib.list():
            key = self._key(m)
            if key is not None:
                self._members.setdefault(key, []).append(m["id"])
            if not (m.get("state") or []) and not any(
                ty not in _SCALAR_TYPES for _, ty in (m.get("params") or [])
            ):
                self._value_cells.append(m["id"])
        for ids in self._members.values():
            ids.sort()
        self._value_cells.sort()

    def _key(self, m: dict) -> tuple | None:
        state = m.get("state") or []
        params = m.get("params") or []
        if state:
            if any(ty not in _STATE_TYPES for _, ty in state):
                return None
            # Same-layout means same names AND types: an f32 `rate` and a u16
            # `rate` are different example surfaces.
            return ("s", tuple((name, ty) for name, ty in state))
        if any(ty not in _SCALAR_TYPES for _, ty in params):
            return None
        if len(params) not in BATTERY:
            return None
        return ("v", len(params))

    def key_for(self, cell_id: str) -> tuple | None:
        m = self.lib.inspect(cell_id)
        return self._key(m) if m else None

    def members(self, key: tuple) -> list[str]:
        return self._members.get(key, [])

    # ── value cells ──────────────────────────────────────────────────────────
    def value_cells(self) -> list[str]:
        return self._value_cells

    def value_outputs(self, arity: int) -> dict[str, list[int | None]]:
        """`outs[cell][row_i]` = result on `BATTERY[arity][row_i]`, or None —
        for EVERY value cell, whatever its own arity: the VM probes uniformly
        (extra register args ignored, missing ones 0), so any value cell is a
        potential co-matcher for an `arity`-shaped example."""
        if arity not in self._value_outs:
            battery = BATTERY[arity]
            outs: dict[str, list[int | None]] = {}
            for cid in self._value_cells:
                h = self.lib.host.load(cid)
                try:
                    row = []
                    for args in battery:
                        try:
                            rep = self.lib.host.run(h, list(args))
                            row.append(int(rep["result"]) if _clean(rep) else None)
                        except ValueError:
                            row.append(None)
                    outs[cid] = row
                finally:
                    self.lib.host.unload(h)
            self._value_outs[arity] = outs
        return self._value_outs[arity]

    # ── state cells ──────────────────────────────────────────────────────────
    def _run_state_rows(
        self, cid: str, assignments: list[dict[str, int]]
    ) -> list[tuple[int, dict] | None]:
        h = self.lib.host.load(cid)
        try:
            out = []
            for fields in assignments:
                try:
                    rep = self.lib.host.run_state(h, dict(fields))
                    if _clean(rep):
                        out.append((int(rep["result"]), dict(rep["state"])))
                    else:
                        out.append(None)
                except ValueError:
                    out.append(None)
            return out
        finally:
            self.lib.host.unload(h)

    def state_tables(
        self, key: tuple
    ) -> tuple[list[dict[str, int]], dict[str, list[tuple[int, dict] | None]], list[str]]:
        """(input_rows, runs, output_fields) for a same-layout state group.

        Pass 1 assigns every field to find the fields any member writes
        (post != assigned); pass 2 re-runs with those output slots omitted
        (0 at reset), which is what a user-authored example looks like.
        """
        if key not in self._state_runs:
            names = [n for n, _ in key[1]]
            types = [t for _, t in key[1]]
            rows = _state_rows(types)
            full = [dict(zip(names, r)) for r in rows]
            pass1 = {cid: self._run_state_rows(cid, full) for cid in self.members(key)}
            out_fields = sorted(
                {
                    name
                    for cid, runs in sorted(pass1.items())
                    for fields, run in zip(full, runs)
                    if run is not None
                    for name, post in run[1].items()
                    if post != fields.get(name)
                }
            )
            inputs = []
            seen: set[tuple] = set()
            for fields in full:
                inp = {n: v for n, v in fields.items() if n not in out_fields}
                sig = tuple(sorted(inp.items()))
                if sig not in seen:
                    seen.add(sig)
                    inputs.append(inp)
            self._state_inputs[key] = inputs
            self._state_out_fields[key] = out_fields
            self._state_runs[key] = {
                cid: self._run_state_rows(cid, inputs) for cid in self.members(key)
            }
        return (
            self._state_inputs[key],
            self._state_runs[key],
            self._state_out_fields[key],
        )


def _greedy_select(
    clean_rows: list[int],
    siblings: list[str],
    matches: "callable[[str, int], bool]",
) -> tuple[list[int], list[str]]:
    """Greedily pick ≤MAX_EXAMPLES row indices, each eliminating the most
    still-co-matching siblings; ties (and the no-sibling case) resolve to the
    earliest battery row. A sibling co-matches iff it reproduces *every*
    selected row. Returns (selected rows, surviving co-matchers)."""
    selected: list[int] = []
    co = list(siblings)
    remaining = list(clean_rows)
    while remaining and len(selected) < MAX_EXAMPLES:
        # max() returns the first maximal element, so ties go to battery order.
        best = max(remaining, key=lambda r: sum(not matches(s, r) for s in co))
        kills = sum(not matches(s, best) for s in co)
        if selected and kills == 0:
            break  # nothing left that any candidate row can separate
        selected.append(best)
        remaining.remove(best)
        co = [s for s in co if matches(s, best)]
        if not co:
            break
    return selected, co


def generate(
    dataset: str = "retrieval",
    library_dir: str | None = None,
) -> tuple[list[dict], GenStats]:
    """Build sidecar rows for every case in `dataset`. Returns (rows, stats)."""
    lib = open_library(library_dir)
    cases = load_jsonl(dataset)
    tables = _Tables(lib)
    stats = GenStats()
    rows_out: list[dict] = []

    for case in cases:
        exp = case.get("expected")
        acceptable = [exp] if isinstance(exp, str) else list(exp or [])
        if not acceptable:
            stats.skip("no-expected")
            continue
        cid = acceptable[0]
        if lib.inspect(cid) is None:
            stats.skip("expected-not-in-library")
            continue
        key = tables.key_for(cid)
        if key is None:
            stats.skip("non-scalar-or-arity")
            continue
        if key[0] == "v":
            # Every value cell is a candidate co-matcher (uniform register probing).
            siblings = [s for s in tables.value_cells() if s not in acceptable]
            outs = tables.value_outputs(key[1])
            battery = BATTERY[key[1]]
            mine = outs[cid]
            clean = [i for i, o in enumerate(mine) if o is not None]
            if not clean:
                stats.skip("no-clean-runs")
                continue

            def match_v(s: str, r: int) -> bool:
                return outs[s][r] is not None and outs[s][r] == mine[r]

            sel, co = _greedy_select(clean, siblings, match_v)
            examples = [{"in": list(battery[r]), "out": mine[r]} for r in sel]
            form = "in"
        else:
            siblings = [s for s in tables.members(key) if s not in acceptable]
            inputs, runs, out_fields = tables.state_tables(key)
            mine_s = runs[cid]
            clean = [i for i, o in enumerate(mine_s) if o is not None]
            if not clean:
                stats.skip("no-clean-runs")
                continue

            def match_res(s: str, r: int) -> bool:
                run = runs[s][r]
                return run is not None and run[0] == mine_s[r][0]

            sel, co = _greedy_select(clean, siblings, match_res)
            expect_mode = bool(co) and bool(out_fields)
            if expect_mode:

                def match_full(s: str, r: int) -> bool:
                    run = runs[s][r]
                    return (
                        run is not None
                        and run[0] == mine_s[r][0]
                        and all(
                            run[1].get(f) == mine_s[r][1].get(f) for f in out_fields
                        )
                    )

                sel, co = _greedy_select(clean, siblings, match_full)
                stats.with_expect += 1
            examples = []
            for r in sel:
                ex: dict = {"fields": dict(inputs[r]), "out": mine_s[r][0]}
                if expect_mode:
                    ex["expect"] = {f: mine_s[r][1].get(f, 0) for f in out_fields}
                examples.append(ex)
            form = "fields"

        if not examples:
            stats.skip("no-examples-selected")
            continue
        stats.equipped += 1
        if co:
            stats.ambiguous += 1
        rows_out.append(
            {
                "id": str(case.get("id", case["query"])),
                "examples": examples,
                "co_match": sorted(co),
                "form": form,
            }
        )
    return rows_out, stats


def write_sidecar(
    dataset: str = "retrieval",
    out: str | pathlib.Path | None = None,
    library_dir: str | None = None,
) -> tuple[pathlib.Path, GenStats]:
    """Generate and write the sidecar JSONL (with a stats comment header)."""
    rows, stats = generate(dataset, library_dir)
    path = pathlib.Path(out) if out else DATASETS_DIR / "retrieval-examples.jsonl"
    lines = [
        "# Example-equipped retrieval sidecar (WS-F/F2) — generated by "
        "`cell-eval gen-examples`; do not hand-edit. Keyed by retrieval.jsonl case id.",
        f"# equipped={stats.equipped} with_expect={stats.with_expect} "
        f"residual_co_match={stats.ambiguous} "
        f"unequipped={json.dumps(stats.unequipped, sort_keys=True)}",
    ]
    lines += [json.dumps(r, sort_keys=True) for r in rows]
    path.write_text("\n".join(lines) + "\n")
    return path, stats
