#!/usr/bin/env python3
"""CN-8 corpus build + audit gate (prereg §3, §5 — cell-native-architectures-cn8-preregistration.md).

Three arm corpora over multi-digit addition, operands 1-4 digits (no leading zeros, d=1 means 1-9):

  B     : scratchpad traces (index-hint grammar, §3.2), ~6M tokens, N_B problems
  A-ex  : the IDENTICAL N_B problems, answer-only ("A + B = R .")
  A-tok : A-ex plus fresh problems until ~6M tokens (token-matched to B)

Eval problem sets (seed 90, frozen): B0 = 4x4 deduped against every arm's training problems,
B1 = 5x5, B2 = 6x6, n=200 each, identical across arms.

Audit gate (§5) — all four must pass or this script raises and nothing trains:
  1. two-route: an independent string/table schoolbook adder re-renders every trace and every
     answer row; texts must match exactly (the generator uses Python int arithmetic — different
     route).
  2. cell signature: every instance with result <= 65535 re-executed through add_sat (u16).
  3. range: operands re-parsed from emitted text; none exceeds 4 digits in any training corpus.
  4. problem identity: B and A-ex carry the same (A, B) multiset.

Run: python3 cn8_corpus.py [--tokens 6000000 --seed 800]
"""
from __future__ import annotations

import argparse
import json
import random
import re
from collections import Counter
from pathlib import Path

from cn1_corpus import Oracle

HERE = Path(__file__).resolve().parent
SP_MODEL = "/Users/christopherhay/chris-source/chris-experiments/compilation/15_v11_model/v11_tokenizer/v11.model"


# ---- generator route: Python int arithmetic ------------------------------------------

def trace_text(a: int, b: int) -> str:
    A, B = str(a), str(b)
    L = max(len(A), len(B))
    Ap, Bp = A.zfill(L), B.zfill(L)
    idx = " ".join(f"a{L-1-i}#{Ap[i]}" for i in range(L)) + " " + \
          " ".join(f"b{L-1-i}#{Bp[i]}" for i in range(L))
    parts = [f"{A} + {B} = | i {idx} |"]
    acc, cin = "", 0
    for c in range(L):
        x, y = int(Ap[L - 1 - c]), int(Bp[L - 1 - c])
        s = x + y + cin
        w, cout = s % 10, s // 10
        acc = str(w) + acc
        parts.append(f" c{c} {x}+{y}+{cin}={s} w{w} c{cout} a#{acc} |")
        cin = cout
    if cin:
        acc = "1" + acc
        parts.append(f" o1 a#{acc} |")
    else:
        parts.append(" o0 |")
    assert int(acc) == a + b and acc[0] != "0", (a, b, acc)
    parts.append(f" ans {acc} .")
    return "".join(parts)


def answer_text(a: int, b: int) -> str:
    return f"{a} + {b} = {a + b} ."


# ---- audit route: string/table schoolbook, no int() on multi-digit values -------------

_DSUM = {}  # (x, y, cin) -> (write_digit, carry_out), built by counting, not by +
for _x in "0123456789":
    for _y in "0123456789":
        for _c in "01":
            _n = "0123456789".index(_x) + "0123456789".index(_y) + "01".index(_c)
            _DSUM[(_x, _y, _c)] = (str(_n % 10), str(_n // 10), str(_n))


def audit_trace_text(A: str, B: str) -> str:
    L = max(len(A), len(B))
    Ap = "0" * (L - len(A)) + A
    Bp = "0" * (L - len(B)) + B
    idx = " ".join(f"a{L-1-i}#{Ap[i]}" for i in range(L)) + " " + \
          " ".join(f"b{L-1-i}#{Bp[i]}" for i in range(L))
    parts = [f"{A} + {B} = | i {idx} |"]
    acc, cin = "", "0"
    for c in range(L):
        x, y = Ap[L - 1 - c], Bp[L - 1 - c]
        w, cout, s = _DSUM[(x, y, cin)]
        acc = w + acc
        parts.append(f" c{c} {x}+{y}+{cin}={s} w{w} c{cout} a#{acc} |")
        cin = cout
    if cin == "1":
        acc = "1" + acc
        parts.append(f" o1 a#{acc} |")
    else:
        parts.append(" o0 |")
    parts.append(f" ans {acc} .")
    return "".join(parts)


def audit_answer_text(A: str, B: str) -> str:
    L = max(len(A), len(B))
    Ap = "0" * (L - len(A)) + A
    Bp = "0" * (L - len(B)) + B
    acc, cin = "", "0"
    for c in range(L):
        w, cout, _ = _DSUM[(Ap[L - 1 - c], Bp[L - 1 - c], cin)]
        acc = w + acc
        cin = cout
    if cin == "1":
        acc = "1" + acc
    return f"{A} + {B} = {acc} ."


# ---- build -----------------------------------------------------------------------------

def draw_operand(rng, d: int) -> int:
    return rng.randint(1, 9) if d == 1 else rng.randint(10 ** (d - 1), 10 ** d - 1)


def draw_problem(rng) -> tuple[int, int]:
    return draw_operand(rng, rng.randint(1, 4)), draw_operand(rng, rng.randint(1, 4))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--tokens", type=int, default=6_000_000)
    ap.add_argument("--seed", type=int, default=800)
    ap.add_argument("--eval-n", type=int, default=200)
    args = ap.parse_args()
    import sentencepiece as spm
    sp = spm.SentencePieceProcessor(model_file=SP_MODEL)
    rng = random.Random(args.seed)

    def rows_for(problems, kind):
        rows = []
        for a, b in problems:
            text = trace_text(a, b) if kind == "trace" else answer_text(a, b)
            ids = sp.encode(text)
            assert len(ids) <= 250, (a, b, len(ids))
            rows.append({"text": text, "ids": ids, "loss": [1] * len(ids), "meta": {"a": a, "b": b}})
        return rows

    # arm B: accumulate scratchpad problems to the token budget
    problems_b, tok = [], 0
    while tok < args.tokens:
        a, b = draw_problem(rng)
        problems_b.append((a, b))
        tok += len(sp.encode(trace_text(a, b)))
    rows_b = rows_for(problems_b, "trace")

    # arm A-ex: identical problems, answer-only
    rows_aex = rows_for(problems_b, "answer")
    aex_tok = sum(len(r["ids"]) for r in rows_aex)

    # arm A-tok: A-ex plus fresh problems to the token budget
    problems_extra, tok2 = [], aex_tok
    while tok2 < args.tokens:
        a, b = draw_problem(rng)
        problems_extra.append((a, b))
        tok2 += len(sp.encode(answer_text(a, b)))
    rows_atok = rows_aex + rows_for(problems_extra, "answer")

    # eval sets (seed 90, frozen; B0 deduped against every training problem)
    trained = set(problems_b) | set(problems_extra)
    erng = random.Random(90)
    evalsets = {}
    for band, d in [("B0", 4), ("B1", 5), ("B2", 6)]:
        got = []
        while len(got) < args.eval_n:
            a, b = draw_operand(erng, d), draw_operand(erng, d)
            if band == "B0" and (a, b) in trained:
                continue
            got.append([a, b])
        evalsets[band] = got

    # ---- audit gate (§5) ----
    n_signed = 0
    oracle = Oracle(["add_sat"])
    for r in rows_b:
        a, b = r["meta"]["a"], r["meta"]["b"]
        assert r["text"] == audit_trace_text(str(a), str(b)), ("two-route trace mismatch", a, b)
        res = oracle.run("add_sat", [a, b])
        assert res.get("halt") == "returned" and res["result"] == a + b, ("add_sat refused", a, b)
        n_signed += 1
    for r in rows_atok:  # includes all of A-ex
        a, b = r["meta"]["a"], r["meta"]["b"]
        assert r["text"] == audit_answer_text(str(a), str(b)), ("two-route answer mismatch", a, b)
        if a + b <= 65535:
            res = oracle.run("add_sat", [a, b])
            assert res.get("halt") == "returned" and res["result"] == a + b, ("add_sat refused", a, b)
            n_signed += 1
    prompt_re = re.compile(r"^(\d+) \+ (\d+) =")
    for rows in (rows_b, rows_atok):
        for r in rows:
            m = prompt_re.match(r["text"])
            assert m and len(m.group(1)) <= 4 and len(m.group(2)) <= 4, ("range audit", r["text"][:40])
    assert Counter(problems_b) == Counter((r["meta"]["a"], r["meta"]["b"]) for r in rows_aex), "identity audit"
    for a, b in evalsets["B0"]:
        assert (a, b) not in trained, ("B0 dedup audit", a, b)

    for name, rows in [("b", rows_b), ("aex", rows_aex), ("atok", rows_atok)]:
        random.Random(7).shuffle(rows)
        with (HERE / f"cn8_corpus_{name}.jsonl").open("w") as f:
            for r in rows:
                f.write(json.dumps(r) + "\n")
    (HERE / "cn8_eval_problems.json").write_text(json.dumps(evalsets, indent=1))

    stats = {arm: {"rows": len(rows), "tokens": sum(len(r["ids"]) for r in rows)}
             for arm, rows in [("b", rows_b), ("aex", rows_aex), ("atok", rows_atok)]}
    stats["_audit"] = {"two_route": "PASS", "cell_signed_instances": n_signed,
                       "range": "PASS", "identity": "PASS", "b0_dedup": "PASS"}
    stats["_seed"] = args.seed
    (HERE / "cn8_corpus_stats.json").write_text(json.dumps(stats, indent=1))
    print(json.dumps(stats, indent=1))


if __name__ == "__main__":
    main()
