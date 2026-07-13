#!/usr/bin/env python3
"""CN-1 real build, smoke-slice diagnostic: does the base model represent WHICH operation a set
of behavioral I/O demonstrations shows, at the <call> decision point?

The smoke training run (`cn1_train.py`) collapsed to predicting one cell regardless of context.
This probe isolates why: it measures the cosine similarity of the base's hidden state at the
<call> position, WITHIN a cell (same operation, different operands) vs. BETWEEN cells (different
operations). If within ≈ between (separation ≈ 0), the base does not encode the operation at the
decision point, so no embedding strategy (W_f or free rows) has a context signal to condition on
— the invocation reflex is unlearnable from behavioral demonstrations alone at this scale, which
is a corpus/representation finding, not a training bug.

Run: python3 cn1_probe_separation.py
"""
from __future__ import annotations

import itertools
import json
import math
import statistics as st
from collections import defaultdict
from pathlib import Path

import torch
import torch.nn.functional as F

import cn1_model

HERE = Path(__file__).resolve().parent


def main():
    import v11

    tok = v11.Tokenizer.from_file(str(HERE / "v11-cells.vocab.bin"))
    model, names, held = cn1_model.build("fingerprint")  # cpu, untrained
    model.eval()

    rows = [json.loads(l) for l in (HERE / "cn1_corpus_train.jsonl").read_text().splitlines() if l.strip()]
    by_cell = defaultdict(list)
    for r in rows:
        by_cell[r["cell"]].append(r)
    cells = [c for c in by_cell if len(by_cell[c]) >= 4][:8]

    w = model.effective_embed_weight()

    def hidden(text):
        ids = torch.tensor([[2] + tok.encode(text + " <call>")])
        x = F.embedding(ids, w) * math.sqrt(model.dim)
        for layer in model.base.layers:
            x = layer(x, model.base.rope_freqs)
        x = model.base.norm(x)
        return x[0, -1].detach()

    vecs = {c: [hidden(by_cell[c][i]["context"]) for i in range(4)] for c in cells}

    def cos(a, b):
        return F.cosine_similarity(a, b, dim=0).item()

    within, between = [], []
    for c in cells:
        vs = vecs[c]
        for i, j in itertools.combinations(range(4), 2):
            within.append(cos(vs[i], vs[j]))
    for c1, c2 in itertools.combinations(cells, 2):
        between.append(cos(vecs[c1][0], vecs[c2][0]))

    sep = st.mean(within) - st.mean(between)
    print("frozen-base hidden state at the <call> position — cosine similarity:")
    print(f"  WITHIN  same cell (diff operands): {st.mean(within):.3f}")
    print(f"  BETWEEN diff cells:                {st.mean(between):.3f}")
    print(f"  separation (within - between):     {sep:+.3f}")
    verdict = (
        "the base DOES NOT encode which operation the demos show at the decision point"
        if abs(sep) < 0.05
        else "the base separates operations to some degree"
    )
    print(f"  -> {verdict}")
    return sep


if __name__ == "__main__":
    main()
