#!/usr/bin/env python3
"""CN-7.1 — corpus build (pre-registration §3.2/§4, amended v0.2: SP id space).

Four species, all cell-authored and cell-signed at build time, emitted as id sequences with a
per-token loss mask in the ORIGINAL v11 SP id space (the training mapping — see prereg §8.1):

  S1 tier-a-drill : in-tier arithmetic, 50:50 canonical ("7 + 5 = 12") : narrative (TinyStories
                    register). Full loss. Every answer signed by a library cell.
  S2 interleaved  : TinyStories-register word problems; in-tier steps computed in text (loss),
                    beyond-tier steps emit `<call> ⟨cell⟩ args </call>` and the oracle-computed
                    result is injected AFTER the closing delimiter with ZERO loss on its tokens.
                    Continuation text never restates the injected numeral.
  S3 emission     : CN-6 stage-2 grammar (descriptor + `a b = r ; ...` inside <call>...</call>),
                    k=6 deliberately range-varied pairs per transcript, training cells only.
                    Loss on descriptor/format/inputs; an ANSWER carries loss only if the
                    (cell, args) instance passes the Tier-A instance check below — §3.3's audited
                    property ("no beyond-tier answer token anywhere carries loss") dominates
                    §3.2's shorthand "full loss" row.
  S4 replay       : TinyStories (local HF cache), full loss, 40–50% of the token mix.

Vocab: SP base 0..71260; <call>=71261, </call>=71262, cell tokens 71263+ (sorted by name), map
saved to cn7_token_map.json. Text stores cell tokens as `⟨name⟩`; encode_text() is the single
canonical text->ids path (used by the trainer, the audits, and the evals).

Tier-A instance check (the corpus-side tier function; the frozen 24-cell classification file
governs EVAL stratification, this governs TRAINING loss):
  add/sub: all operands <= 99 · mul: times tables (both <= 12) or 2d x 1d · cmp/minmax/parity:
  operands <= 999 · mod: divisor <= 12, dividend <= 99 · clamp: operands <= 999.
Held-out (axis-A) cells never appear anywhere in the corpus.

Run: python3 cn7_corpus.py [--s1 90000 --s2 25000 --s3-per-cell 45 --replay-frac 0.45]
"""
from __future__ import annotations

import argparse
import json
import random
import re
from pathlib import Path

from cn1_corpus import Oracle, describe
from artifact_paths import dataset_output

HERE = Path(__file__).resolve().parent
SP_MODEL = "/Users/christopherhay/chris-source/chris-experiments/compilation/15_v11_model/v11_tokenizer/v11.model"
CALL_ID, CLOSE_ID = 71261, 71262
CELL_FIRST_ID = 71263

# ---- tier function -------------------------------------------------------------------

# training cells whose ANSWERS may carry loss when the instance is in-tier (kind -> cells)
CELL_KIND = {
    "add_sat": "add", "add3_i16": "add",
    "sub_sat": "sub", "sub_i16": "sub",
    "mul_sat": "mul", "mul_i16": "mul", "unit_mul": "mul",
    "safe_mod": "mod", "excel_mod": "mod",
    "is_lt": "cmp", "is_gt": "cmp", "is_ge": "cmp", "is_le": "cmp", "neq": "cmp",
    "is_even": "cmp", "is_odd": "cmp",
    "min": "cmp", "max": "cmp", "min3": "cmp", "max3": "cmp",
    "min_i16": "cmp", "max_i16": "cmp", "min3_i16": "cmp", "max3_i16": "cmp",
    "mode3": "cmp", "argmin3": "cmp", "argmax3": "cmp",
    "clamp": "clamp",
}


def tier_a_instance(cell: str, args: list[int]) -> bool:
    kind = CELL_KIND.get(cell)
    if kind is None:
        return False
    if kind == "add" or kind == "sub":
        return all(a <= 99 for a in args)
    if kind == "mul":
        a, b = sorted(args[:2])
        return (a <= 12 and b <= 12) or (a <= 9 and b <= 99)
    if kind == "cmp" or kind == "clamp":
        return all(a <= 999 for a in args)
    if kind == "mod":
        return args[1] <= 12 and args[0] <= 99
    return False


# ---- encoder -------------------------------------------------------------------------

_MARK = re.compile(r"(<call>|</call>|⟨[a-z0-9_]+⟩)")


class Enc:
    def __init__(self, cell_ids: dict[str, int]):
        import sentencepiece as spm
        self.sp = spm.SentencePieceProcessor(model_file=SP_MODEL)
        self.cell_ids = cell_ids

    def seg_ids(self, seg: str) -> list[int]:
        if seg == "<call>":
            return [CALL_ID]
        if seg == "</call>":
            return [CLOSE_ID]
        if seg.startswith("⟨") and seg.endswith("⟩"):
            return [self.cell_ids[seg[1:-1]]]
        return self.sp.encode(seg)

    def encode(self, parts: list[tuple[str, int]]) -> tuple[str, list[int], list[int]]:
        """parts = [(text, loss_flag), ...] -> (joined_text, ids, loss). Marker-aware."""
        text, ids, loss = [], [], []
        for t, fl in parts:
            text.append(t)
            for seg in filter(None, _MARK.split(t)):
                si = self.seg_ids(seg)
                ids.extend(si)
                loss.extend([fl] * len(si))
        return "".join(text), ids, loss


# ---- S1: tier-A drill ----------------------------------------------------------------

NAMES = ["Tom", "Lily", "Ben", "Mia", "Sam", "Anna", "Max", "Sue", "Tim", "Amy"]
OBJS = ["apples", "shells", "stones", "berries", "buttons", "stickers", "marbles", "acorns",
        "flowers", "coins"]


def s1_item(rng, oracle):
    op = rng.choice(["add", "add", "sub", "sub", "mul", "mod", "cmp", "parity", "min3", "succ"])
    if op == "add":
        a, b = rng.randint(0, 99), rng.randint(0, 99)
        r = oracle_val(oracle, "add_sat", [a, b])
        can = f"{a} + {b} = {r}"
        n1, n2, o = rng.choice(NAMES), rng.choice(NAMES), rng.choice(OBJS)
        nar = f"{n1} had {a} {o}. {n2} gave {n1} {b} more. Now {n1} has {r} {o}."
    elif op == "sub":
        a = rng.randint(1, 99); b = rng.randint(0, a)
        r = oracle_val(oracle, "sub_sat", [a, b])
        can = f"{a} - {b} = {r}"
        n1, o = rng.choice(NAMES), rng.choice(OBJS)
        nar = f"{n1} had {a} {o} and lost {b}. {n1} has {r} {o} left."
    elif op == "mul":
        if rng.random() < 0.5:
            a, b = rng.randint(0, 12), rng.randint(0, 12)
        else:
            a, b = rng.randint(0, 9), rng.randint(10, 99)
        r = oracle_val(oracle, "mul_sat", [a, b])
        can = f"{a} x {b} = {r}"
        o = rng.choice(OBJS)
        nar = f"There were {a} bags with {b} {o} in each bag. That made {r} {o} in all."
    elif op == "mod":
        a, b = rng.randint(0, 99), rng.randint(2, 12)
        r = oracle_val(oracle, "safe_mod", [a, b])
        can = f"{a} mod {b} = {r}"
        o = rng.choice(OBJS)
        nar = f"{a} {o} were put in rows of {b}. There were {r} {o} left over."
    elif op == "cmp":
        a, b = rng.randint(0, 999), rng.randint(0, 999)
        if a == b:
            b += 1
        lt = oracle_val(oracle, "is_lt", [a, b])
        big, small = (b, a) if lt == 1 else (a, b)
        can = f"{small} < {big}"
        (n1, n2), o = rng.sample(NAMES, 2), rng.choice(OBJS)
        w = n1 if a > b else n2
        nar = f"{n1} found {a} {o} and {n2} found {b} {o}. {w} found more."
    elif op == "parity":
        a = rng.randint(0, 999)
        even = oracle_val(oracle, "is_even", [a])
        can = f"{a} is {'even' if even == 1 else 'odd'}"
        nar = f"{rng.choice(NAMES)} counted {a} {rng.choice(OBJS)}. {a} is an {'even' if even == 1 else 'odd'} number."
    elif op == "min3":
        xs = [rng.randint(0, 999) for _ in range(3)]
        r = oracle_val(oracle, "min3", xs)
        can = f"smallest of {xs[0]}, {xs[1]}, {xs[2]} is {r}"
        nar = f"Three piles had {xs[0]}, {xs[1]} and {xs[2]} {rng.choice(OBJS)}. The smallest pile had {r}."
    else:  # succ
        a = rng.randint(0, 998)
        r = oracle_val(oracle, "add_sat", [a, 1])
        can = f"after {a} comes {r}"
        nar = f"{rng.choice(NAMES)} counted {a}, then {r}."
    text = can if rng.random() < 0.5 else nar
    return [(text, 1)], {"op": op}


def oracle_val(oracle, cell, args):
    r = oracle.run(cell, args)
    assert r.get("halt") == "returned", f"signer {cell}{args} did not return"
    return r["result"]


# ---- S2: interleaved word problems ----------------------------------------------------

# beyond-tier step templates: (cell, arggen, story(args), tail) — tail NEVER contains the result
S2_BEYOND = [
    ("mul_sat", lambda rng: [rng.randint(13, 99), rng.randint(13, 99)],
     lambda a: f"The truck brought {a[0]} crates with {a[1]} apples in each crate. The counting machine worked it out: ",
     " apples in all. Everyone cheered."),
    ("safe_div", lambda rng: [rng.randint(100, 999), rng.randint(3, 19)],
     lambda a: f"{a[0]} sweets were shared fairly between {a[1]} children. The sharing machine said each child gets ",
     " sweets. The children smiled."),
    ("ceil_div", lambda rng: [rng.randint(100, 999), rng.randint(6, 24)],
     lambda a: f"{a[0]} books had to go in boxes of {a[1]}. The packing machine counted the boxes needed: ",
     " boxes. Off they went."),
    ("add_sat", lambda rng: [rng.randint(100, 999), rng.randint(100, 999)],
     lambda a: f"One field grew {a[0]} pumpkins and the other grew {a[1]}. The farm machine added them up: ",
     " pumpkins altogether. What a harvest."),
    ("sub_sat", lambda rng: [rng.randint(500, 999), rng.randint(100, 499)],
     lambda a: f"The shop had {a[0]} balloons and sold {a[1]}. The till machine counted what was left: ",
     " balloons stayed in the shop."),
    ("round_to_multiple", lambda rng: [rng.randint(100, 999), rng.choice([25, 50, 100])],
     lambda a: f"About {a[0]} people came to the fair. Rounded to the nearest {a[1]}, the sign machine wrote ",
     " visitors. The mayor was proud."),
]


def s2_item(rng, oracle, enc):
    parts = []
    # optional in-tier warm-up step (full loss, signed)
    if rng.random() < 0.6:
        a, b = rng.randint(2, 20), rng.randint(2, 20)
        r = oracle_val(oracle, "add_sat", [a, b])
        n = rng.choice(NAMES)
        parts.append((f"{n} picked {a} berries and then {b} more, so {n} had {r} berries. ", 1))
    cell, gen, story, tail = S2_BEYOND[rng.randrange(len(S2_BEYOND))]
    args = gen(rng)
    res = oracle_val(oracle, cell, args)
    parts.append((story(args), 1))
    parts.append((f"<call> ⟨{cell}⟩ {' '.join(map(str, args))} </call> ", 1))
    parts.append((str(res), 0))          # environment-injected, ZERO loss
    parts.append((tail, 1))
    return parts, {"cell": cell, "args": args, "res": res}


# ---- S3: emission transcripts ---------------------------------------------------------

RANGES = [(0, 10), (0, 10), (0, 100), (0, 100), (0, 1000), (0, 1000)]  # k=6, varied


def s3_item(rng, oracle, lib, name):
    arity = lib[name]["arity"]
    pairs = []
    for lo, hi in RANGES:
        for _ in range(12):
            a = [rng.randint(lo, hi) for _ in range(arity)]
            r = oracle.run(name, a)
            if r.get("halt") == "returned":
                pairs.append((a, r["result"]))
                break
        else:
            return None
    rng.shuffle(pairs)
    parts = [(f"{describe(name, lib[name]['pack'])} <call>", 1)]
    for i, (a, o) in enumerate(pairs):
        sep = " ;" if i < len(pairs) - 1 else " </call>"
        parts.append((f" {' '.join(map(str, a))} =", 1))
        parts.append((f" {o}", 1 if tier_a_instance(name, a) else 0))
        parts.append((sep, 1))
    return parts, {"cell": name, "pairs": [[a, o] for a, o in pairs]}


# ---- S4: TinyStories replay ------------------------------------------------------------

def s4_rows(n_tokens, enc, rng):
    from datasets import load_dataset
    ds = load_dataset("roneneldan/TinyStories", split="train", streaming=False)
    idx = list(range(len(ds)))
    rng.shuffle(idx)
    rows, tot = [], 0
    for i in idx:
        txt = ds[int(i)]["text"].strip()
        if not txt:
            continue
        text, ids, loss = enc.encode([(txt, 1)])
        if len(ids) > 256:
            ids, loss = ids[:256], loss[:256]
            text = txt  # truncation is on ids; text kept whole for audit readability
        rows.append({"species": "s4", "text": text, "ids": ids, "loss": loss, "meta": {}})
        tot += len(ids)
        if tot >= n_tokens:
            break
    return rows, tot


# ---- main ------------------------------------------------------------------------------

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--s1", type=int, default=90000)
    ap.add_argument("--s2", type=int, default=25000)
    ap.add_argument("--s3-per-cell", type=int, default=45)
    ap.add_argument("--replay-frac", type=float, default=0.45)
    ap.add_argument("--seed", type=int, default=80)
    args = ap.parse_args()
    rng = random.Random(args.seed)

    lib = {json.loads(l)["name"]: json.loads(l) for l in (HERE / "cn1_library.jsonl").read_text().splitlines() if l.strip()}
    held = {h["name"] for h in json.loads((HERE / "cn1_axis_a_heldout.json").read_text())["held_out_cells"]}
    value = [n for n, r in lib.items() if r["arity"] >= 1]
    train_cells = sorted(n for n in value if n not in held)

    cell_ids = {n: CELL_FIRST_ID + i for i, n in enumerate(sorted(lib))}  # all 790, held-out included
    (HERE / "cn7_token_map.json").write_text(json.dumps(
        {"sp_model": SP_MODEL, "call": CALL_ID, "close": CLOSE_ID, "cell_first_id": CELL_FIRST_ID,
         "vocab": CELL_FIRST_ID + len(cell_ids), "cells": cell_ids}, indent=1))
    enc = Enc(cell_ids)
    oracle = Oracle(sorted(set(list(CELL_KIND) + [c for c, *_ in S2_BEYOND] + train_cells)))

    rows = []

    def emit(species, parts, meta):
        text, ids, loss = enc.encode(parts)
        rows.append({"species": species, "text": text, "ids": ids, "loss": loss, "meta": meta})

    for _ in range(args.s1):
        parts, meta = s1_item(rng, oracle)
        emit("s1", parts, meta)
    for _ in range(args.s2):
        parts, meta = s2_item(rng, oracle, enc)
        emit("s2", parts, meta)
    for name in train_cells:
        made = 0
        for _ in range(args.s3_per_cell * 3):
            if made >= args.s3_per_cell:
                break
            item = s3_item(rng, oracle, lib, name)
            if item:
                emit("s3", *item)
                made += 1

    task_tokens = sum(len(r["ids"]) for r in rows)
    replay_target = int(task_tokens * args.replay_frac / (1 - args.replay_frac))
    s4, s4_tok = s4_rows(replay_target, enc, rng)
    rows.extend(s4)
    rng.shuffle(rows)

    out = dataset_output("cn7_corpus_train.jsonl")
    with out.open("w") as f:
        for r in rows:
            f.write(json.dumps(r) + "\n")

    stats = {}
    for r in rows:
        s = stats.setdefault(r["species"], {"rows": 0, "tokens": 0, "loss_tokens": 0, "masked_tokens": 0})
        s["rows"] += 1
        s["tokens"] += len(r["ids"])
        s["loss_tokens"] += sum(r["loss"])
        s["masked_tokens"] += len(r["loss"]) - sum(r["loss"])
    stats["_total"] = {"rows": len(rows), "tokens": task_tokens + s4_tok,
                       "replay_frac": s4_tok / (task_tokens + s4_tok),
                       "seed": args.seed, "s3_cells": len(train_cells)}
    (HERE / "cn7_corpus_stats.json").write_text(json.dumps(stats, indent=1))
    print(json.dumps(stats, indent=1))
    print(f"wrote {out.name} ({len(rows)} rows)")


if __name__ == "__main__":
    main()
