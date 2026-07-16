#!/usr/bin/env python3
"""CN-8 band eval (prereg §6) — free-running greedy, mechanistic trace grading, TF-NLL secondary.

Prompts `{A} + {B} =` from cn8_eval_problems.json (seed-90 frozen sets, identical across arms),
greedy decode to the first ` .` or position 256. Headline: exact match of the parsed answer.
Arm B traces are graded per-production against the model's OWN prior state (index / fetch /
table / carry-prop / acc-copy / overflow / readout / truncation; structurally unparseable
segments are reported as `format`, EXCLUDED from the A-rule artifact numerator — conservative).
P5 = per-column conditional correctness on B0. Secondary: teacher-forced NLL (answer span for
A arms, oracle trace for B). Carry-depth strata per band.

Run: python3 cn8_eval.py --ckpt cn8_ckpt_b_s80.pt --format trace
     python3 cn8_eval.py --raw --format answer          (R0 floor)
"""
from __future__ import annotations

import argparse
import json
import math
import re
import time
from pathlib import Path

import torch
import torch.nn.functional as F

import cn1_model
from cn8_corpus import trace_text, SP_MODEL

HERE = Path(__file__).resolve().parent
MAX_SEQ = 256


def wilson(k, n, z=1.96):
    if n == 0:
        return (0.0, 0.0, 1.0)
    p = k / n
    d = 1 + z * z / n
    c = (p + z * z / (2 * n)) / d
    h = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / d
    return (round(p, 4), round(max(0, c - h), 4), round(min(1, c + h), 4))


def carries(a, b):
    c = n = 0
    while a or b:
        n += (a % 10 + b % 10 + c) >= 10
        c = 1 if (a % 10 + b % 10 + c) >= 10 else 0
        a //= 10; b //= 10
    return n


# ---- trace grading (against the model's OWN prior state) ------------------------------

COL_RE = re.compile(r"^c(\d+) (\d)\+(\d)\+(\d)=(\d{1,2}) w(\d) c(\d) a#(\d+)$")
LAB_RE = re.compile(r"([ab])(\d+)#(\d)")


def grade_trace(a: int, b: int, gen: str, truncated: bool):
    """Returns dict: exact, first_error (None if clean), col_ok, col_n, index_ok."""
    A, B = str(a), str(b)
    L = max(len(A), len(B))
    Ap, Bp = A.zfill(L), B.zfill(L)
    expected_labels = {("a", L - 1 - i): Ap[i] for i in range(L)}
    expected_labels |= {("b", L - 1 - i): Bp[i] for i in range(L)}

    out = {"exact": False, "first_error": None, "col_ok": 0, "col_n": 0, "index_ok": False}

    def fail(cls):
        if out["first_error"] is None:
            out["first_error"] = cls

    segs = [s.strip() for s in gen.split("|")]
    segs = [s for s in segs if s != ""]
    if not segs:
        fail("truncation" if truncated else "format")
        return out

    # index line
    if segs[0].startswith("i ") or segs[0] == "i":
        labels = {(m.group(1), int(m.group(2))): m.group(3) for m in LAB_RE.finditer(segs[0])}
        out["index_ok"] = labels == expected_labels and len(LAB_RE.findall(segs[0])) == 2 * L
        if not out["index_ok"]:
            fail("index")
        segs = segs[1:]
    else:
        fail("index")
        labels = {}

    prev_cout, prev_acc, last_acc = "0", "", ""
    ci = 0
    while segs and segs[0].startswith("c") and COL_RE.match(segs[0]):
        m = COL_RE.match(segs[0])
        cnum, x, y, cin, s, w, cout, acc = m.groups()
        col_clean = True
        if int(cnum) != ci:
            fail("format"); col_clean = False
        if labels.get(("a", ci)) != x or labels.get(("b", ci)) != y:
            fail("fetch"); col_clean = False
        if cin != prev_cout:
            fail("carry-prop"); col_clean = False
        if int(s) != int(x) + int(y) + int(cin) or int(w) != int(s) % 10 or int(cout) != int(s) // 10:
            fail("table"); col_clean = False
        if acc != w + prev_acc:
            fail("acc-copy"); col_clean = False
        out["col_n"] += 1
        out["col_ok"] += col_clean
        prev_cout, prev_acc, last_acc = cout, acc, acc
        ci += 1
        segs = segs[1:]

    if ci != L:  # wrong number of column iterations (prereg §6: loop-count)
        fail("truncation" if truncated and not segs else "loop-count")

    # overflow line
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

    # readout
    if segs and re.match(r"^ans \d+ ?\.?$", segs[0]):
        r = re.match(r"^ans (\d+)", segs[0]).group(1)
        if r != last_acc:
            fail("readout")
        out["exact"] = int(r) == a + b if r else False
    else:
        fail("truncation" if truncated else "readout")

    if segs and not (segs[0].startswith("ans") or COL_RE.match(segs[0])):
        if out["first_error"] is None and not out["exact"]:
            fail("format")
    return out


ANS_RE = re.compile(r"^\s*(\d+)\s*\.")


def grade_answer(a: int, b: int, gen: str, truncated: bool):
    m = ANS_RE.match(gen)
    if not m:
        return {"exact": False, "first_error": "truncation" if truncated else "format"}
    return {"exact": int(m.group(1)) == a + b, "first_error": None}


# ---- generation ------------------------------------------------------------------------

@torch.no_grad()
def generate(model, sp, prompts, device, bs=32):
    dot_id = sp.encode(" .")[-1]
    outs = []
    for i in range(0, len(prompts), bs):
        chunk = prompts[i:i + bs]
        enc = [sp.encode(p) for p in chunk]
        plen = len(enc[0])
        assert all(len(e) == plen for e in enc), "band prompts must be equal length"
        ids = torch.tensor(enc, device=device)
        finished = torch.zeros(len(chunk), dtype=torch.bool, device=device)
        while ids.shape[1] < MAX_SEQ and not bool(finished.all()):
            nxt = model(ids)[:, -1].argmax(-1)
            nxt = torch.where(finished, torch.full_like(nxt, dot_id), nxt)
            ids = torch.cat([ids, nxt[:, None]], 1)
            finished |= nxt == dot_id
        fin = finished.tolist()
        for k in range(len(chunk)):
            gen_ids = ids[k, plen:].tolist()
            if dot_id in gen_ids:
                gen_ids = gen_ids[:gen_ids.index(dot_id) + 1]
            outs.append((sp.decode(gen_ids), not fin[k]))
    return outs


@torch.no_grad()
def tf_nll(model, sp, prompt, cont, device):
    p, f = sp.encode(prompt), sp.encode(prompt + cont)
    x = torch.tensor([f], device=device)
    lg = model(x)[0]
    k, n = len(p), len(f) - len(p)
    tgt = x[0, k:k + n]
    return float(F.cross_entropy(lg[k - 1:k - 1 + n], tgt, reduction="mean"))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ckpt")
    ap.add_argument("--raw", action="store_true", help="R0 floor: raw v11, no finetune")
    ap.add_argument("--format", required=True, choices=["trace", "answer"])
    ap.add_argument("--tf-n", type=int, default=60)
    ap.add_argument("--device", default=None)
    args = ap.parse_args()
    assert args.raw != bool(args.ckpt), "exactly one of --ckpt / --raw"
    device = args.device or ("mps" if torch.backends.mps.is_available() else "cpu")
    t0 = time.time()

    import sentencepiece as spm
    sp = spm.SentencePieceProcessor(model_file=SP_MODEL)
    from tiny_model_v11.loader import load_from_artifacts
    base, cfg = load_from_artifacts(str(cn1_model.TINY_MODEL / "model" / "v11"), device="cpu")
    if args.ckpt:
        ck = torch.load(HERE / args.ckpt, map_location="cpu")
        base.load_state_dict(ck["state"])
    base = base.to(device).eval()

    evalsets = json.loads((HERE / "cn8_eval_problems.json").read_text())
    stem = "r0_raw" if args.raw else Path(args.ckpt).stem.replace("cn8_ckpt_", "")
    out = {"ckpt": args.ckpt or "raw_v11", "format": args.format, "bands": {}}

    for band, probs in evalsets.items():
        prompts = [f"{a} + {b} =" for a, b in probs]
        gens = generate(base, sp, prompts, device)
        exact, trunc, strata, fe_hist = 0, 0, {}, {}
        col_ok = col_n = idx_ok = 0
        for (a, b), (gen, truncated) in zip(probs, gens):
            g = (grade_trace(a, b, gen, truncated) if args.format == "trace"
                 else grade_answer(a, b, gen, truncated))
            exact += g["exact"]
            trunc += truncated
            if g["first_error"]:
                fe_hist[g["first_error"]] = fe_hist.get(g["first_error"], 0) + 1
            d = carries(a, b)
            e = strata.setdefault(d, [0, 0])
            e[0] += g["exact"]; e[1] += 1
            if args.format == "trace":
                col_ok += g["col_ok"]; col_n += g["col_n"]; idx_ok += g["index_ok"]
        n = len(probs)
        row = {"n": n, "exact": wilson(exact, n), "exact_k": exact, "truncated": trunc,
               "first_error": dict(sorted(fe_hist.items(), key=lambda kv: -kv[1])),
               "by_carries": {str(k): f"{x}/{m}" for k, (x, m) in sorted(strata.items())}}
        if args.format == "trace":
            row["index_ok"] = round(idx_ok / n, 4)
            row["col_cond_correct"] = (round(col_ok / col_n, 5) if col_n else None)
            row["col_n"] = col_n
        # teacher-forced secondary
        nlls = []
        for a, b in probs[:args.tf_n]:
            if args.format == "answer":
                nlls.append(tf_nll(base, sp, f"{a} + {b} =", f" {a + b} .", device))
            else:
                full = trace_text(a, b)
                nlls.append(tf_nll(base, sp, f"{a} + {b} =", full[len(f"{a} + {b} ="):], device))
        row["tf_nll"] = round(sum(nlls) / len(nlls), 4)
        out["bands"][band] = row
        print(f"  {band}: exact {row['exact'][0]:.3f} [{row['exact'][1]:.3f},{row['exact'][2]:.3f}] "
              f"(k={exact}/{n})  trunc {trunc}  tf_nll {row['tf_nll']:.3f}  "
              f"first_err {row['first_error']}", flush=True)

    path = HERE / f"cn8_eval_{stem}_{args.format}.json"
    path.write_text(json.dumps(out, indent=1))
    print(f"wrote {path.name} ({time.time()-t0:.0f}s)")


if __name__ == "__main__":
    main()
