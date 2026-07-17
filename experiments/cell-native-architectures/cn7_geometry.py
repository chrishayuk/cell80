#!/usr/bin/env python3
"""CN-7 — target-space geometry drift (prereg §8.9 disambiguation instrument).

W_f's inputs (cell-side behavioural fingerprints) are frozen by construction; the 105→208
P-d1′ regression must therefore live in the TARGET space — the hidden state at the emission
position that W_f(FP) is trained to project into. This measures how far that geometry moved
between two bases (default: raw v11 vs full-model midtrain) on the FIXED eval contexts
(same Random(0)-shuffled items as every fp eval).

Metrics per bucket and pooled:
  - RSA: Pearson + Spearman correlation of condensed pairwise-distance matrices
  - orthogonal Procrustes residual (centered, scale-normalised): ||A R - B||_F / ||B||_F
  - linear CKA
Reading (registered): geometry moved -> drift story; geometry stable -> W_f fit failure.

Run: python3 cn7_geometry.py --post cn7_ckpt_midtrain.pt [--pre cn7_ckpt_midtrain_attn.pt]
"""
from __future__ import annotations

import argparse
import json
import math
import random
import time
from pathlib import Path

from artifact_paths import checkpoint_input, dataset_input

import torch
import torch.nn.functional as F

import cn1_model
from cn1_model import resize_embedding
from cn7_fp_rebaseline import SpEnc

HERE = Path(__file__).resolve().parent


def load_base(ckpt, vocab_from_map):
    from tiny_model_v11.loader import load_from_artifacts
    base, cfg = load_from_artifacts(str(cn1_model.TINY_MODEL / "model" / "v11"), device="cpu")
    if ckpt:
        ck = torch.load(checkpoint_input(ckpt), map_location="cpu")
        resize_embedding(base, ck["vocab"])
        base.load_state_dict(ck["state"])
    else:
        resize_embedding(base, vocab_from_map)
    return base.eval()


@torch.no_grad()
def states(base, enc, items):
    out = []
    for r in items:
        ids = torch.tensor([[2] + enc.encode(r["context"] + " <call>")])
        x = F.embedding(ids, base.embed.weight) * math.sqrt(base.dim)
        for layer in base.layers:
            x = layer(x, base.rope_freqs)
        out.append(base.norm(x)[0, -1])
    return torch.stack(out)


def rsa(A, B):
    dA = torch.pdist(A)
    dB = torch.pdist(B)
    pear = float(torch.corrcoef(torch.stack([dA, dB]))[0, 1])
    ra = dA.argsort().argsort().float()
    rb = dB.argsort().argsort().float()
    spear = float(torch.corrcoef(torch.stack([ra, rb]))[0, 1])
    return pear, spear


def procrustes(A, B):
    A = A - A.mean(0)
    B = B - B.mean(0)
    A = A / A.norm()
    B = B / B.norm()
    U, _, Vt = torch.linalg.svd(A.T @ B)
    R = U @ Vt
    return float((A @ R - B).norm() / B.norm())


def cka(A, B):
    A = A - A.mean(0)
    B = B - B.mean(0)
    hsic = (A.T @ B).norm() ** 2
    return float(hsic / ((A.T @ A).norm() * (B.T @ B).norm()))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--pre", default=None, help="baseline ckpt (default: raw v11)")
    ap.add_argument("--post", required=True)
    ap.add_argument("--cap", type=int, default=200)
    args = ap.parse_args()
    t0 = time.time()

    tokmap = json.load(open(HERE / "cn7_token_map.json"))
    enc = SpEnc(tokmap["cells"])
    ev = [json.loads(l) for l in dataset_input("cn1_corpus_eval.jsonl").read_text().splitlines() if l.strip()]
    buckets = {}
    for r in ev:
        buckets.setdefault((r["bucket_cell"], r["bucket_comp"]), []).append(r)

    pre = load_base(args.pre, tokmap["vocab"])
    post = load_base(args.post, tokmap["vocab"])

    out = {"pre": args.pre or "raw_v11", "post": args.post, "buckets": {}}
    pooled_pre, pooled_post = [], []
    for bucket in (("novel_cell", "seen_comp"), ("seen_cell", "seen_comp")):
        items = list(buckets[bucket])
        random.Random(0).shuffle(items)
        items = items[:args.cap]
        A = states(pre, enc, items)
        B = states(post, enc, items)
        pooled_pre.append(A)
        pooled_post.append(B)
        pear, spear = rsa(A, B)
        row = {"n": len(items), "rsa_pearson": round(pear, 4), "rsa_spearman": round(spear, 4),
               "procrustes_residual": round(procrustes(A, B), 4), "linear_cka": round(cka(A, B), 4)}
        out["buckets"]["|".join(bucket)] = row
        print(f"  {'|'.join(bucket):<22} RSA {pear:.3f}/{spear:.3f}  procrustes {row['procrustes_residual']:.3f}  CKA {row['linear_cka']:.3f}", flush=True)
    A = torch.cat(pooled_pre)
    B = torch.cat(pooled_post)
    pear, spear = rsa(A, B)
    out["pooled"] = {"rsa_pearson": round(pear, 4), "rsa_spearman": round(spear, 4),
                     "procrustes_residual": round(procrustes(A, B), 4), "linear_cka": round(cka(A, B), 4)}
    print(f"  pooled                 RSA {pear:.3f}/{spear:.3f}  procrustes {out['pooled']['procrustes_residual']:.3f}  CKA {out['pooled']['linear_cka']:.3f}")

    tagp = Path(args.post).stem
    tagq = Path(args.pre).stem if args.pre else "raw"
    path = HERE / f"cn7_geometry_{tagq}__{tagp}.json"
    path.write_text(json.dumps(out, indent=1))
    print(f"wrote {path.name} ({time.time()-t0:.0f}s)")


if __name__ == "__main__":
    main()
