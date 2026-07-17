#!/usr/bin/env python3
"""CN-8b corpus build + audit gate (prereg §3/§4 — cell-native-architectures-cn8b-preregistration.md).

Arms over CN-8's IDENTICAL 51,031 problems (read from cn8_corpus_b.jsonl meta):
  B'    : peel-grammar traces, 1 epoch (token count measured and reported)
  A-ex' : answer-only, the same problems repeated/reshuffled to 6.0M tokens (token parity)

Audit gate: (1) two-route independent schoolbook re-render; (2) add_sat signature <= 65535;
(3) range <= 4 digits by independent scan; (4) problem identity vs CN-8; (5) GRAMMAR AUDIT —
piece/production coverage of all 600 oracle eval traces vs the B' training corpus, with the
novelty list required to contain nothing beyond the registered residual (prefix-render
lengths 4-5, loop iterations 5-6).

Run: python3 cn8b_corpus.py
"""
from __future__ import annotations

import json
import random
import re
from collections import Counter
from pathlib import Path

from cn1_corpus import Oracle
from artifact_paths import dataset_input, dataset_output

HERE = Path(__file__).resolve().parent
SP_MODEL = "/Users/christopherhay/chris-source/chris-experiments/compilation/15_v11_model/v11_tokenizer/v11.model"
ATOK_TOKENS = 6_000_000


def enc_ids(sp, text: str) -> list[int]:
    """Canonical text->ids: encode each space-delimited word independently, so segmentation
    can never depend on surrounding context (CN-7 §8.1 family; the grammar audit caught a
    context-flipped ' w8' segmentation pre-training). Single canonical path shared by the
    corpus builder, eval prompts, and TF-NLL."""
    ids = []
    for w in text.split(" "):
        if w:
            ids.extend(sp.encode(" " + w))
    return ids


# ---- generator route (Python int arithmetic) -------------------------------------------

def peel_text(a: int, b: int) -> str:
    A, B = str(a), str(b)
    parts = [f"{A} + {B} = |"]
    Pa, Pb, cin, acc = A, B, 0, ""
    while Pa or Pb:
        x = int(Pa[-1]) if Pa else 0
        y = int(Pb[-1]) if Pb else 0
        Pa, Pb = Pa[:-1], Pb[:-1]
        s = x + y + cin
        w, cout = s % 10, s // 10
        acc = str(w) + acc
        parts.append(f" {Pa or '-'} {Pb or '-'} {x}+{y}+{cin}={s} w{w} c{cout} a#{acc} |")
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


# ---- audit route (string/table schoolbook, no int() on multi-digit values) --------------

_DSUM = {}
for _x in "0123456789":
    for _y in "0123456789":
        for _c in "01":
            _n = "0123456789".index(_x) + "0123456789".index(_y) + "01".index(_c)
            _DSUM[(_x, _y, _c)] = (str(_n % 10), str(_n // 10), str(_n))


def audit_peel_text(A: str, B: str) -> str:
    parts = [f"{A} + {B} = |"]
    Pa, Pb, cin, acc = A, B, "0", ""
    while Pa or Pb:
        x = Pa[-1] if Pa else "0"
        y = Pb[-1] if Pb else "0"
        Pa, Pb = Pa[:-1], Pb[:-1]
        w, cout, s = _DSUM[(x, y, cin)]
        acc = w + acc
        parts.append(f" {Pa or '-'} {Pb or '-'} {x}+{y}+{cin}={s} w{w} c{cout} a#{acc} |")
        cin = cout
    if cin == "1":
        acc = "1" + acc
        parts.append(f" o1 a#{acc} |")
    else:
        parts.append(" o0 |")
    parts.append(f" ans {acc} .")
    return "".join(parts)


def audit_answer_text(A: str, B: str) -> str:
    Pa, Pb, cin, acc = A, B, "0", ""
    while Pa or Pb:
        x = Pa[-1] if Pa else "0"
        y = Pb[-1] if Pb else "0"
        Pa, Pb = Pa[:-1], Pb[:-1]
        w, cout, _ = _DSUM[(x, y, cin)]
        acc = w + acc
        cin = cout
    if cin == "1":
        acc = "1" + acc
    return f"{A} + {B} = {acc} ."


# ---- grammar-audit helpers ---------------------------------------------------------------

COL_RE = re.compile(r" ([0-9]+|-) ([0-9]+|-) (\d)\+(\d)\+(\d)=(\d{1,2}) w\d c\d a#\d+ \|")


def productions(text: str):
    """Extract production shapes from a peel trace: digit triples, prefix render lengths,
    boundary configs, loop iteration count."""
    triples, preflens, bounds = set(), set(), set()
    cols = COL_RE.findall(text)
    for pa, pb, x, y, cin, _s in cols:
        triples.add((x, y, cin))
        preflens.add(0 if pa == "-" else len(pa))
        preflens.add(0 if pb == "-" else len(pb))
        bounds.add((pa == "-", pb == "-"))
    return triples, preflens, bounds, len(cols)


def main():
    import sentencepiece as spm
    sp = spm.SentencePieceProcessor(model_file=SP_MODEL)

    cn8_rows = [json.loads(l) for l in dataset_input("cn8_corpus_b.jsonl").read_text().splitlines() if l.strip()]
    problems = [(r["meta"]["a"], r["meta"]["b"]) for r in cn8_rows]
    assert len(problems) == 51031, len(problems)

    def rows_for(probs, kind):
        rows = []
        for a, b in probs:
            text = peel_text(a, b) if kind == "peel" else answer_text(a, b)
            ids = enc_ids(sp, text)
            assert len(ids) <= 250, (a, b, len(ids))
            rows.append({"text": text, "ids": ids, "loss": [1] * len(ids), "meta": {"a": a, "b": b}})
        return rows

    rows_b = rows_for(problems, "peel")
    b_tokens = sum(len(r["ids"]) for r in rows_b)

    # A-ex': repeat/reshuffle the same problems to ATOK_TOKENS
    arng = random.Random(8080)
    rows_aex, tok = [], 0
    while tok < ATOK_TOKENS:
        order = list(problems)
        arng.shuffle(order)
        for a, b in order:
            text = answer_text(a, b)
            ids = enc_ids(sp, text)
            rows_aex.append({"text": text, "ids": ids, "loss": [1] * len(ids), "meta": {"a": a, "b": b}})
            tok += len(ids)
            if tok >= ATOK_TOKENS:
                break

    # ---- audits ----
    oracle = Oracle(["add_sat"])
    n_signed = 0
    for r in rows_b:
        a, b = r["meta"]["a"], r["meta"]["b"]
        assert r["text"] == audit_peel_text(str(a), str(b)), ("two-route peel mismatch", a, b)
        res = oracle.run("add_sat", [a, b])
        assert res.get("halt") == "returned" and res["result"] == a + b, ("add_sat refused", a, b)
        n_signed += 1
    seen_answer_audit = set()
    for r in rows_aex:
        a, b = r["meta"]["a"], r["meta"]["b"]
        if (a, b) not in seen_answer_audit:  # text is identical across repeats
            assert r["text"] == audit_answer_text(str(a), str(b)), ("two-route answer mismatch", a, b)
            seen_answer_audit.add((a, b))
    prompt_re = re.compile(r"^(\d+) \+ (\d+) =")
    for rows in (rows_b, rows_aex):
        for r in rows:
            m = prompt_re.match(r["text"])
            assert m and len(m.group(1)) <= 4 and len(m.group(2)) <= 4, ("range audit", r["text"][:40])
    assert Counter((r["meta"]["a"], r["meta"]["b"]) for r in rows_b) == Counter(problems), "identity audit B'"
    assert set((r["meta"]["a"], r["meta"]["b"]) for r in rows_aex) == set(problems), "identity audit A-ex'"

    # grammar audit (§4.5)
    train_pieces = set()
    tr_triples, tr_preflens, tr_bounds, tr_maxloop = set(), set(), set(), 0
    for r in rows_b:
        train_pieces.update(r["ids"])
        t3, pl, bd, nc = productions(r["text"])
        tr_triples |= t3; tr_preflens |= pl; tr_bounds |= bd; tr_maxloop = max(tr_maxloop, nc)
    evalsets = json.loads((HERE / "cn8_eval_problems.json").read_text())
    novel = {"pieces": set(), "triples": set(), "preflens": set(), "bounds": set(), "loops": set()}
    for band, probs in evalsets.items():
        for a, b in probs:
            t = peel_text(a, b)
            novel["pieces"] |= set(enc_ids(sp, t)) - train_pieces
            t3, pl, bd, nc = productions(t)
            novel["triples"] |= t3 - tr_triples
            novel["preflens"] |= pl - tr_preflens
            novel["bounds"] |= bd - tr_bounds
            if nc > tr_maxloop:
                novel["loops"].add(nc)
    assert not novel["pieces"], f"grammar audit: novel SP pieces {novel['pieces']}"
    assert not novel["triples"], f"grammar audit: novel digit triples {novel['triples']}"
    assert not novel["bounds"], f"grammar audit: novel boundary configs {novel['bounds']}"
    assert novel["preflens"] <= {4, 5}, f"grammar audit: unexpected prefix lengths {novel['preflens']}"
    assert novel["loops"] <= {5, 6}, f"grammar audit: unexpected loop counts {novel['loops']}"

    for name, rows in [("b", rows_b), ("aex", rows_aex)]:
        random.Random(7).shuffle(rows)
        with dataset_output(f"cn8b_corpus_{name}.jsonl").open("w") as f:
            for r in rows:
                f.write(json.dumps(r) + "\n")

    stats = {
        "b": {"rows": len(rows_b), "tokens": b_tokens},
        "aex": {"rows": len(rows_aex), "tokens": tok,
                "epochs": round(tok / sum(len(enc_ids(sp, answer_text(a, b))) for a, b in problems), 2)},
        "_audit": {"two_route": "PASS", "cell_signed_instances": n_signed, "range": "PASS",
                   "identity": "PASS",
                   "grammar": {"novel_pieces": 0, "novel_triples": 0, "novel_bounds": 0,
                               "residual_preflens": sorted(novel["preflens"]),
                               "residual_loops": sorted(novel["loops"]),
                               "verdict": "PASS — nothing beyond the registered residual"}},
    }
    (HERE / "cn8b_corpus_stats.json").write_text(json.dumps(stats, indent=1))
    print(json.dumps(stats, indent=1))


if __name__ == "__main__":
    main()
