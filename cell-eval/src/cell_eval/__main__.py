"""CLI: `cell-eval retrieval` and `cell-eval adoption`.

    cell-eval retrieval                  # deterministic, runs anywhere, no LLM
    cell-eval retrieval --k 3 --json
    cell-eval adoption --model qwen2.5   # talks to Ollama at :11434 by default
    cell-eval adoption --model llama3.1 --base-url http://host:11434/v1

Exit code is 0 on a clean run regardless of scores — these are measurements, not pass/fail
gates. (Use `--fail-under` on retrieval if you want to wire it into CI as a guard.)
"""

from __future__ import annotations

import argparse
import json
import sys


def _cmd_retrieval(args) -> int:
    from .report import render_retrieval
    from .retrieval import run_retrieval

    report = run_retrieval(dataset=args.dataset, library_dir=args.library, k=args.k)
    if args.json:
        print(json.dumps(report.as_dict(), indent=2))
    else:
        print(render_retrieval(report))
    if args.fail_under is not None and report.overall.precision_at_1 < args.fail_under:
        print(
            f"\nP@1 {report.overall.precision_at_1:.3f} < --fail-under {args.fail_under}",
            file=sys.stderr,
        )
        return 1
    return 0


def _cmd_adoption(args) -> int:
    from .adoption import run_adoption
    from .report import render_adoption

    try:
        report = run_adoption(
            dataset=args.dataset, library_dir=args.library, model=args.model
        )
    except (ValueError, RuntimeError) as e:
        # No model configured, or the OpenAI client isn't installed — config errors,
        # not crashes. (Per-task network/endpoint errors are recorded in the report.)
        print(f"cell-eval adoption: {e}", file=sys.stderr)
        return 2
    if args.json:
        print(json.dumps(report.as_dict(), indent=2))
    else:
        print(render_adoption(report))
    return 0


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(prog="cell-eval", description="cell80 agent eval harness")
    p.add_argument("--library", default=None, help="cells dir (default: seed cell80/cells)")
    sub = p.add_subparsers(dest="cmd", required=True)

    r = sub.add_parser("retrieval", help="deterministic retrieval-precision eval")
    r.add_argument("--dataset", default="retrieval")
    r.add_argument("--k", type=int, default=5, help="top-k window (default 5)")
    r.add_argument("--json", action="store_true")
    r.add_argument("--fail-under", type=float, default=None, help="exit 1 if P@1 below this")
    r.set_defaults(func=_cmd_retrieval)

    a = sub.add_parser("adoption", help="LLM adoption eval (OpenAI-compatible / Ollama)")
    a.add_argument("--dataset", default="tasks")
    a.add_argument("--model", default=None, help="model name (or set CELL_EVAL_MODEL)")
    a.add_argument("--json", action="store_true")
    a.set_defaults(func=_cmd_adoption)

    args = p.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
