#!/usr/bin/env python3
"""CN-1 description baseline (pre-registration amendment): compute each cell's *language* address —
a sentence-encoding of its descriptor — for arm (d), the mandatory CoTools-style baseline that the
behaviour arm must beat. Same descriptor text the corpus context uses (`describe(name, pack)`),
encoded with bge-small-en-v1.5 (384-d), cached to `cn1_desc_features.json` (name -> 384 floats).

Run: python3 cn1_desc_features.py
"""
from __future__ import annotations

import json
from pathlib import Path

from cn1_corpus import ABBREV

HERE = Path(__file__).resolve().parent
LIBRARY = HERE / "cn1_library.jsonl"
OUT = HERE / "cn1_desc_features.json"
ENCODER = "BAAI/bge-small-en-v1.5"


def describe_rich(r: dict) -> str:
    """The STRONG description address (arm d must be the best version of CoTools, not a
    bag-of-name-words strawman): expanded operation words + family + typed signature, phrased as a
    natural sentence for a real sentence encoder. Uses the richest available documentation — cells
    lack docstrings, so name-words + signature + pack is the ceiling of what description-routing
    could see for a documented cell."""
    words = " ".join(ABBREV.get(t, t) for t in r["name"].split("_"))
    family = r["pack"].replace("-", " ")
    ret = r.get("ret", "u16")
    return (
        f"operation: {words}. family: {family}. "
        f"signature: takes {r['arity']} argument(s), returns {ret}."
    )


def main():
    from sentence_transformers import SentenceTransformer

    rows = [json.loads(l) for l in LIBRARY.read_text().splitlines() if l.strip()]
    names = [r["name"] for r in rows]
    texts = [describe_rich(r) for r in rows]
    model = SentenceTransformer(ENCODER)
    embs = model.encode(texts, normalize_embeddings=True, show_progress_bar=False)
    out = {n: [float(x) for x in e] for n, e in zip(names, embs)}
    OUT.write_text(json.dumps(out))
    print(f"encoded {len(names)} rich descriptions -> {OUT.name} (dim {len(next(iter(out.values())))})")
    print("sample:", names[0], "::", describe_rich(rows[0]))


if __name__ == "__main__":
    main()
