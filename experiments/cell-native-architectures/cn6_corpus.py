#!/usr/bin/env python3
"""CN-6 stage 2 corpus: train the model to emit an I/O example SPEC (not a cell token), which the
runtime router resolves by execution. Two arms, identical machinery, only the target differs:

  --arm generation : target is a fresh example set the model must PRODUCE from the descriptor
                     (moderate inputs it could plausibly compute; the "delegate by demonstrating"
                     claim; the headline). Held-out cells test whether descriptor->behaviour
                     generalizes enough to emit resolving examples.
  --arm extraction : the example pairs are already present in the context (demos); the target
                     RE-EMITS them (a copy/select task; the equipped-query deployment path).

Spec surface reuses the <call>/</call> delimiters: `... <call> a1 b1 = r1 ; a2 b2 = r2 ; a3 b3 = r3
</call>` — numbers are ordinary digit tokens, so the target is a real multi-token generation.
Axis-A held-out cells never appear as training targets (eval only). Oracle-verified throughout.

Run: python3 cn6_corpus.py --arm generation
"""
from __future__ import annotations

import argparse
import json
import random
from pathlib import Path

from cn1_corpus import Oracle, describe

HERE = Path(__file__).resolve().parent
LIBRARY = HERE / "cn1_library.jsonl"
AXIS_A = HERE / "cn1_axis_a_heldout.json"
N_EX = 3  # examples per spec


def spec_str(pairs):
    return " ; ".join(f"{' '.join(map(str, a))} = {o}" for a, o in pairs)


def moderate_args(arity, rng, hi=1000):
    # inputs in [0,hi]; small hi => computable by a real base — the powered check found no width penalty,
    # so we don't force tiny; we do avoid the 0..65535 tail the model can't compute in the gen arm.
    return [rng.randint(0, hi) for _ in range(arity)]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--arm", choices=["generation", "extraction"], default="generation")
    ap.add_argument("--per-cell", type=int, default=60)
    ap.add_argument("--seed", type=int, default=80)
    ap.add_argument("--input-max", type=int, default=1000)
    args = ap.parse_args()
    rng = random.Random(args.seed)

    TAG = "" if args.input_max == 1000 else f"_i{args.input_max}"
    lib = {json.loads(l)["name"]: json.loads(l) for l in LIBRARY.read_text().splitlines() if l.strip()}
    held = {h["name"] for h in json.loads(AXIS_A.read_text())["held_out_cells"]}
    value = [n for n, r in lib.items() if r["arity"] >= 1]
    oracle = Oracle(value)

    def make_row(name):
        arity = lib[name]["arity"]
        # gather N_EX + (extraction: N_EX demos shown in context) clean pairs
        pairs, tries = [], 0
        need = N_EX * 2 if args.arm == "extraction" else N_EX
        while len(pairs) < need and tries < need * 10:
            tries += 1
            a = moderate_args(arity, rng, args.input_max)
            r = oracle.run(name, a)
            if r.get("halt") == "returned":
                pairs.append((a, r["result"]))
        if len(pairs) < need:
            return None
        desc = describe(name, lib[name]["pack"])
        if args.arm == "extraction":
            shown, target = pairs[:N_EX], pairs[:N_EX]  # demos in context ARE the spec (copy)
            context = f"{desc} ; {spec_str(shown)}"
        else:
            target = pairs[:N_EX]
            context = desc
        text = f"{context} <call> {spec_str(target)} </call>"
        return {"cell": name, "arm": args.arm, "context": context,
                "spec": spec_str(target), "text": text, "target_pairs": target}

    train, evalr = [], []
    for name in value:
        n = args.per_cell if name not in held else 12
        made, tries = 0, 0
        while made < n and tries < n * 6:
            tries += 1
            row = make_row(name)
            if row is None:
                continue
            (evalr if name in held else train).append(row)
            made += 1

    rng.shuffle(train)
    (HERE / f"cn6_corpus_train_{args.arm}{TAG}.jsonl").write_text("\n".join(json.dumps(r) for r in train) + "\n")
    (HERE / f"cn6_corpus_eval_{args.arm}{TAG}.jsonl").write_text("\n".join(json.dumps(r) for r in evalr) + "\n")
    print(f"arm {args.arm}: train {len(train)} rows | held-out eval {len(evalr)} rows "
          f"({len({r['cell'] for r in evalr})} held cells)")
    print("sample:", train[0]["text"][:130])


if __name__ == "__main__":
    main()
