"""The adoption eval — does an LLM agent actually use a cell instead of doing the math?

Drives a model over an **OpenAI-compatible** endpoint (Ollama by default), giving it the
cell tools (`search`/`inspect`/`list`/`run`) and a task phrased as a user request. We then
read three things per task:

* **used_cell**        — did it call `cell_run` at all (adoption), vs. answering from its head?
* **correct**          — is the final numeric answer right?
* **correct_via_cell** — correct *and* it ran a cell (the outcome we actually want).

The *steering* (the system prompt) is a single fixed constant below. The roadmap is
explicit that low adoption is usually weak steering, not bad retrieval — so keep this
constant fixed across runs and vary the library/model instead. If you want to A/B the
steering, change it deliberately and note it; don't let it drift between runs.

Gated: nothing here imports `openai` or touches the network until you call `run_adoption`.
By default it talks to Ollama at http://localhost:11434/v1; override via env or args.
"""

from __future__ import annotations

import json
import os
import re
from dataclasses import dataclass, field

from .datasets import load_jsonl
from .library import open_library
from .tools import TOOLS, ToolTrace, dispatch

# ── steering (HOLD THIS FIXED across runs; see module docstring) ──────────────────
SYSTEM_PROMPT = (
    "You answer numeric questions. You have a library of tiny, verified, deterministic "
    "tools called 'cells', each a small typed function (e.g. gcd, clamp, max). "
    "STRONGLY PREFER finding and running a cell over doing the arithmetic yourself: "
    "the cell's result is verified and you are not. "
    "Workflow: call cell_search to find a candidate, cell_inspect to read its typed "
    "signature so you pass arguments in the right order, then cell_run to execute it. "
    "Only compute a value yourself if no cell fits. "
    "End your reply with a final line exactly of the form 'ANSWER: <integer>'."
)

DEFAULT_BASE_URL = "http://localhost:11434/v1"  # Ollama's OpenAI-compatible endpoint
DEFAULT_API_KEY = "ollama"  # Ollama ignores it, but the SDK requires a non-empty key

_ANSWER_RE = re.compile(r"ANSWER:\s*(-?\d+)", re.IGNORECASE)


@dataclass
class AdoptionConfig:
    model: str
    base_url: str = DEFAULT_BASE_URL
    api_key: str = DEFAULT_API_KEY
    max_turns: int = 8
    temperature: float = 0.0

    @classmethod
    def from_env(cls, model: str | None = None) -> "AdoptionConfig":
        m = model or os.environ.get("CELL_EVAL_MODEL")
        if not m:
            raise ValueError(
                "no model set — pass --model or set CELL_EVAL_MODEL "
                "(e.g. a model you've pulled in Ollama like 'qwen2.5' or 'llama3.1')"
            )
        return cls(
            model=m,
            base_url=os.environ.get("CELL_EVAL_BASE_URL", DEFAULT_BASE_URL),
            api_key=os.environ.get("CELL_EVAL_API_KEY", DEFAULT_API_KEY),
            max_turns=int(os.environ.get("CELL_EVAL_MAX_TURNS", "8")),
        )


@dataclass
class TaskResult:
    task_id: str
    prompt: str
    expected: int
    answer: int | None
    used_cell: bool
    cells_run: list[str]
    correct: bool
    turns: int
    error: str | None = None

    @property
    def correct_via_cell(self) -> bool:
        return self.correct and self.used_cell

    def as_dict(self) -> dict:
        return {
            "task_id": self.task_id,
            "prompt": self.prompt,
            "expected": self.expected,
            "answer": self.answer,
            "used_cell": self.used_cell,
            "cells_run": self.cells_run,
            "correct": self.correct,
            "correct_via_cell": self.correct_via_cell,
            "turns": self.turns,
            "error": self.error,
        }


@dataclass
class AdoptionReport:
    model: str
    base_url: str
    tasks: list[TaskResult] = field(default_factory=list)

    def as_dict(self) -> dict:
        n = len(self.tasks)
        frac = lambda pred: (sum(1 for t in self.tasks if pred(t)) / n) if n else 0.0
        return {
            "eval": "adoption",
            "model": self.model,
            "base_url": self.base_url,
            "overall": {
                "n": n,
                "adoption": round(frac(lambda t: t.used_cell), 4),
                "correct": round(frac(lambda t: t.correct), 4),
                "correct_via_cell": round(frac(lambda t: t.correct_via_cell), 4),
            },
            "tasks": [t.as_dict() for t in self.tasks],
        }


def _parse_answer(text: str) -> int | None:
    matches = _ANSWER_RE.findall(text or "")
    return int(matches[-1]) if matches else None


def _run_one(client, cfg: AdoptionConfig, lib, task: dict) -> TaskResult:
    trace = ToolTrace()
    messages = [
        {"role": "system", "content": SYSTEM_PROMPT},
        {"role": "user", "content": task["prompt"]},
    ]
    answer, turns, error = None, 0, None
    try:
        for turns in range(1, cfg.max_turns + 1):
            resp = client.chat.completions.create(
                model=cfg.model,
                messages=messages,
                tools=TOOLS,
                temperature=cfg.temperature,
            )
            msg = resp.choices[0].message
            tool_calls = msg.tool_calls or []
            # Echo the assistant turn back into the transcript.
            messages.append(
                {
                    "role": "assistant",
                    "content": msg.content or "",
                    "tool_calls": [tc.model_dump() for tc in tool_calls] or None,
                }
            )
            if not tool_calls:
                answer = _parse_answer(msg.content or "")
                break
            for tc in tool_calls:
                try:
                    args = json.loads(tc.function.arguments or "{}")
                except json.JSONDecodeError:
                    args = {}
                result = dispatch(lib, tc.function.name, args, trace)
                messages.append(
                    {
                        "role": "tool",
                        "tool_call_id": tc.id,
                        "content": json.dumps(result),
                    }
                )
    except Exception as e:  # network / endpoint / SDK error — record, don't crash the run
        error = f"{type(e).__name__}: {e}"

    expected = int(task["expected"])
    return TaskResult(
        task_id=str(task.get("id", task["prompt"])),
        prompt=task["prompt"],
        expected=expected,
        answer=answer,
        used_cell=bool(trace.cells_run),
        cells_run=trace.cells_run,
        correct=(answer == expected),
        turns=turns,
        error=error,
    )


def run_adoption(
    dataset: str = "tasks",
    library_dir: str | None = None,
    model: str | None = None,
    config: AdoptionConfig | None = None,
) -> AdoptionReport:
    """Run the adoption eval. Imports `openai` lazily so the package stays usable (and the
    retrieval eval stays dependency-free) when the adoption extra isn't installed."""
    try:
        from openai import OpenAI
    except ImportError as e:
        raise RuntimeError(
            "the adoption eval needs the OpenAI client — install with: "
            "pip install 'cell-eval[adoption]'"
        ) from e

    cfg = config or AdoptionConfig.from_env(model)
    lib = open_library(library_dir)
    client = OpenAI(base_url=cfg.base_url, api_key=cfg.api_key)
    tasks = load_jsonl(dataset)

    report = AdoptionReport(model=cfg.model, base_url=cfg.base_url)
    for task in tasks:
        report.tasks.append(_run_one(client, cfg, lib, task))
    return report
