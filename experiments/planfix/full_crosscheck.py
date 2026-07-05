#!/usr/bin/env python3
"""End-to-end scorecard: run all 20 pilot problems through the structured cross-check
(inline vs library-composed, accept iff they agree). Reports the honest metrics —
  precision  = accepted & correct / all accepted   (the SAFETY number: false-positives must be ~0)
  yield      = accepted & correct / 20
  escalations, split into RECOVERABLE (a path had the right answer) vs genuine.
"""
import pathlib
import sys

from structured_consensus import METHODS, solve

REPO = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "experiments" / "gsm8k-small-model-pilot"))
import gsm8k_small_model_pilot as pilot  # noqa: E402


def run():
    import os
    print(f"model = {os.environ.get('BAKEOFF_MODEL', 'qwen2.5:3b')}   structured cross-check, full 20\n")
    print(f"{'row':16s} {'want':>6s} {'inline':>7s} {'composed':>9s}  verdict")
    print("-" * 78)
    acc_ok = acc_bad = esc_recoverable = esc_genuine = 0
    for name, problem, exp in pilot.PROBLEMS:
        ai, _ = solve(problem, METHODS["inline"])
        ac, _ = solve(problem, METHODS["composed"])
        valid = [a for a in (ai, ac) if a is not None]
        recoverable = exp in (ai, ac)
        if len(valid) == 2 and ai == ac:
            ok = ai == exp
            acc_ok += ok
            acc_bad += (not ok)
            verdict = f"ACCEPT {ai}  {'✓' if ok else '✗ WRONG (false positive!)'}"
        else:
            if recoverable:
                esc_recoverable += 1
                verdict = "escalate (recoverable — a path had it)"
            else:
                esc_genuine += 1
                verdict = "escalate (genuine)"
        print(f"{name:16s} {str(exp):>6s} {str(ai):>7s} {str(ac):>9s}  {verdict}")
    n = len(pilot.PROBLEMS)
    accepted = acc_ok + acc_bad
    print("-" * 78)
    print(f"ACCEPTED: {accepted}/{n}   correct {acc_ok}, WRONG {acc_bad}")
    print(f"precision (accepted correct / accepted): {acc_ok}/{accepted if accepted else '-'}"
          f"{' = %d%%' % (100*acc_ok//accepted) if accepted else ''}")
    print(f"yield (accepted correct / 20): {acc_ok}/{n} ({100*acc_ok//n}%)")
    print(f"escalated: {esc_recoverable + esc_genuine}  (recoverable {esc_recoverable}, genuine {esc_genuine})")
    print(f"\nSAFETY: false positives (accepted-but-wrong) = {acc_bad}   (must be 0)")


if __name__ == "__main__":
    run()
