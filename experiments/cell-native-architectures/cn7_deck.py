#!/usr/bin/env python3
"""CN-7 frozen prompt deck — the register-drift seismometer.

Twelve fixed TinyStories openings, committed once, greedy-decoded at every panel forever.
P-e (val NLL) is a READING measure and can hold flat while the generation register drifts
(the Inkling CoT-compression lesson: compression pressure shows in the OUTPUT register
first). A frozen deck makes drift longitudinal and diffable across the whole run ladder:
R0 (raw v11) is the baseline; every later checkpoint's outputs diff against it.

Decode-legality: raw v11 restricts to train_mask (the ~62k never-trained rows are noise).
Midtrained checkpoints restrict to train_mask ∪ ids-present-in-cn7-corpus ∪ appended tokens
(cached to cn7_decode_mask.pt) — midtraining moves hidden states, so untouched garbage rows
could otherwise win argmax through the tied head.

Run: python3 cn7_deck.py --raw                       # R0 baseline
     python3 cn7_deck.py --ckpt cn7_ckpt_midtrain.pt # panel reading
"""
from __future__ import annotations

import argparse
import json
import time
from pathlib import Path

import torch

import cn1_model
from cn1_model import resize_embedding
from cn7_corpus import SP_MODEL, CELL_FIRST_ID
from artifact_paths import checkpoint_input, checkpoint_output, dataset_input

HERE = Path(__file__).resolve().parent
GEN_TOKENS = 60

DECK = [
    "Once upon a time, there was a little girl who",
    "Tom and his dog ran to the",
    "One sunny day, Lily found a shiny",
    "The big red truck stopped in front of",
    "Mia was sad because her",
    "In the garden, a small bird",
    "Ben looked up at the sky and saw",
    "The old cat liked to sleep on",
    "Every morning, Sam would count his",
    "Anna opened the box and inside was",
    "It was raining, so the children",
    "The little fish swam far away from",
]


def decode_mask(vocab):
    """train_mask ∪ corpus ids ∪ appended tokens, cached."""
    cache = checkpoint_output("cn7_decode_mask.pt")
    if cache.exists():
        m = torch.load(cache, map_location="cpu")
        if m.shape[0] == vocab:
            return m
    m = torch.zeros(vocab, dtype=torch.bool)
    tm = torch.load(str(cn1_model.TINY_MODEL / "model" / "v11" / "artifacts" / "train_mask.pt"),
                    map_location="cpu")
    m[:tm.shape[0]] = tm
    m[71261:] = True  # <call>, </call>, cell tokens — all touched by the midtrain corpus
    seen = set()
    for line in dataset_input("cn7_corpus_train.jsonl").open():
        seen.update(json.loads(line)["ids"])
    m[sorted(i for i in seen if i < vocab)] = True
    torch.save(m, cache)
    print(f"  built decode mask: {int(m.sum())}/{vocab} legal ids (cached)")
    return m


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", action="store_true")
    ap.add_argument("--ckpt", default=None)
    ap.add_argument("--device", default="cpu")
    args = ap.parse_args()
    import sentencepiece as spm
    sp = spm.SentencePieceProcessor(model_file=SP_MODEL)
    from tiny_model_v11.loader import load_from_artifacts
    base, cfg = load_from_artifacts(str(cn1_model.TINY_MODEL / "model" / "v11"), device="cpu")

    if args.raw:
        tag = "r0_raw_v11"
        mask = torch.load(str(cn1_model.TINY_MODEL / "model" / "v11" / "artifacts" / "train_mask.pt"),
                          map_location="cpu")
        vocab = mask.shape[0]
    elif args.ckpt:
        ck = torch.load(checkpoint_input(args.ckpt), map_location="cpu")
        vocab = ck["vocab"]
        resize_embedding(base, vocab)
        base.load_state_dict(ck["state"])
        tag = Path(args.ckpt).stem
        mask = decode_mask(vocab)
    else:
        raise SystemExit("pass --raw or --ckpt PATH")
    base = base.to(args.device).eval()
    neg = torch.full((vocab,), float("-inf"), device=args.device)
    neg[mask] = 0.0

    t0 = time.time()
    out = {"tag": tag, "gen_tokens": GEN_TOKENS, "decode": "greedy, legality-masked", "rows": []}
    with torch.no_grad():
        for prompt in DECK:
            ids = [sp.bos_id()] + sp.encode(prompt)
            toks = []
            for _ in range(GEN_TOKENS):
                lg = base(torch.tensor([ids + toks], device=args.device))[0, -1][:vocab] + neg
                t = int(lg.argmax())
                if t == sp.eos_id():
                    break
                toks.append(t)
            cont = sp.decode([t for t in toks if t < 71261])
            out["rows"].append({"prompt": prompt, "continuation": cont})
            print(f"  {prompt[:38]:<40} -> {cont[:70]}", flush=True)
    path = HERE / f"cn7_deck_out_{tag}.json"
    path.write_text(json.dumps(out, indent=1))
    print(f"wrote {path.name} ({time.time()-t0:.0f}s)")


if __name__ == "__main__":
    main()
