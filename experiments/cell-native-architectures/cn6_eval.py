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
    ap.add_argument("--input-max", type=int, default=1000, help="oracle-mode: draw example inputs in [0,this]")
    ap.add_argument("--sample", type=float, default=0.0, help="ckpt-hf: temperature (>0 => sample, tests decode-collapse)")
    ap.add_argument("--nsample", type=int, default=1, help="ckpt-hf: union pairs from this many sampled specs")
    ap.add_argument("--seed", type=int, default=0)
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
                args_ = [rng.randint(0, args.input_max) for _ in range(a)]
                r = host.run(handles[name], args_)
                if r.get("halt") == "returned":
                    out.append((args_, r["result"]))
            return out
        print(f"ORACLE ceiling (correct examples, inputs 0..{args.input_max} — pipeline + powered bar):")
        report("all value cells", [r for r in (route_rank(n, oracle_pairs(n)) for n in value) if r is not None])
        report("held-out only", [r for r in (route_rank(n, oracle_pairs(n)) for n in held_val) if r is not None])
        return

    if args.ckpt_hf:
        import torch
        from transformers import AutoModelForCausalLM, AutoTokenizer
        ck = torch.load(args.ckpt_hf, map_location="cpu")
        arm = ck.get("arm", args.arm)
        base_id = ck.get("base_id", "HuggingFaceTB/SmolLM2-135M")
        imax = ck.get("input_max", 1000)
        etag = "" if imax == 1000 else f"_i{imax}"
        tok = AutoTokenizer.from_pretrained(base_id)
        tok.add_tokens(["<call>", "</call>"], special_tokens=True)
        close_id = tok.convert_tokens_to_ids("</call>")
        model = AutoModelForCausalLM.from_pretrained(base_id, dtype=torch.float32)
        model.resize_token_embeddings(ck["vocab"])
        model.load_state_dict(ck["base"])
        device = "cpu"  # generation on CPU dodges the MPSGraph LM-head bug (small n)
        model.to(device).eval()
        print(f"  base {base_id} | input-max {imax}", flush=True)
        ev = [json.loads(l) for l in (HERE / f"cn6_corpus_eval_{arm}{etag}.jsonl").read_text().splitlines() if l.strip()]

        T = args.sample
        torch.manual_seed(args.seed)

        def pick(lg):
            return int(torch.multinomial(torch.softmax(lg / T, -1), 1)) if T > 0 else int(lg.argmax())

        @torch.no_grad()
        def gen_spec(context):
            prompt = [tok.bos_token_id] + tok.encode(context + " <call>", add_special_tokens=False)
            out = model(input_ids=torch.tensor([prompt], device=device), use_cache=True)
            past = out.past_key_values
            toks = [pick(out.logits[0, -1])]
            for _ in range(47):
                if toks[-1] == close_id:
                    break
                out = model(input_ids=torch.tensor([[toks[-1]]], device=device), past_key_values=past, use_cache=True)
                past = out.past_key_values
                toks.append(pick(out.logits[0, -1]))
            gen = tok.decode(prompt + toks)
            seg = gen.split("<call>", 1)[-1].split("</call>", 1)[0]
            return seg

        def cell_pairs(context):
            # union distinct pairs across nsample sampled specs (nsample=1 => single spec)
            seen_p, pairs = set(), []
            for _ in range(max(1, args.nsample)):
                for a, o in parse_spec(gen_spec(context)):
                    key = (tuple(a), o)
                    if key not in seen_p:
                        seen_p.add(key); pairs.append((a, o))
            return pairs

        print(f"  decode: {'greedy' if T == 0 else f'sample T={T}'} | nsample {args.nsample}", flush=True)
        # one row per held-out cell (dedup) — the cell is the unit
        seen, ranks, corr, per, ndist = {}, [], [], [], []
        for r in ev:
            seen.setdefault(r["cell"], r)
        for cell, r in seen.items():
            pairs = cell_pairs(r["context"])
            c = correctness(cell, pairs)
            rk = route_rank(cell, pairs)
            rk = rk if rk is not None else len(value)
            corr.append(c); ranks.append(rk); per.append((cell, c, rk))
            ndist.append(len({tuple(a) for a, _ in pairs}))
        print(f"  mean distinct inputs / spec: {sum(ndist)/len(ndist):.2f}", flush=True)
        print(f"CN-6 {arm} — model-generated specs, held-out cells:")
        report("held-out", ranks, corr)
        # per-cell: separates arithmetic failure (correctness) from non-discrimination (rank | correct)
        print("\n  per cell (correctness | rank | resolves@5):")
        for cell, c, rk in sorted(per, key=lambda x: -x[1]):
            print(f"      {cell:<26} corr {c:.2f}  rank {rk:>3}  {'Y' if rk < 5 else '.'}")
        # among cells the model computed correctly, does the spec resolve? (isolates discrimination)
        good = [(c, rk) for _, c, rk in per if c >= 0.99]
        if good:
            res = sum(rk < 5 for _, rk in good) / len(good)
            print(f"\n  resolve@5 | fully-correct spec (n={len(good)}): {res:.3f}"
                  f"  — the discrimination ceiling the model actually hit")
        return

    print("pass --oracle (ceiling/smoke) or --ckpt-hf PATH (trained model)")


if __name__ == "__main__":
    main()
