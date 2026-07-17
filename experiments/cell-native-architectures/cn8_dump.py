#!/usr/bin/env python3
"""CN-8 diagnostic trace dump — post-hoc DESCRIPTION only, grades nothing (prereg thresholds
untouched). Prints the first N free-running generations for a band so failure modes named by
the first-error histogram can be seen verbatim.

Run: python3 cn8_dump.py --ckpt cn8_ckpt_b_s80.pt --band B1 --n 10
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

import torch

import cn1_model
from cn8_corpus import SP_MODEL, trace_text
from cn8_eval import generate

HERE = Path(__file__).resolve().parent


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ckpt", required=True)
    ap.add_argument("--band", default="B1")
    ap.add_argument("--n", type=int, default=10)
    ap.add_argument("--device", default=None)
    args = ap.parse_args()
    device = args.device or ("mps" if torch.backends.mps.is_available() else "cpu")

    import sentencepiece as spm
    sp = spm.SentencePieceProcessor(model_file=SP_MODEL)
    from tiny_model_v11.loader import load_from_artifacts
    base, _ = load_from_artifacts(str(cn1_model.TINY_MODEL / "model" / "v11"), device="cpu")
    ck = torch.load(HERE / args.ckpt, map_location="cpu")
    base.load_state_dict(ck["state"])
    base = base.to(device).eval()

    probs = json.loads((HERE / "cn8_eval_problems.json").read_text())[args.band][:args.n]
    prompts = [f"{a} + {b} =" for a, b in probs]
    gens = generate(base, sp, prompts, device)
    for (a, b), (gen, truncated) in zip(probs, gens):
        print(f"### {a} + {b} = {a + b}   (truncated={truncated})")
        print(f"  model : {gen}")
        print(f"  oracle: {trace_text(a, b)[len(f'{a} + {b} ='):].strip()}")
        print()


if __name__ == "__main__":
    main()
