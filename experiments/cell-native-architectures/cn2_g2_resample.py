#!/usr/bin/env python3
"""CN-2 G2 — verified decoding WITH resample-on-mismatch (the correction loop).

The measurement-only harness (cn2_verified_decoding.py) established the
baseline: wrong_number_rate 0.016 on the 60-problem battery, with every span
re-derived exactly by cell80's plan IR (i32 signed lane). This harness closes
the loop, per the CN-2 spec's G2 design (experiments/cell-native-
architectures.md): when cell80 refutes a span, the completion is truncated at
the refuted claim, the *verified* equation is asserted in its place, and the
model continues from there — greedy decoding, so the whole pipeline stays
deterministic. Repeat until no refuted span remains (or a round cap).

This is the harness-level slice of G2: the correction happens between
requests, not inside LARQL's decode loop (that's the in-decoder follow-up).
The continuation uses /v1/completions with the Gemma chat template rendered
exactly the way larql-server's chat route renders it (system turn + user turn
+ model turn open), so the corrected prefix sits in the same token context
the baseline completion was decoded in.

Measured: wrong-span count and final-answer accuracy before vs after the
correction loop, rounds used, and the wall-clock cost of correction.

Prereq (same as the baseline harness):
  LARQL_SPIN_POOL=0 larql serve \
    /Users/christopherhay/chris-source/larql/output/gemma3-4b-q4k-v2.vindex --port 8080

Run: python3 cn2_g2_resample.py [--url http://localhost:8080]
"""
from __future__ import annotations

import argparse
import json
import re
import time
from pathlib import Path

import cell80_py
import requests

from cn2_verified_decoding import BATTERY, SYSTEM, chat, extract_spans, verify_span

OUT = Path(__file__).resolve().parent / "cn2_g2_resample_results.json"

FINAL_RE = re.compile(r"Answer:\s*(-?\d+(?:\.\d+)?)")


def gemma_prompt(question: str) -> str:
    # Byte-for-byte the rendering larql-server's chat route produces for
    # [{system}, {user}] (GemmaRenderer + assistant_open) - the corrected
    # prefix must continue in the same token context the baseline decoded in.
    return (
        f"<start_of_turn>system\n{SYSTEM}<end_of_turn>\n"
        f"<start_of_turn>user\n{question}<end_of_turn>\n"
        f"<start_of_turn>model\n"
    )


def continue_completion(url: str, question: str, partial: str, max_tokens: int, timeout: int) -> str:
    payload = {
        "prompt": gemma_prompt(question) + partial,
        "max_tokens": max_tokens,
        "temperature": 0.0,
        "stream": False,
        "stop": ["<end_of_turn>"],
    }
    r = requests.post(f"{url}/v1/completions", json=payload, timeout=timeout)
    r.raise_for_status()
    return r.json()["choices"][0]["text"] or ""


def verified_spans(host: cell80_py.CellHost, text: str) -> list[dict]:
    return [verify_span(host, s) for s in extract_spans(text)]


def first_refuted(spans: list[dict]) -> dict | None:
    return next((s for s in spans if s["verdict"] == "mismatch"), None)


def final_answer(text: str):
    m = FINAL_RE.search(text)
    return float(m.group(1)) if m else None


def run_problem(host, url, item, max_tokens, timeout, max_rounds) -> dict:
    t0 = time.time()
    baseline = chat(url, item["q"], max_tokens, timeout)
    t_baseline = time.time() - t0

    spans_before = verified_spans(host, baseline)
    text = baseline
    corrections = []
    t1 = time.time()
    while len(corrections) < max_rounds:
        bad = first_refuted(verified_spans(host, text))
        if bad is None:
            break
        # Truncate at the refuted claim, assert the verified equation in the
        # model's own notation, and let it continue from there.
        corrected = f"{bad['a']} {bad['sym']} {bad['b']} = {bad['cell80_answer']}"
        prefix = text[: bad["start"]] + corrected
        corrections.append({
            "refuted_line": bad["line"],
            "claimed": bad["c"],
            "verified": bad["cell80_answer"],
            "corrected_to": corrected,
        })
        text = prefix + continue_completion(url, item["q"], prefix, max_tokens, timeout)
    t_correction = time.time() - t1

    spans_after = verified_spans(host, text)
    fb, fa = final_answer(baseline), final_answer(text)
    expected = float(item["final"])
    return {
        "question": item["q"],
        "final_expected": item["final"],
        "baseline_completion": baseline,
        "final_completion": text if corrections else None,  # None = unchanged
        "corrections": corrections,
        "spans_before": spans_before,
        "spans_after": spans_after,
        "final_stated_before": fb,
        "final_stated_after": fa,
        "final_correct_before": fb is not None and fb == expected,
        "final_correct_after": fa is not None and fa == expected,
        "t_baseline_s": round(t_baseline, 2),
        "t_correction_s": round(t_correction, 2),
    }


def summarize(rows: list[dict]) -> dict:
    def span_stats(key):
        spans = [s for r in rows for s in r[key]]
        n = len(spans)
        mm = sum(1 for s in spans if s["verdict"] == "mismatch")
        return {
            "n_spans": n,
            "n_mismatch": mm,
            "n_escalated": sum(1 for s in spans if s["verdict"] == "escalated"),
            "wrong_number_rate": round(mm / n, 3) if n else None,
        }

    corrected_rows = [r for r in rows if r["corrections"]]
    return {
        "n_problems": len(rows),
        "before": {
            **span_stats("spans_before"),
            "final_answer_accuracy": round(
                sum(r["final_correct_before"] for r in rows) / len(rows), 3
            ),
        },
        "after": {
            **span_stats("spans_after"),
            "final_answer_accuracy": round(
                sum(r["final_correct_after"] for r in rows) / len(rows), 3
            ),
        },
        "n_problems_corrected": len(corrected_rows),
        "n_corrections_total": sum(len(r["corrections"]) for r in rows),
        "finals_flipped_right": sum(
            1 for r in corrected_rows if r["final_correct_after"] and not r["final_correct_before"]
        ),
        "finals_flipped_wrong": sum(
            1 for r in corrected_rows if r["final_correct_before"] and not r["final_correct_after"]
        ),
        "t_baseline_total_s": round(sum(r["t_baseline_s"] for r in rows), 1),
        "t_correction_total_s": round(sum(r["t_correction_s"] for r in rows), 1),
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--url", default="http://localhost:8080")
    ap.add_argument("--max-tokens", type=int, default=120)
    ap.add_argument("--limit", type=int, default=None)
    ap.add_argument("--timeout", type=int, default=600)
    ap.add_argument("--max-rounds", type=int, default=4)
    args = ap.parse_args()

    host = cell80_py.CellHost()
    host.set_cache(True)

    battery = BATTERY[: args.limit] if args.limit else BATTERY
    t0 = time.time()
    rows = []
    for i, item in enumerate(battery):
        try:
            row = run_problem(host, args.url, item, args.max_tokens, args.timeout, args.max_rounds)
        except requests.RequestException as e:
            print(f"[{i}] HTTP error: {e}", flush=True)
            rows.append({
                "question": item["q"], "final_expected": item["final"], "error": str(e),
                "corrections": [], "spans_before": [], "spans_after": [],
                "final_correct_before": False, "final_correct_after": False,
                "t_baseline_s": 0.0, "t_correction_s": 0.0,
            })
            continue
        rows.append(row)
        tag = f", {len(row['corrections'])} corrected" if row["corrections"] else ""
        print(
            f"[{i}] {len(row['spans_before'])} spans{tag}, "
            f"final {row['final_correct_before']}->{row['final_correct_after']}  "
            f"({time.time()-t0:.1f}s)",
            flush=True,
        )

    summary = {"url": args.url, **summarize(rows)}
    OUT.write_text(json.dumps({"summary": summary, "rows": rows}, indent=2))
    print("\n=== CN-2 G2 summary (resample-on-mismatch) ===")
    print(json.dumps(summary, indent=2))
    print(f"\nwrote {OUT} ({time.time()-t0:.1f}s)")


if __name__ == "__main__":
    main()
