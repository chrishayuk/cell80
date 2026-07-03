"""Train/eval decontamination audit (pre-registered, cell-potion spec).

Both the training corpus and the frozen eval were authored from the same 100
manifests, so independent generators can collide on phrasing. A training query
that near-duplicates a frozen eval query is memorisation, not learning — this
audit measures the overlap under a NEUTRAL embedder (ollama:nomic-embed-text,
not the model being trained) and drops training rows above the threshold.

Direction of information flow: eval text is used ONLY to remove training rows.
Nothing about the eval feeds back into generation or training. The overlap
distribution is banked with the result either way.

Pre-registered rule: drop a training row if max cosine similarity to any frozen
eval query >= 0.92, or if it is a case-insensitive exact duplicate.

Usage:  python audit_overlap.py --pairs ../datasets/potion-train-pairs.jsonl
Writes: <pairs>.clean.jsonl + prints the audit summary (bank this).
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "src"))

from cell_eval.tiers import Embedder  # noqa: E402

THRESHOLD = 0.92
NEUTRAL = "ollama:nomic-embed-text"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--pairs", type=Path, required=True)
    ap.add_argument("--eval", type=Path, default=HERE.parent / "datasets" / "retrieval.jsonl")
    a = ap.parse_args()

    train_rows = [json.loads(l) for l in a.pairs.read_text().splitlines()
                  if l.strip() and not l.startswith("#")]
    eval_qs = [json.loads(l)["query"] for l in a.eval.read_text().splitlines()
               if l.strip() and not l.startswith("#")]

    emb = Embedder(NEUTRAL)

    def encode_chunked(texts, n=128):
        parts = [emb.encode(texts[i:i + n]) for i in range(0, len(texts), n)]
        return np.vstack(parts)

    T = encode_chunked([r["query"] for r in train_rows])
    E = encode_chunked(eval_qs)
    S = T @ E.T  # cosine (both L2-normalised)
    nn = S.max(axis=1)

    eval_lower = {q.strip().lower() for q in eval_qs}
    keep, drop = [], []
    for r, sim in zip(train_rows, nn):
        exact = r["query"].strip().lower() in eval_lower
        (drop if (sim >= THRESHOLD or exact) else keep).append(
            {**r, "_nn_sim": round(float(sim), 4), "_exact": exact})

    hist = {f"{lo:.2f}-{lo + 0.05:.2f}": int(((nn >= lo) & (nn < lo + 0.05)).sum())
            for lo in np.arange(0.5, 1.0, 0.05)}
    summary = {
        "neutral_embedder": NEUTRAL,
        "threshold": THRESHOLD,
        "train_rows": len(train_rows),
        "eval_queries": len(eval_qs),
        "dropped": len(drop),
        "kept": len(keep),
        "nn_sim": {"max": round(float(nn.max()), 4),
                   "mean": round(float(nn.mean()), 4),
                   "p95": round(float(np.percentile(nn, 95)), 4)},
        "nn_sim_hist_0.5+": hist,
        "dropped_rows": [{"cell": d["cell"], "kind": d["kind"], "query": d["query"],
                          "sim": d["_nn_sim"]} for d in sorted(drop, key=lambda x: -x["_nn_sim"])],
    }

    out = a.pairs.with_suffix(".clean.jsonl")
    out.write_text("\n".join(json.dumps({k: v for k, v in r.items()
                                         if not k.startswith("_")}) for r in keep) + "\n")
    print(json.dumps(summary, indent=1))
    print(f"\nwrote {out}", file=sys.stderr)


if __name__ == "__main__":
    main()
