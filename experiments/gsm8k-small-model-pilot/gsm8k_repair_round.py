#!/usr/bin/env python3
"""One-shot repair round, mirroring cell-eval's repair.py philosophy (repair.py's own
words): the model gets exactly one shot — its own broken output plus the diagnostic,
no tools, no further retries — and it counts only if the repaired plan runs AND
matches the expected answer (a fix that parses but is still wrong is a miss).

Reuses gsm8k_small_model_pilot.py's PROBLEMS/SYSTEM_PROMPT and pilot_results.json
(the first-shot transcript) so the model sees its own prior turn as real conversation
history, not just a cold restart.
"""
import json
import os
import subprocess
import tempfile
import urllib.request

from gsm8k_small_model_pilot import (
    OLLAMA_URL,
    MODEL,
    CELL80_BIN,
    SYSTEM_PROMPT,
    PROBLEMS,
    extract_json,
    run_solve,
)

PILOT_RESULTS_FILE = os.environ.get("PILOT_RESULTS_FILE", "pilot_results.json")
REPAIR_RESULTS_FILE = os.environ.get("REPAIR_RESULTS_FILE", "repair_results.json")

PROBLEM_MAP = {name: (text, expected) for name, text, expected in PROBLEMS}


def diagnostic_for(r):
    """The one piece of feedback repair.py's philosophy says should carry the signal."""
    if r["stage"] in ("bad_json", "no_json"):
        return f"Your reply was not valid JSON ({r.get('error', 'unparseable')}). Reply with ONLY the JSON plan object, nothing else."
    if r["stage"] == "solve_error":
        return f"Your plan was rejected: {r['detail']}"
    if r["stage"] == "solved" and not r.get("correct"):
        # Need the actual kill reason for render-time kills; re-derive it since the
        # pilot script only stored the top-level answer, not the per-plan kill string.
        plan_path = os.path.join(tempfile.gettempdir(), "_gsm8k_repair_diag.json")
        with open(plan_path, "w") as f:
            json.dump([r["plan"]], f)
        out = subprocess.run(
            [CELL80_BIN, "solve", plan_path, "--json"],
            capture_output=True,
            text=True,
            timeout=30,
        )
        try:
            rep = json.loads(out.stdout)
            kill = rep.get("plans", [{}])[0].get("kill")
        except json.JSONDecodeError:
            kill = None
        if kill:
            return f"Your plan was rejected: {kill}"
        return (
            f"Your plan ran but got the wrong final answer: {r['answer']} "
            f"(the correct answer is {r['expected']}). Re-read the problem carefully — "
            "check you're computing what's actually asked (not just one sub-quantity), "
            "and check your arithmetic/reasoning."
        )
    return None


def ask_repair(problem_text, prior_plan_text, diagnostic):
    messages = [
        {"role": "system", "content": SYSTEM_PROMPT},
        {"role": "user", "content": f'Problem: "{problem_text}"\nPlan:'},
        {"role": "assistant", "content": prior_plan_text},
        {
            "role": "user",
            "content": f"{diagnostic}\n\nFix it. Reply with ONLY the corrected JSON plan object.",
        },
    ]
    body = {"model": MODEL, "messages": messages, "temperature": 0, "stream": False}
    req = urllib.request.Request(
        OLLAMA_URL,
        data=json.dumps(body).encode(),
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=120) as resp:
        data = json.loads(resp.read())
    return data["choices"][0]["message"]["content"]


def main():
    with open(PILOT_RESULTS_FILE) as f:
        first_shot = json.load(f)

    repair_results = []
    for r in first_shot:
        name = r["name"]
        problem_text, expected = PROBLEM_MAP[name]
        if r["stage"] == "solved" and r.get("correct"):
            continue  # already correct first-shot, not part of the repair pool

        diagnostic = diagnostic_for(r)
        prior_plan_text = r.get("raw") or json.dumps(r.get("plan", {}))

        print(f"--- repair: {name} ---", flush=True)
        try:
            raw = ask_repair(problem_text, prior_plan_text, diagnostic)
        except Exception as e:
            repair_results.append({"name": name, "stage": "model_call_error", "detail": str(e)})
            print(f"  MODEL CALL ERROR: {e}")
            continue

        plan_json = extract_json(raw)
        if plan_json is None:
            repair_results.append({"name": name, "stage": "no_json", "raw": raw})
            print(f"  NO JSON EXTRACTED. Raw: {raw[:200]!r}")
            continue
        try:
            plan_obj = json.loads(plan_json)
        except json.JSONDecodeError as e:
            repair_results.append({"name": name, "stage": "bad_json", "raw": plan_json, "error": str(e)})
            print(f"  STILL BAD JSON: {e}")
            continue

        wrapped = json.dumps([plan_obj]) if isinstance(plan_obj, dict) else json.dumps(plan_obj)
        rep = run_solve(wrapped)
        if "error" in rep:
            repair_results.append({"name": name, "stage": "solve_error", "plan": plan_obj, "detail": rep["error"]})
            print(f"  STILL SOLVE ERROR: {rep['error']}")
            continue

        answer = rep.get("answer")
        ok = answer == expected
        repair_results.append({
            "name": name, "stage": "solved", "plan": plan_obj,
            "answer": answer, "expected": expected, "correct": ok,
        })
        print(f"  {'REPAIRED (correct)' if ok else 'STILL WRONG'} (got {answer}, want {expected})")

    print("\n=== REPAIR-ROUND SUMMARY ===")
    n = len(repair_results)
    fixed = sum(1 for r in repair_results if r.get("correct"))
    print(f"attempted repairs: {n}")
    print(f"fixed on repair@1: {fixed}/{n}")
    first_correct = sum(1 for r in first_shot if r["stage"] == "solved" and r.get("correct"))
    print(f"combined (first-shot correct + repaired): {first_correct + fixed}/{len(first_shot)}")

    with open(REPAIR_RESULTS_FILE, "w") as f:
        json.dump(repair_results, f, indent=2)


if __name__ == "__main__":
    main()
