#!/usr/bin/env python3
"""Crux test for the reframe loop: for rows the model got WRONG by comprehension
(not format), does a deterministic SEARCH over plausible interpretations surface
the correct answer as a *candidate*? If yes, the model's job shrinks from
open extraction to picking among grounded, already-computed options.

Interpretation beam (mechanical, no model): given the model's normalized plan,
enumerate alternative "what is the answer" readings —
  - each single value (leaf or op output) as the target
  - sum of declared leaves / sum of unconsumed sinks / sum of everything
Each becomes a candidate plan; solve() computes it. Then we ask: is `expected`
in the candidate answers?  (The MODEL, given the problem text, then picks.)
"""
import json
import os
import pathlib
import subprocess
import sys
import tempfile

import planfix

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[1]
PILOT_DIR = REPO / "experiments" / "gsm8k-small-model-pilot"
CELL80 = os.environ.get(
    "CELL80_BIN", "/Users/christopherhay/chris-source/cell80/target/release/cell80"
)
sys.path.insert(0, str(PILOT_DIR))
import gsm8k_small_model_pilot as _pilot  # noqa: E402

EXPECTED = {n: e for (n, _t, e) in _pilot.PROBLEMS}
TEXT = {n: t for (n, t, _e) in _pilot.PROBLEMS}
CORPUS = json.loads((PILOT_DIR / "results" / "pilot_results_granite.json").read_text())


def solve_one(plan):
    path = os.path.join(tempfile.gettempdir(), "_cbeam.json")
    with open(path, "w") as f:
        json.dump([plan], f)
    out = subprocess.run([CELL80, "solve", path, "--json"],
                         capture_output=True, text=True, timeout=30)
    if out.returncode != 0:
        return None
    try:
        return json.loads(out.stdout).get("answer")
    except json.JSONDecodeError:
        return None


def with_target(plan, tid):
    p = json.loads(json.dumps(plan))
    p["target"] = tid
    return p


def with_sum(plan, ids, label):
    p = json.loads(json.dumps(plan))
    ids = [i for i in ids if i]
    if len(ids) < 2:
        return None
    acc = ids[0]
    for j, nxt in enumerate(ids[1:], 1):
        out = f"__sum_{label}_{j}"
        p["ops"].append(["add", acc, nxt, out])
        acc = out
    p["target"] = acc
    return p


def interpretation_beam(plan):
    qids = [q["id"] for q in plan["quantities"]]
    outs = [o[3] for o in plan["ops"]]
    all_ids = qids + outs
    consumed = set()
    for o in plan["ops"]:
        consumed.update((o[1], o[2]))
    sinks = [i for i in all_ids if i not in consumed]

    # promoted constants / multipliers carry unit "scalar" — they are not
    # entities to be summed, so exclude them from entity-sum readings.
    scalar_ids = {q["id"] for q in plan["quantities"] if q.get("unit") == "scalar"}
    entities = [i for i in all_ids if i not in scalar_ids]

    cands = {}
    for i in all_ids:
        cands[f"={i}"] = with_target(plan, i)
    for label, group in (("leaves", qids), ("sinks", sinks),
                         ("all", all_ids), ("entities", entities)):
        s = with_sum(plan, group, label)
        if s:
            cands[f"sum_{label}"] = s
    return cands


def run():
    print(f"{'row':16s} {'want':>6s} {'model':>6s}  correct-answer surfaced by the beam?")
    print("-" * 100)
    for rec in CORPUS:
        name = rec["name"]
        if "plan" not in rec:
            continue
        exp = EXPECTED[name]
        model_ans = rec.get("answer")
        if model_ans == exp:
            continue  # already correct — not a comprehension miss
        try:
            plans, _ = planfix.normalize(rec["plan"])
            plan = plans[0]
        except planfix.Reject:
            continue
        beam = interpretation_beam(plan)
        answers = {}
        for label, cand in beam.items():
            a = solve_one(cand)
            if a is not None:
                answers[label] = a
        hit = [lbl for lbl, a in answers.items() if a == exp]
        surfaced = f"YES via {hit}" if hit else "no"
        distinct = sorted({a for a in answers.values()})
        print(f"{name:16s} {str(exp):>6s} {str(model_ans):>6s}  {surfaced}")
        print(f"{'':32s}candidates={dict(list(answers.items())[:8])}")
    print("-" * 100)
    print("If the correct answer is in the candidate set, the model only has to RECOGNISE it")
    print("(given the problem text) — grounded multiple-choice, not blank-page extraction.")


if __name__ == "__main__":
    run()
