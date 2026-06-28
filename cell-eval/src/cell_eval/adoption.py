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

from dataclasses import dataclass, field

from .agent import AgentConfig, make_client, parse_answer, run_episode
from .datasets import load_jsonl
from .library import open_library
from .tools import TOOLS

# Back-compat aliases — the names this module has always exported (the loop + config moved
# to `agent.py` so the composition eval can share them).
AdoptionConfig = AgentConfig
_parse_answer = parse_answer

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


def _run_one(client, cfg: AdoptionConfig, lib, task: dict) -> TaskResult:
    ep = run_episode(client, cfg, lib, task["prompt"], SYSTEM_PROMPT, TOOLS)
    expected = int(task["expected"])
    return TaskResult(
        task_id=str(task.get("id", task["prompt"])),
        prompt=task["prompt"],
        expected=expected,
        answer=ep.answer,
        used_cell=bool(ep.trace.cells_run),
        cells_run=ep.trace.cells_run,
        correct=(ep.answer == expected),
        turns=ep.turns,
        error=ep.error,
    )


def run_adoption(
    dataset: str = "tasks",
    library_dir: str | None = None,
    model: str | None = None,
    config: AdoptionConfig | None = None,
) -> AdoptionReport:
    """Run the adoption eval (needs an OpenAI-compatible endpoint; the client is built lazily
    so the retrieval eval and offline tests stay dependency-free)."""
    cfg = config or AdoptionConfig.from_env(model)
    lib = open_library(library_dir)
    client = make_client(cfg)
    report = AdoptionReport(model=cfg.model, base_url=cfg.base_url)
    for task in load_jsonl(dataset):
        report.tasks.append(_run_one(client, cfg, lib, task))
    return report
