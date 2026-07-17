#!/usr/bin/env python3
"""CN-1 real build, step 4 source 1 (`cell-native-architectures-cn1-preregistration.md`): the
H1 factory — assemble (behavioral context, `<call> <cell:NAME> args </call>`, verified result)
training rows across the value cells, with the two factorized held-out axes the gates depend on.

**Why behavioral (I/O-demonstration) context.** Gate (ii) asks whether a cell *never seen
called* can still be invoked, purely because `W_f(fingerprint)` gave it an address. A cell's
fingerprint IS its behavior on the probe battery; so the smoothest substrate for that transfer
is a context that *demonstrates the behavior* — a few `input = output` examples of the
operation — followed by a query the model must route. The model then learns "demonstrated
behavior → the region of embedding space `W_f` maps that behavior to", and a held-out cell,
sharing the behaviorally-smooth fingerprint space with its trained siblings, inherits an
address there. An arbitrary linguistic name could not transfer; a behavioral cue can. The
demonstrations are the exact-oracle's own outputs (`CellHost.run`), so every row is verified by
construction.

**The two axes, never conflated (pre-registration §Corpus):**
  - **Axis A — held-out cells.** The 24 value cells in `cn1_axis_a_heldout.json` NEVER appear as
    a call target here. (State cells aren't call targets in this arithmetic-shaped corpus at all.)
  - **Axis B — held-out compositions.** Composition = (surface *template* × cell *family*).
    Family = pack. Template = the demonstration's surface format (several uniform formats that
    work for any cell, so the factorization is clean and needs no per-cell natural language,
    which the library mostly lacks). A fixed set of (template, pack) pairs is held out of
    training: every template appears with multiple packs and every pack with multiple
    templates, but the held-out pairs never co-occur — so a pass/fail on them is attributable
    to composition, not to an unseen token.

Deterministic (seeded). No LLM: the oracle is `CellHost`, so this is tractable on the M3 and
reproducible. Emits `cn1_corpus_train.jsonl` + held-out eval rows split by the four buckets,
and `cn1_corpus_stats.json`. This is source 1; the CN-2 harvest (source 2) is added later and
its mix ratio reported then.

Run: python3 cn1_corpus.py [--per-cell N] [--seed S]
"""
from __future__ import annotations

import argparse
import json
import random
from collections import defaultdict, Counter
from pathlib import Path

import cell80_py
from artifact_paths import dataset_output

HERE = Path(__file__).resolve().parent
CELLS_DIR = HERE.parent.parent / "cell80" / "cells"
LIBRARY = HERE / "cn1_library.jsonl"
AXIS_A = HERE / "cn1_axis_a_heldout.json"
TOKEN_MAP = HERE / "cn1_cell_token_map.json"

OUT_TRAIN = dataset_output("cn1_corpus_train.jsonl")
OUT_EVAL = dataset_output("cn1_corpus_eval.jsonl")
OUT_STATS = HERE / "cn1_corpus_stats.json"

# Surface templates for a single "input(s) = output" demonstration (and the query line). Each
# is a (name, render) pair; render(args:list[int], out:int|None) -> str. These are the axis-B
# "template" factor: uniform across all cells, so composition is (template × pack).
def _t_eq(args, out):
    lhs = " ".join(str(a) for a in args)
    return f"{lhs} = {out}" if out is not None else f"{lhs} ="


def _t_arrow(args, out):
    lhs = " , ".join(str(a) for a in args)
    return f"{lhs} -> {out}" if out is not None else f"{lhs} ->"


def _t_io(args, out):
    lhs = " ".join(str(a) for a in args)
    return f"in {lhs} out {out}" if out is not None else f"in {lhs} out"


TEMPLATES = {"eq": _t_eq, "arrow": _t_arrow, "io": _t_io}
TEMPLATE_NAMES = list(TEMPLATES)

N_DEMOS = 2  # behavioral demonstrations per context (reinforce; descriptor is the learnable cue)

# Compositional descriptor vocabulary: the context carries an operation *description* built from
# a controlled set of attribute words, so (a) seen cells are learnable (description -> cell) and
# (b) a held-out cell's description reuses words seen with OTHER cells and, being behaviorally
# similar to its siblings, lands in the region W_f maps that fingerprint to. Cell names are
# already compositional snake_case (add_sat, mul_checked, smallest_prime_factor), so the words
# recur heavily across the library. Common abbreviations expand to shared attribute words to
# maximize reuse (mul appears in mul_sat, mul_checked, mul_i16 -> all share "multiply").
ABBREV = {
    "sat": "saturating", "gt": "greater than", "ge": "at least", "lt": "less than",
    "le": "at most", "eq": "equal", "ne": "not equal", "sub": "subtract", "add": "add",
    "mul": "multiply", "div": "divide", "mod": "modulo", "rem": "remainder", "abs": "absolute",
    "neg": "negate", "min": "minimum", "max": "maximum", "avg": "average", "sqrt": "square root",
    "pow": "power", "exp": "exponent", "log": "logarithm", "gcd": "gcd", "lcm": "lcm",
    "i16": "signed", "u16": "unsigned", "i32": "signed wide", "u32": "unsigned wide",
    "bcd": "binary coded decimal", "rotl": "rotate left", "rotr": "rotate right",
    "shl": "shift left", "shr": "shift right", "popcount": "population count", "clz": "leading zeros",
    "ctz": "trailing zeros", "lerp": "interpolate", "clamp": "clamp", "pct": "percent",
    "is": "is", "has": "has", "to": "to", "of": "of",
}


def describe(name: str, pack: str) -> str:
    """A compositional operation description from the cell's name-words + pack. Deterministic."""
    words = []
    for tok in name.split("_"):
        words.append(ABBREV.get(tok, tok))
    kind = pack.replace("-", " ")
    return f"op {' '.join(words)} kind {kind}"


def load_meta():
    lib = [json.loads(l) for l in LIBRARY.read_text().splitlines() if l.strip()]
    by_name = {r["name"]: r for r in lib}
    held = {h["name"] for h in json.loads(AXIS_A.read_text())["held_out_cells"]}
    tok_map = json.loads(TOKEN_MAP.read_text())
    value = [r for r in lib if r["arity"] >= 1]
    return by_name, held, tok_map, value


def sample_args(arity: int, rng: random.Random, safe: bool = False) -> list[int]:
    """Mix operand regimes so demonstrations span behavior, not one corner. u16 domain.
    `safe=True` draws small non-zero operands — the fallback for cells that escalate (div-by-
    zero, domain limits, overflow) on most random draws, so they still get clean examples."""
    if safe:
        return [rng.randint(1, 12) for _ in range(arity)]
    regime = rng.choice(["small", "mid", "boundary", "wide"])
    def one():
        if regime == "small":
            return rng.randint(0, 20)
        if regime == "mid":
            return rng.randint(0, 999)
        if regime == "boundary":
            return rng.choice([0, 1, 2, 255, 256, 32767, 32768, 65534, 65535])
        return rng.randint(0, 65535)
    return [one() for _ in range(arity)]


class Oracle:
    def __init__(self, names):
        self.host = cell80_py.CellHost()
        self.handles = {}
        for n in names:
            src = next(CELLS_DIR.rglob(f"{n}.rs")).read_text()
            self.host.add_source(n, src)
            self.handles[n] = self.host.load(n)

    def run(self, name, args):
        r = self.host.run(self.handles[name], list(args))
        return r  # {result, halt, trapped_ops, ...}


def make_example(name, arity, template, rng, oracle, pack, grounding="descriptor", safe=False):
    """One row: a context + a query, target is the call, plus verified result. Grounding:
      - "descriptor": compositional operation description + N_DEMOS behavioral demos + query
        (the chosen design — descriptor is the learnable cue, demos reinforce the behavior).
      - "behavioral": demos + query only (the original; kept for the ablation/probe).
    Returns None if any draw doesn't cleanly return (halt != returned)."""
    render = TEMPLATES[template]
    demos = []
    total_trapped = 0
    for _ in range(N_DEMOS + 1):  # last one is the query
        args = sample_args(arity, rng, safe=safe)
        r = oracle.run(name, args)
        if r.get("halt") != "returned":
            return None
        total_trapped += int(r.get("trapped_ops", 0) or 0)
        demos.append((args, r["result"]))
    *demo_pairs, (q_args, q_out) = demos
    demo_text = " ; ".join(render(a, o) for a, o in demo_pairs)
    query_text = render(q_args, None)  # query shows inputs, not the answer
    if grounding == "descriptor":
        context = f"{describe(name, pack)} ; {demo_text} ; {query_text}"
    else:
        context = f"{demo_text} ; {query_text}"
    call = f"<call> <cell:{name}> {' '.join(str(a) for a in q_args)} </call>"
    return {
        "cell": name,
        "template": template,
        "arity": arity,
        "context": context,
        "call": call,
        "args": q_args,
        "result": q_out,
        "text": f"{context} {call} = {q_out}",
        "trapped_ops": total_trapped,
    }


def held_out_compositions(packs, rng):
    """Choose (template, pack) pairs to hold out for axis B: one template held out per pack for
    a stratified ~1/len(templates) of packs, ensuring every template still appears with other
    packs and every pack with other templates. Deterministic given rng."""
    held = set()
    pack_list = sorted(packs)
    for i, pack in enumerate(pack_list):
        # hold out one (template, pack) for every 3rd pack, cycling templates so coverage stays balanced
        if i % 3 == 0:
            t = TEMPLATE_NAMES[i % len(TEMPLATE_NAMES)]
            held.add((t, pack))
    return held


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--per-cell", type=int, default=40, help="training examples per seen cell (summed over templates)")
    ap.add_argument("--seed", type=int, default=80)
    ap.add_argument("--eval-per-cell", type=int, default=12)
    ap.add_argument("--grounding", choices=["descriptor", "behavioral"], default="descriptor")
    ap.add_argument("--max-cells", type=int, default=0, help="restrict to first N value cells (0=all) — learnability diagnostic")
    args = ap.parse_args()
    rng = random.Random(args.seed)

    by_name, held_cells, tok_map, value = load_meta()
    if args.max_cells:
        value = value[: args.max_cells]
    value_names = [r["name"] for r in value]
    packs = {r["pack"] for r in value}
    oracle = Oracle(value_names)

    axis_b_held = held_out_compositions(packs, rng)
    print(f"value cells: {len(value_names)} ({len(held_cells & set(value_names))} axis-A held out)")
    print(f"packs: {len(packs)}; axis-B held-out (template,pack) pairs: {len(axis_b_held)}")

    train_rows = []
    eval_rows = []  # tagged by bucket
    per_cell_written = Counter()
    skipped_unclean = 0

    def gen_n(name, arity, template, pack, n, dest_list, tag, per_cell_key=None):
        """Generate n clean rows for (name, template) into dest_list, tagging with `tag`
        (a (bucket_cell, bucket_comp) pair or None for train). Falls back to safe operands if
        random draws keep escalating, so escalation-prone cells still get covered."""
        nonlocal skipped_unclean
        made, attempts = 0, 0
        while made < n and attempts < n * 8 + 8:
            attempts += 1
            safe = attempts > n * 3  # first try varied regimes, then fall back to safe operands
            ex = make_example(name, arity, template, rng, oracle, pack, grounding=args.grounding, safe=safe)
            if ex is None:
                skipped_unclean += 1
                continue
            ex["cell_id"] = tok_map[f"<cell:{name}>"]
            if tag is not None:
                ex["bucket_cell"], ex["bucket_comp"] = tag
            dest_list.append(ex)
            if per_cell_key is not None:
                per_cell_written[per_cell_key] += 1
            made += 1

    # For each value cell × template, route to train or one of the four eval buckets. A cell in
    # axis-A (held) or a (template, pack) in axis-B (held) is an eval-only combination; an
    # otherwise-seen combination contributes mostly to train plus a reserved slice to the
    # in-distribution seen×seen eval bucket (so all four buckets are populated and disjoint).
    n_per_template = max(1, args.per_cell // len(TEMPLATE_NAMES))
    for r in value:
        name, pack, arity = r["name"], r["pack"], r["arity"]
        cell_held = name in held_cells
        for template in TEMPLATE_NAMES:
            comp_held = (template, pack) in axis_b_held
            if cell_held or comp_held:
                bc = "novel_cell" if cell_held else "seen_cell"
                bk = "novel_comp" if comp_held else "seen_comp"
                gen_n(name, arity, template, pack, args.eval_per_cell, eval_rows, (bc, bk))
            else:
                # seen×seen: reserve a held-out eval slice, rest to train
                gen_n(name, arity, template, pack, args.eval_per_cell, eval_rows, ("seen_cell", "seen_comp"))
                gen_n(name, arity, template, pack, n_per_template, train_rows, None, per_cell_key=name)

    rng.shuffle(train_rows)
    OUT_TRAIN.write_text("\n".join(json.dumps(r) for r in train_rows) + "\n")
    OUT_EVAL.write_text("\n".join(json.dumps(r) for r in eval_rows) + "\n")

    bucket_counts = Counter((e["bucket_cell"], e["bucket_comp"]) for e in eval_rows)
    stats = {
        "seed": args.seed,
        "per_cell": args.per_cell,
        "n_train": len(train_rows),
        "n_eval": len(eval_rows),
        "n_value_cells": len(value_names),
        "n_axis_a_value_held": len(held_cells & set(value_names)),
        "n_packs": len(packs),
        "axis_b_held_pairs": sorted(f"{t}|{p}" for t, p in axis_b_held),
        "templates": TEMPLATE_NAMES,
        "n_demos_per_context": N_DEMOS,
        "skipped_unclean": skipped_unclean,
        "eval_buckets": {f"{c}|{k}": v for (c, k), v in sorted(bucket_counts.items())},
        "train_cells_covered": len(per_cell_written),
        "train_per_cell_min": min(per_cell_written.values()) if per_cell_written else 0,
        "train_per_cell_max": max(per_cell_written.values()) if per_cell_written else 0,
    }
    OUT_STATS.write_text(json.dumps(stats, indent=2))

    print(f"\ntrain rows: {len(train_rows)}  eval rows: {len(eval_rows)}  (unclean skipped: {skipped_unclean})")
    print("eval buckets:")
    for k, v in sorted(stats["eval_buckets"].items()):
        print(f"  {k:<26} {v}")
    print(f"\nwrote {OUT_TRAIN.name}, {OUT_EVAL.name}, {OUT_STATS.name}")
    print("\nsample train row:")
    print(" ", train_rows[0]["text"])


if __name__ == "__main__":
    main()
