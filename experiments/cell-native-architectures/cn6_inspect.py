#!/usr/bin/env python3
"""Inspect what the trained generation model ACTUALLY emits, cell by cell, against the oracle.
Separates the three failure modes the aggregate can't: (1) function misidentified / miscomputed
(wrong outputs), (2) correct-but-degenerate inputs (small/repeated => non-discriminating), (3)
malformed spec (parse loses examples). Prints the raw generation, the parsed pairs, and for each
emitted input the TRUE output, so a mismatch is visible per-example.

Run: python3 cn6_inspect.py --ckpt-hf cn6_ckpt_generation_llama-32-1b.pt --cells square luhn_check ...
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

import cell80_py
import torch
from transformers import AutoModelForCausalLM, AutoTokenizer

from cn6_eval import parse_spec
from artifact_paths import checkpoint_input, dataset_input

HERE = Path(__file__).resolve().parent
CELLS_DIR = HERE.parent.parent / "cell80" / "cells"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ckpt-hf", required=True)
    ap.add_argument("--cells", nargs="*", default=None)
    args = ap.parse_args()

    ck = torch.load(checkpoint_input(args.ckpt_hf), map_location="cpu")
    arm = ck.get("arm", "generation")
    base_id = ck.get("base_id", "HuggingFaceTB/SmolLM2-135M")
    imax = ck.get("input_max", 1000)
    etag = "" if imax == 1000 else f"_i{imax}"
    tok = AutoTokenizer.from_pretrained(base_id)
    tok.add_tokens(["<call>", "</call>"], special_tokens=True)
    close_id = tok.convert_tokens_to_ids("</call>")
    model = AutoModelForCausalLM.from_pretrained(base_id, dtype=torch.float32)
    model.resize_token_embeddings(ck["vocab"])
    model.load_state_dict(ck["base"])
    model.eval()

    lib = {json.loads(l)["name"]: json.loads(l) for l in (HERE / "cn1_library.jsonl").read_text().splitlines() if l.strip()}
    ev = [json.loads(l) for l in dataset_input(f"cn6_corpus_eval_{arm}{etag}.jsonl").read_text().splitlines() if l.strip()]
    ctx = {}
    for r in ev:
        ctx.setdefault(r["cell"], r["context"])

    host = cell80_py.CellHost()
    handles = {}
    for n in ctx:
        try:
            host.add_source(n, next(CELLS_DIR.rglob(f"{n}.rs")).read_text())
            handles[n] = host.load(n)
        except Exception:
            pass

    @torch.no_grad()
    def gen(context):
        ids = torch.tensor([[tok.bos_token_id] + tok.encode(context + " <call>", add_special_tokens=False)])
        for _ in range(48):
            nxt = int(model(input_ids=ids).logits[0, -1].argmax())
            ids = torch.cat([ids, torch.tensor([[nxt]])], 1)
            if nxt == close_id:
                break
        g = tok.decode(ids[0].tolist())
        return g.split("<call>", 1)[-1].split("</call>", 1)[0].strip()

    cells = args.cells or list(ctx)
    for name in cells:
        if name not in handles:
            continue
        raw = gen(ctx[name])
        pairs = parse_spec(raw)
        print(f"\n=== {name}  (arity {lib[name]['arity']}) ===")
        print(f"  descriptor: {ctx[name]}")
        print(f"  emitted:    {raw!r}")
        for a, o in pairs:
            try:
                r = host.run(handles[name], list(a))
                true = r.get("result") if r.get("halt") == "returned" else "ERR"
            except Exception:
                true = "EXC"
            mark = "ok" if true == o else "WRONG"
            print(f"    {a} -> emitted {o} | true {true}   {mark}")


if __name__ == "__main__":
    main()
