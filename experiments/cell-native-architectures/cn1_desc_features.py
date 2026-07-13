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

from cn1_corpus import describe

HERE = Path(__file__).resolve().parent
LIBRARY = HERE / "cn1_library.jsonl"
OUT = HERE / "cn1_desc_features.json"
ENCODER = "BAAI/bge-small-en-v1.5"


def main():
    from sentence_transformers import SentenceTransformer

    rows = [json.loads(l) for l in LIBRARY.read_text().splitlines() if l.strip()]
    names = [r["name"] for r in rows]
    texts = [describe(r["name"], r["pack"]) for r in rows]
    model = SentenceTransformer(ENCODER)
    embs = model.encode(texts, normalize_embeddings=True, show_progress_bar=False)
    out = {n: [float(x) for x in e] for n, e in zip(names, embs)}
    OUT.write_text(json.dumps(out))
    print(f"encoded {len(names)} descriptors -> {OUT.name} (dim {len(next(iter(out.values())))})")
    print("sample:", names[0], "::", describe(rows[0]["name"], rows[0]["pack"]))


if __name__ == "__main__":
    main()
