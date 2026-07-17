#!/usr/bin/env python3
"""CN-8b band eval (prereg §5) — peel-grammar grading; protocol otherwise identical to cn8_eval.

Production classes: {peel-copy, fetch, table, carry-prop, acc-copy, overflow, readout,
loop-count, format, truncation}, each graded against the model's OWN prior state.

Run: python3 cn8b_eval.py --ckpt cn8b_ckpt_bp_s80.pt --format trace
     python3 cn8b_eval.py --ckpt cn8b_ckpt_aexp_s80.pt --format answer
"""
from __future__ import annotations

import argparse
import json
import re
import time
from pathlib import Path

import torch

import cn1_model
from cn8b_corpus import peel_text, SP_MODEL
from cn8_eval import wilson, carries, generate, tf_nll, grade_answer

HERE = Path(__file__).resolve().parent

COL_RE = re.compile(r"^([0-9]+|-) ([0-9]+|-) (\d)\+(\d)\+(\d)=(\d{1,2}) w(\d) c(\d) a#(\d+)$")


def grade_peel(a: int, b: int, gen: str, truncated: bool):
    A, B = str(a), str(b)
    L = max(len(A), len(B))
    out = {"exact": False, "first_error": None, "col_ok": 0, "col_n": 0}

    def fail(cls):
        if out["first_error"] is None:
            out["first_error"] = cls

    segs = [s.strip() for s in gen.split("|")]
    segs = [s for s in segs if s != ""]
    if not segs:
        fail("truncation" if truncated else "format")
        return out

    prev_pa, prev_pb, prev_cout, prev_acc, last_acc = A, B, "0", "", ""
    ci = 0
    while segs and COL_RE.match(segs[0]):
        pa, pb, x, y, cin, s, w, cout, acc = COL_RE.match(segs[0]).groups()
        col_clean = True
        exp_pa = prev_pa[:-1] if prev_pa not in ("", "-") else ""
        exp_pb = prev_pb[:-1] if prev_pb not in ("", "-") else ""
        if pa != (exp_pa or "-") or pb != (exp_pb or "-"):
            fail("peel-copy"); col_clean = False
        exp_x = prev_pa[-1] if prev_pa not in ("", "-") else "0"
        exp_y = prev_pb[-1] if prev_pb not in ("", "-") else "0"
        if x != exp_x or y != exp_y:
            fail("fetch"); col_clean = False
        if cin != prev_cout:
            fail("carry-prop"); col_clean = False
        if int(s) != int(x) + int(y) + int(cin) or int(w) != int(s) % 10 or int(cout) != int(s) // 10:
            fail("table"); col_clean = False
        if acc != w + prev_acc:
            fail("acc-copy"); col_clean = False
        out["col_n"] += 1
        out["col_ok"] += col_clean
        prev_pa, prev_pb, prev_cout, prev_acc, last_acc = pa, pb, cout, acc, acc
        ci += 1
        segs = segs[1:]

    if ci != L:
        fail("truncation" if truncated and not segs else "loop-count")

    if segs and re.match(r"^o[01]( a#\d+)?$", segs[0]):
        ob = segs[0][1]
        if ob != prev_cout:
            fail("overflow")
        if ob == "1":
            m = re.match(r"^o1 a#(\d+)$", segs[0])
            if not m or m.group(1) != "1" + prev_acc:
                fail("overflow")
            else:
                last_acc = m.group(1)
        segs = segs[1:]
    else:
        fail("truncation" if truncated and not segs else "overflow")

    if segs and re.match(r"^ans \d+ ?\.?$", segs[0]):
        r = re.match(r"^ans (\d+)", segs[0]).group(1)
        if r != last_acc:
            fail("readout")
        out["exact"] = int(r) == a + b
    else:
        fail("truncation" if truncated else "readout")
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ckpt")
    ap.add_argument("--raw", action="store_true")
    ap.add_argument("--format", required=True, choices=["trace", "answer"])
    ap.add_argument("--tf-n", type=int, default=60)
    ap.add_argument("--limit", type=int, default=None)
    ap.add_argument("--device", default=None)
    args = ap.parse_args()
    assert args.raw != bool(args.ckpt), "exactly one of --ckpt / --raw"
    device = args.device or ("mps" if torch.backends.mps.is_available() else "cpu")
    t0 = time.time()

    import sentencepiece as spm
    sp = spm.SentencePieceProcessor(model_file=SP_MODEL)
    from tiny_model_v11.loader import load_from_artifacts
    base, _ = load_from_artifacts(str(cn1_model.TINY_MODEL / "model" / "v11"), device="cpu")
    if args.ckpt:
        ck = torch.load(HERE / args.ckpt, map_location="cpu")
        base.load_state_dict(ck["state"])
    base = base.to(device).eval()

    evalsets = json.loads((HERE / "cn8_eval_problems.json").read_text())
    stem = "r0_raw" if args.raw else Path(args.ckpt).stem.replace("cn8b_ckpt_", "")
    out = {"ckpt": args.ckpt or "raw_v11", "format": args.format, "bands": {}}

    for band, probs in evalsets.items():
        if args.limit:
            probs = probs[:args.limit]
        prompts = [f"{a} + {b} =" for a, b in probs]
        gens = generate(base, sp, prompts, device)
        exact, trunc, strata, fe_hist = 0, 0, {}, {}
        col_ok = col_n = 0
        for (a, b), (gen, truncated) in zip(probs, gens):
            g = (grade_peel(a, b, gen, truncated) if args.format == "trace"
                 else grade_answer(a, b, gen, truncated))
            exact += g["exact"]
            trunc += truncated
            if g["first_error"]:
                fe_hist[g["first_error"]] = fe_hist.get(g["first_error"], 0) + 1
            d = carries(a, b)
            e = strata.setdefault(d, [0, 0])
            e[0] += g["exact"]; e[1] += 1
            if args.format == "trace":
                col_ok += g["col_ok"]; col_n += g["col_n"]
        n = len(probs)
        row = {"n": n, "exact": wilson(exact, n), "exact_k": exact, "truncated": trunc,
               "first_error": dict(sorted(fe_hist.items(), key=lambda kv: -kv[1])),
               "by_carries": {str(k): f"{x}/{m}" for k, (x, m) in sorted(strata.items())}}
        if args.format == "trace":
            row["col_cond_correct"] = (round(col_ok / col_n, 5) if col_n else None)
            row["col_n"] = col_n
        nlls = []
        for a, b in probs[:args.tf_n]:
            if args.format == "answer":
                nlls.append(tf_nll(base, sp, f"{a} + {b} =", f" {a + b} .", device))
            else:
                full = peel_text(a, b)
                nlls.append(tf_nll(base, sp, f"{a} + {b} =", full[len(f"{a} + {b} ="):], device))
        row["tf_nll"] = round(sum(nlls) / len(nlls), 4)
        out["bands"][band] = row
        print(f"  {band}: exact {row['exact'][0]:.3f} [{row['exact'][1]:.3f},{row['exact'][2]:.3f}] "
              f"(k={exact}/{n})  trunc {trunc}  tf_nll {row['tf_nll']:.3f}  "
              f"first_err {row['first_error']}", flush=True)

    path = HERE / f"cn8b_eval_{stem}_{args.format}.json"
    path.write_text(json.dumps(out, indent=1))
    print(f"wrote {path.name} ({time.time()-t0:.0f}s)")


if __name__ == "__main__":
    main()
