"""cell-potion corpus regeneration — `cell-eval potion-pairs`.

The cell-potion experiment (docs/cell-potion-training-spec.md) **earned in** and its
run is banked: corpus `datasets/potion-train-pairs.jsonl`, trainer + protocol under
`cell-eval/potion/`. The banked corpus was authored by session agents (PROTOCOL.md);
this module is the *committed, mechanical* regeneration path for when the library
grows — new cells need new training rows before `potion/train.py` can rebuild the
rung-2 artifact (the spec: "regenerate training pairs for new cells but do not touch
the eval rows").

It reproduces the banked corpus protocol exactly:

* per cell: 8 `paraphrase` rows (avoid the cell's id/tag vocabulary,
  `hard_negatives` empty), 4 `adversarial` rows (each skirting one **named**
  confusable, recorded first in `hard_negatives`), and 1 mechanical `direct`
  anchor (the manifest summary verbatim);
* confusables: top-4 cosine neighbours over the harness manifest-doc text under a
  neutral embedder (`ollama:nomic-embed-text`, same as the banked run);
* authored **from manifest text only** — this module never reads
  `datasets/retrieval.jsonl`. Decontamination is not this module's job: run the
  pre-registered audit (`cell-eval/potion/audit_overlap.py`) on the output to
  produce the `.clean.jsonl` the trainer consumes.

Validation before writing: every cell at exactly the requested counts, every
cell/negative id resolves, zero duplicate queries — the same checks the banked
corpus passed.
"""

from __future__ import annotations

import json
import pathlib

from .agent import AgentConfig, make_client
from .datasets import DATASETS_DIR
from .library import open_library
from .tiers import _doc

PAIRS_PATH = DATASETS_DIR / "potion-train-pairs.jsonl"
NEUTRAL_EMBED = "ollama:nomic-embed-text"  # confusable map only — training input
N_PARAPHRASE = 8
N_ADVERSARIAL = 4  # one per named confusable
RETRIES = 2

PAIRGEN_SYSTEM = (
    "You author search queries for a library of tiny integer utility functions "
    "('cells'). You are given one cell's manifest and the manifests of its nearest "
    "neighbours in the library. Reply with JSON only — no prose, no code fences."
)


def _pairgen_prompt(m: dict, neighbours: list[dict], n_para: int) -> str:
    """The authoring prompt — manifest fields only (id, summary, tags, signature).
    Nothing from the eval set is available to leak."""
    avoid = sorted(set(m["id"].split("_")) | set(m.get("tags", [])))
    nl = "\n".join(
        f"  - {n['id']}: {n.get('summary', '')} (tags: {', '.join(n.get('tags', []))})"
        for n in neighbours
    )
    neigh_ids = ", ".join(n["id"] for n in neighbours)
    return (
        f"The cell:\n"
        f"  id: {m['id']}\n"
        f"  summary: {m.get('summary', '')}\n"
        f"  tags: {', '.join(m.get('tags', []))}\n"
        f"  signature: {m.get('signature', '')}\n\n"
        f"Its nearest neighbours in the library (easily confused with it):\n{nl}\n\n"
        f"Author search queries a user might type when they need exactly this cell:\n"
        f'1. "paraphrase": {n_para} natural rewordings that AVOID all of these words: '
        f"{', '.join(avoid)}. Everyday task language, not library vocabulary. Vary "
        f"register and sentence shape.\n"
        f'2. "adversarial": for EACH neighbour ({neigh_ids}), exactly one query that '
        f"still asks for THIS cell's behaviour but is phrased to skirt that neighbour "
        f"— borrow its framing or vocabulary while the actual need remains this "
        f"cell's.\n\n"
        f'Reply with exactly: {{"paraphrase": ["..."], '
        f'"adversarial": [{{"query": "...", "skirts": "<neighbour id>"}}]}}'
    )


def _extract_json(reply: str | None) -> dict | None:
    """Pull the first JSON object out of a model reply (tolerates fences/prose)."""
    if not reply:
        return None
    s = reply.find("{")
    if s < 0:
        return None
    depth = 0
    for i in range(s, len(reply)):
        if reply[i] == "{":
            depth += 1
        elif reply[i] == "}":
            depth -= 1
            if depth == 0:
                try:
                    return json.loads(reply[s : i + 1])
                except json.JSONDecodeError:
                    return None
    return None


def confusable_map(lib, embed_model: str = NEUTRAL_EMBED, k: int = 4) -> dict[str, list[str]]:
    """Top-k cosine neighbours per cell over the manifest-doc text under a neutral
    embedder — the banked run's confusable notion (PROTOCOL.md). Training input
    only; the eval judge is untouched."""
    import numpy as np

    from .tiers import Embedder

    mans = sorted(lib.list(), key=lambda m: m["id"])
    ids = [m["id"] for m in mans]
    vecs = Embedder(embed_model).encode([_doc(m) for m in mans])
    sims = np.asarray(vecs) @ np.asarray(vecs).T
    out = {}
    for i, cid in enumerate(ids):
        order = np.argsort(-sims[i])
        out[cid] = [ids[j] for j in order if j != i][:k]
    return out


def _author_cell(client, cfg, m: dict, neighbours: list[dict], n_para: int) -> list[dict] | None:
    """One cell's rows, validated to the banked shape — or None if the model's reply
    doesn't satisfy the counts after parsing."""
    resp = client.chat.completions.create(
        model=cfg.model,
        temperature=cfg.temperature,
        messages=[
            {"role": "system", "content": PAIRGEN_SYSTEM},
            {"role": "user", "content": _pairgen_prompt(m, neighbours, n_para)},
        ],
    )
    data = _extract_json(resp.choices[0].message.content)
    if not data:
        return None
    neigh_ids = {n["id"] for n in neighbours}
    paras = [q.strip() for q in data.get("paraphrase", []) if isinstance(q, str) and q.strip()]
    advs = []
    for a in data.get("adversarial", []):
        if (
            isinstance(a, dict)
            and isinstance(a.get("query"), str)
            and a["query"].strip()
            and a.get("skirts") in neigh_ids
        ):
            advs.append({"query": a["query"].strip(), "skirts": a["skirts"]})
    if len(paras) < n_para or {a["skirts"] for a in advs} != neigh_ids:
        return None
    rows = [
        {"cell": m["id"], "kind": "paraphrase", "query": q, "hard_negatives": []}
        for q in paras[:n_para]
    ]
    rows += [
        {"cell": m["id"], "kind": "adversarial", "query": a["query"], "hard_negatives": [a["skirts"]]}
        for a in advs
    ]
    rows.append(
        {"cell": m["id"], "kind": "direct", "query": m.get("summary", m["id"]), "hard_negatives": []}
    )
    return rows


def validate_corpus(rows: list[dict], cell_ids: set[str]) -> list[str]:
    """The banked corpus's admission checks; returns problems (empty = valid)."""
    problems = []
    seen: dict[str, str] = {}
    for r in rows:
        if r["cell"] not in cell_ids:
            problems.append(f"unknown cell {r['cell']!r}")
        for n in r.get("hard_negatives", []):
            if n not in cell_ids:
                problems.append(f"{r['cell']}: unknown hard negative {n!r}")
        q = " ".join(r["query"].lower().split())
        if q in seen and seen[q] != r["cell"]:
            problems.append(f"duplicate query across cells: {r['query']!r}")
        seen[q] = r["cell"]
    return problems


def generate_pairs(
    model: str | None = None,
    library_dir: str | None = None,
    n_para: int = N_PARAPHRASE,
    n_adv: int = N_ADVERSARIAL,
    neighbours_embed: str = NEUTRAL_EMBED,
    client=None,
    temperature: float = 0.8,
    only_cells: list[str] | None = None,
) -> tuple[list[dict], dict]:
    """Author corpus rows for every cell (or `only_cells` — the library-growth case:
    regenerate for new cells without touching existing rows). Returns (rows, stats)."""
    lib = open_library(library_dir)
    cfg = AgentConfig.from_env(model)
    cfg.temperature = temperature
    client = client or make_client(cfg)

    confusables = confusable_map(lib, neighbours_embed, k=n_adv)
    cell_ids = sorted(confusables)
    targets = [c for c in cell_ids if only_cells is None or c in set(only_cells)]

    rows: list[dict] = []
    failures: list[str] = []
    for cid in targets:
        m = lib.inspect(cid)
        neigh = [lib.inspect(n) for n in confusables[cid]]
        got = None
        for _ in range(RETRIES + 1):
            got = _author_cell(client, cfg, m, neigh, n_para)
            if got:
                break
        if got:
            rows.extend(got)
        else:
            failures.append(cid)

    problems = validate_corpus(rows, set(cell_ids))
    stats = {
        "cells": len(targets),
        "rows": len(rows),
        "failed_cells": failures,
        "validation_problems": problems,
        "model": cfg.model,
        "neighbours_embed": neighbours_embed,
    }
    return rows, stats


def write_pairs(rows: list[dict], stats: dict, path: pathlib.Path = PAIRS_PATH) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    header = (
        "# cell-potion TRAINING corpus — authored from manifest text only\n"
        f"# (docs/cell-potion-training-spec.md; regenerate: cell-eval potion-pairs). "
        f"Stats: {json.dumps(stats)}\n"
        "# Before training, run the pre-registered decontamination audit:\n"
        "#   python cell-eval/potion/audit_overlap.py --pairs <this file>\n"
        "# NEVER add rows from datasets/retrieval.jsonl: that is the frozen eval judge.\n"
    )
    body = "\n".join(json.dumps(r) for r in rows)
    path.write_text(header + body + "\n")
