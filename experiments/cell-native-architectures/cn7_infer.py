#!/usr/bin/env python3
"""Interactive inference against a CN-7 checkpoint.

Prompt syntax is the training surface: plain text, plus `<call>`, `</call>`, and `⟨cellname⟩`
for a cell token (e.g. "... <call> ⟨mul_sat⟩ 47 23 </call> "). Decode is greedy by default
(pass --temp for sampling), legality-masked, no BOS (the training format).

Run: python3 cn7_infer.py --ckpt cn7_ckpt_midtrain_nomask.pt --prompt "47 + 8 = "
     python3 cn7_infer.py --ckpt cn7_ckpt_midtrain_nomask.pt          # demo sweep
     python3 cn7_infer.py --ckpt cn7_ckpt_midtrain.pt --raw           # raw v11 (SP mapping)
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

import torch

import cn1_model
from cn1_model import resize_embedding
from cn7_corpus import CALL_ID, CLOSE_ID, Enc
from cn7_deck import decode_mask

HERE = Path(__file__).resolve().parent

DEMO = [
    ("tier-A canonical      ", "47 + 8 = "),
    ("tier-A canonical mul  ", "7 x 9 = "),
    ("tier-A narrative      ", "Tom had 47 apples. Lily gave Tom 8 more. Now Tom has "),
    ("tier-A mod narrative  ", "83 berries were put in rows of 7. There were "),
    ("beyond, in-range call ", "The truck brought 47 crates with 23 apples in each crate. The counting machine worked it out: <call> ⟨mul_sat⟩ 47 23 </call> "),
    ("beyond, OFF-range call", "The truck brought 347 crates with 23 apples in each crate. The counting machine worked it out: <call> ⟨mul_sat⟩ 347 23 </call> "),
    ("division call         ", "157 sweets were shared fairly between 16 children. The sharing machine said each child gets <call> ⟨safe_div⟩ 157 16 </call> "),
    ("emission grammar      ", "op square kind safe arith <call>"),
    ("story register        ", "Once upon a time, there was a little girl who"),
]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ckpt", default="cn7_ckpt_midtrain_nomask.pt")
    ap.add_argument("--prompt", default=None)
    ap.add_argument("--temp", type=float, default=0.0)
    ap.add_argument("--max-tokens", type=int, default=40)
    ap.add_argument("--device", default="cpu")
    args = ap.parse_args()

    tokmap = json.load(open(HERE / "cn7_token_map.json"))
    enc = Enc(tokmap["cells"])
    from tiny_model_v11.loader import load_from_artifacts
    base, cfg = load_from_artifacts(str(cn1_model.TINY_MODEL / "model" / "v11"), device="cpu")
    ck = torch.load(HERE / args.ckpt, map_location="cpu")
    resize_embedding(base, ck["vocab"])
    base.load_state_dict(ck["state"])
    base = base.to(args.device).eval()
    neg = torch.full((ck["vocab"],), float("-inf"), device=args.device)
    neg[decode_mask(ck["vocab"])] = 0.0
    inv = {v: k for k, v in tokmap["cells"].items()}

    @torch.no_grad()
    def gen(prompt):
        ids = enc.seg_ids(prompt)
        toks = []
        for _ in range(args.max_tokens):
            lg = base(torch.tensor([ids + toks], device=args.device))[0, -1] + neg
            if args.temp > 0:
                t = int(torch.multinomial(torch.softmax(lg / args.temp, -1), 1))
            else:
                t = int(lg.argmax())
            if t == 3:
                break
            toks.append(t)
        out = []
        for t in toks:
            if t == CALL_ID:
                out.append(" <call>")
            elif t == CLOSE_ID:
                out.append(" </call>")
            elif t >= 71263:
                out.append(f" ⟨{inv[t]}⟩")
            else:
                out.append(None)
        # decode contiguous sp runs, splice specials
        text, run = [], []
        for t, o in zip(toks, out):
            if o is None:
                run.append(t)
            else:
                if run:
                    text.append(" " + enc.sp.decode(run))
                    run = []
                text.append(o)
        if run:
            text.append(" " + enc.sp.decode(run))
        return "".join(text).strip()

    if args.prompt:
        print(f"[{args.ckpt}]")
        print(f">>> {args.prompt}")
        print(f"    {gen(args.prompt)}")
    else:
        print(f"== demo sweep: {args.ckpt} ==")
        for label, p in DEMO:
            print(f"  {label} | {p[:64]}{'…' if len(p) > 64 else ''}")
            print(f"    -> {gen(p)[:110]}")


if __name__ == "__main__":
    main()
