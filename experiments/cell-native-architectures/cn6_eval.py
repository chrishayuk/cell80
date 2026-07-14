#!/usr/bin/env python3
"""CN-6 stage 2 eval — graded on END-TO-END RESOLUTION, not router P@1, with error bars.

The two-tier pipeline needs the true cell in an EXECUTABLE set, not at rank-1: the model emits a
spec, the router narrows to top-k by behavioural match, and execution confirms the winner exactly.
So the operating metric is "does the emitted spec put the true cell in the router's top-k" (k the
executable budget), and every number ships with its SE (the terrain is noisy — small held-out sets,
library-property measurements; five sampling catches so far).

Modes:
  --oracle          : emit ground-truth examples (validates parse->route->resolution AND gives the
                      properly-powered ceiling; leave-one-out over all 249 value cells, n big).
  --ckpt-hf PATH    : a trained SmolLM2 CN-6 model generates the spec, we parse it, route, resolve
                      (held-out cells only, n=24 — the number that matters, with its wide SE).
Also reports emitted-example CORRECTNESS (run the true cell on the emitted inputs) — the generation
arm's diagnostic: a spec can be un-resolving because the model computed wrong, or because it was
non-discriminating; correctness separates them.

Run: python3 cn6_eval.py --oracle
"""
from __future__ import annotations

import argparse
import json
import math
import random
import re
from pathlib import Path

import cell80_py

HERE = Path(__file__).resolve().parent
CELLS_DIR = HERE.parent.parent / "cell80" / "cells"


def wilson(k, n):
    if n == 0:
        return (0.0, 0.0, 0.0)
    p = k / n
    z = 1.96
    d = 1 + z * z / n
    c = (p + z * z / (2 * n)) / d
    h = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / d
    return p, max(0, c - h), min(1, c + h)


def parse_spec(text):
    """Parse `a b = r ; c d = s ; ...` -> list of ([ints], out). Tolerant of trailing junk."""
    out = []
    for chunk in text.split(";"):
        if "=" not in chunk:
            continue
        lhs, rhs = chunk.split("=", 1)
        nums = re.findall(r"-?\d+", lhs)
        rnum = re.findall(r"-?\d+", rhs)
        if nums and rnum:
            out.append(([int(x) & 0xFFFF for x in nums], int(rnum[0]) & 0xFFFF))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--oracle", action="store_true")
    ap.add_argument("--ckpt-hf", default=None)
    ap.add_argument("--arm", default="generation")
    ap.add_argument("--k", type=int, nargs="+", default=[1, 5, 10])
    args = ap.parse_args()

    lib = {json.loads(l)["name"]: json.loads(l) for l in (HERE / "cn1_library.jsonl").read_text().splitlines() if l.strip()}
    held = {h["name"] for h in json.loads((HERE / "cn1_axis_a_heldout.json").read_text())["held_out_cells"]}
    value = [n for n, r in lib.items() if r["arity"] >= 1]

    host = cell80_py.CellHost()
    handles = {}
    for n in value:
        try:
            host.add_source(n, next(CELLS_DIR.rglob(f"{n}.rs")).read_text())
            handles[n] = host.load(n)
        except Exception:
            pass
    value = [n for n in value if n in handles]
    held_val = [n for n in value if n in held]
    rng = random.Random(0)

    def route_rank(name, pairs):
        if not pairs:
            return None
        ranked = host.route([(list(a), o) for a, o in pairs], limit=len(value))
        names = [r.get("id") if isinstance(r, dict) else r for r in ranked]
        return names.index(name) if name in names else len(value)

    def correctness(name, pairs):
        good = 0
        for a, o in pairs:
            try:
                r = host.run(handles[name], list(a))
                good += r.get("halt") == "returned" and r["result"] == o
            except Exception:
                pass
        return good / len(pairs) if pairs else 0.0

    def report(label, ranks, corr=None):
        n = len(ranks)
        print(f"  {label} (n={n}):")
        for k in args.k:
            p, lo, hi = wilson(sum(r < k for r in ranks), n)
            print(f"      resolve@{k:<2} {p:.3f}  [{lo:.3f},{hi:.3f}]")
        if corr is not None and corr:
            print(f"      emitted-example correctness: {sum(corr)/len(corr):.3f}")

    if args.oracle:
        # ceiling: oracle examples, leave-one-out over ALL value cells (tight) and held-out (n=24)
        def oracle_pairs(name, k=3):
            a = lib[name]["arity"]; out = []; t = 0
            while len(out) < k and t < k * 12:
                t += 1
                args_ = [rng.randint(0, 1000) for _ in range(a)]
                r = host.run(handles[name], args_)
                if r.get("halt") == "returned":
                    out.append((args_, r["result"]))
            return out
        print("ORACLE ceiling (correct examples — validates pipeline + powered ceiling):")
        report("all value cells", [r for r in (route_rank(n, oracle_pairs(n)) for n in value) if r is not None])
        report("held-out only", [r for r in (route_rank(n, oracle_pairs(n)) for n in held_val) if r is not None])
        return

    if args.ckpt_hf:
        import torch
        from transformers import AutoModelForCausalLM, AutoTokenizer
        ck = torch.load(args.ckpt_hf, map_location="cpu")
        arm = ck.get("arm", args.arm)
        tok = AutoTokenizer.from_pretrained("HuggingFaceTB/SmolLM2-135M")
        tok.add_tokens(["<call>", "</call>"], special_tokens=True)
        close_id = tok.convert_tokens_to_ids("</call>")
        model = AutoModelForCausalLM.from_pretrained("HuggingFaceTB/SmolLM2-135M", dtype=torch.float32)
        model.resize_token_embeddings(ck["vocab"])
        model.load_state_dict(ck["base"])
        device = "cpu"  # generation on CPU dodges the MPSGraph LM-head bug (small n)
        model.to(device).eval()
        ev = [json.loads(l) for l in (HERE / f"cn6_corpus_eval_{arm}.jsonl").read_text().splitlines() if l.strip()]

        @torch.no_grad()
        def gen_spec(context):
            ids = torch.tensor([[tok.bos_token_id] + tok.encode(context + " <call>", add_special_tokens=False)], device=device)
            for _ in range(48):
                nxt = int(model(input_ids=ids).logits[0, -1].argmax())
                ids = torch.cat([ids, torch.tensor([[nxt]], device=device)], 1)
                if nxt == close_id:
                    break
            gen = tok.decode(ids[0].tolist())
            seg = gen.split("<call>", 1)[-1].split("</call>", 1)[0]
            return seg

        # one row per held-out cell (dedup) — the cell is the unit
        seen, ranks, corr = {}, [], []
        for r in ev:
            seen.setdefault(r["cell"], r)
        for cell, r in seen.items():
            pairs = parse_spec(gen_spec(r["context"]))
            corr.append(correctness(cell, pairs))
            rk = route_rank(cell, pairs)
            ranks.append(rk if rk is not None else len(value))
        print(f"CN-6 {arm} — model-generated specs, held-out cells:")
        report("held-out", ranks, corr)
        return

    print("pass --oracle (ceiling/smoke) or --ckpt-hf PATH (trained model)")


if __name__ == "__main__":
    main()
