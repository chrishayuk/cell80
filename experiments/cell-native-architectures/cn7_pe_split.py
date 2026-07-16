#!/usr/bin/env python3
"""CN-7.2 panel — the §8.8 P-e gain decomposition (registered pre-checkpoint).

Splits the P-e val slice (last 500 S4 rows) at sentence level: strict cardinal number-word
sentences (one…twelve, twenty, hundred; 541 sentences at registration) vs the rest. Computes
per-slice NLL under BOTH the raw v11 base and the midtrained checkpoint, and reports the
relative gain per slice. Transfer story: cardinal slice gains more. Continued-pretraining
story (the §8.8 favourite): uniform gains.

Sentence membership is assigned per token by '.'-boundary segments over the DECODED pieces;
each token inherits the sentence it sits in ('.' closes its sentence).

Run: python3 cn7_pe_split.py --ckpt cn7_ckpt_midtrain.pt
"""
from __future__ import annotations

import argparse
import json
import re
import time
from pathlib import Path

import torch
import torch.nn.functional as F

import cn1_model
from cn1_model import resize_embedding
from cn7_corpus import SP_MODEL

HERE = Path(__file__).resolve().parent
CARDINAL = re.compile(r"\b(one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|twenty|hundred)\b", re.I)


def sentence_flags(sp, ids):
    """Per-token bool: token belongs to a cardinal-word sentence."""
    pieces = [sp.id_to_piece(i).replace("▁", " ") if i < 71261 else " ?" for i in ids]
    flags = [None] * len(ids)
    i = 0
    while i < len(ids):
        j = i
        while j < len(ids) and "." not in pieces[j]:
            j += 1
        sent = "".join(pieces[i:j + 1])
        val = bool(CARDINAL.search(sent))
        for k in range(i, min(j + 1, len(ids))):
            flags[k] = val
        i = j + 1
    return flags


def val_nll_split(model, val, sp, device, vocab):
    acc = {True: [0.0, 0], False: [0.0, 0]}
    with torch.no_grad():
        for i in range(0, len(val), 8):
            chunk = val[i:i + 8]
            m = max(len(r["ids"]) for r in chunk)
            x = torch.zeros((len(chunk), m), dtype=torch.long)
            am = torch.zeros((len(chunk), m))
            for k, r in enumerate(chunk):
                x[k, :len(r["ids"])] = torch.tensor(r["ids"])
                am[k, :len(r["ids"])] = 1
            x = x.to(device)
            lg = model(x)[:, :-1, :vocab]
            ce = F.cross_entropy(lg.reshape(-1, lg.shape[-1]), x[:, 1:].reshape(-1),
                                 reduction="none").reshape(len(chunk), -1).cpu()
            for k, r in enumerate(chunk):
                fl = sentence_flags(sp, r["ids"])
                for pos in range(1, len(r["ids"])):
                    a = acc[fl[pos]]
                    a[0] += float(ce[k, pos - 1]); a[1] += 1
    return {("cardinal" if key else "plain"): (v / n, n) for key, (v, n) in acc.items()}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ckpt", required=True)
    ap.add_argument("--device", default="cpu")
    args = ap.parse_args()
    t0 = time.time()
    import sentencepiece as spm
    sp = spm.SentencePieceProcessor(model_file=SP_MODEL)

    rows = [json.loads(l) for l in (HERE / "cn7_corpus_train.jsonl").read_text().splitlines() if l.strip()]
    val = [r for r in rows if r["species"] == "s4"][-500:]

    from tiny_model_v11.loader import load_from_artifacts
    out = {}
    for tag in ("raw", "mid"):
        base, cfg = load_from_artifacts(str(cn1_model.TINY_MODEL / "model" / "v11"), device="cpu")
        if tag == "mid":
            ck = torch.load(HERE / args.ckpt, map_location="cpu")
            resize_embedding(base, ck["vocab"])
            base.load_state_dict(ck["state"])
            vocab = ck["vocab"]
        else:
            vocab = 71261
        base = base.to(args.device).eval()
        out[tag] = val_nll_split(base, val, sp, args.device, vocab)
        print(f"  {tag}: " + "  ".join(f"{k} {v:.4f} (n={n})" for k, (v, n) in out[tag].items()), flush=True)

    rep = {}
    for slice_ in ("cardinal", "plain"):
        pre, n = out["raw"][slice_]
        post, _ = out["mid"][slice_]
        rep[slice_] = {"pre": round(pre, 4), "post": round(post, 4),
                       "rel_gain": round((pre - post) / pre, 4), "tokens": n}
    print(json.dumps(rep, indent=1))
    path = HERE / f"cn7_pe_split_{Path(args.ckpt).stem}.json"
    path.write_text(json.dumps({"ckpt": args.ckpt, "slices": rep}, indent=1))
    print(f"wrote {path.name} ({time.time()-t0:.0f}s)")


if __name__ == "__main__":
    main()
