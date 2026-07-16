#!/usr/bin/env python3
"""The broker loop, closed for the first time: model emits a cell call, the runtime executes
it, the verified result is injected, generation resumes carrying the real number.

This is what the mask trained the model to DO: at the injected-answer position the masked
model abstains (it was never given gradient there); the runtime fills the hole with the
cell's answer — microseconds, exact, greppable provenance — and the model narrates on.

Run: python3 cn7_broker.py --ckpt cn7_ckpt_midtrain.pt \
       --prompt "157 sweets were shared fairly between 16 children. The sharing machine said each child gets"
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

import torch

import cell80_py
import cn1_model
from cn1_model import resize_embedding
from cn7_corpus import CALL_ID, CLOSE_ID, CELL_FIRST_ID, Enc
from cn7_deck import decode_mask

HERE = Path(__file__).resolve().parent
CELLS_DIR = HERE.parent.parent / "cell80" / "cells"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ckpt", default="cn7_ckpt_midtrain.pt")
    ap.add_argument("--prompt", required=True)
    ap.add_argument("--max-tokens", type=int, default=60)
    ap.add_argument("--max-calls", type=int, default=3)
    args = ap.parse_args()

    tokmap = json.load(open(HERE / "cn7_token_map.json"))
    enc = Enc(tokmap["cells"])
    inv = {v: k for k, v in tokmap["cells"].items()}
    from tiny_model_v11.loader import load_from_artifacts
    base, _ = load_from_artifacts(str(cn1_model.TINY_MODEL / "model" / "v11"), device="cpu")
    ck = torch.load(HERE / args.ckpt, map_location="cpu")
    resize_embedding(base, ck["vocab"])
    base.load_state_dict(ck["state"])
    base.eval()
    neg = torch.full((ck["vocab"],), float("-inf"))
    neg[decode_mask(ck["vocab"])] = 0.0
    host = cell80_py.CellHost()
    handles = {}

    ids = enc.seg_ids(args.prompt)
    emitted, calls = [], 0
    print(f"PROMPT: {args.prompt}")
    with torch.no_grad():
        while len(emitted) < args.max_tokens:
            lg = base(torch.tensor([ids + emitted]))[0, -1] + neg
            t = int(lg.argmax())
            if t == 3:
                break
            emitted.append(t)
            if t == CLOSE_ID and calls < args.max_calls:
                # parse the call span just closed: ... CALL_ID, cell_tok, digits..., CLOSE_ID
                span = emitted[len(emitted) - 1 - next(i for i, x in enumerate(reversed(emitted)) if x == CALL_ID):]
                cell_toks = [x for x in span if x >= CELL_FIRST_ID]
                digit_ids = [x for x in span if x < 71261]
                if not cell_toks:
                    continue
                name = inv[cell_toks[0]]
                arg_txt = enc.sp.decode(digit_ids)
                cargs = [int(s) for s in arg_txt.split() if s.lstrip("-").isdigit()]
                if name not in handles:
                    host.add_source(name, next(CELLS_DIR.rglob(f"{name}.rs")).read_text())
                    handles[name] = host.load(name)
                r = host.run(handles[name], cargs)
                res = r["result"] if r.get("halt") == "returned" else None
                print(f"  [broker] model called {name}({', '.join(map(str, cargs))}) -> cell returned {res}")
                if res is not None:
                    emitted.extend(enc.sp.encode(f" {res}"))  # inject the VERIFIED number
                calls += 1
    # render
    out, run = [], []
    for t in emitted:
        s = " <call>" if t == CALL_ID else " </call>" if t == CLOSE_ID else \
            f" ⟨{inv[t]}⟩" if t >= CELL_FIRST_ID else None
        if s is None:
            run.append(t)
        else:
            if run:
                out.append(" " + enc.sp.decode(run))
                run = []
            out.append(s)
    if run:
        out.append(" " + enc.sp.decode(run))
    print(f"OUTPUT: {''.join(out).strip()}")


if __name__ == "__main__":
    main()
