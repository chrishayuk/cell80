"""The composition eval — does an agent *compose* cells instead of writing code?

The capstone of the thesis. The adoption eval asks "did it run the *right cell*"; this asks
the harder question: given a task that needs **several** cells, does the agent wire them
together (via `cell_graph_run`, or by chaining runs) rather than doing the multi-step
arithmetic itself? Same fixed-steering discipline as adoption — one constant prompt below.

Three numbers per task:

* **composed**                  — did it wire cells together (a `cell_graph_run` with ≥2 nodes,
                                  or ≥2 distinct cells run)?
* **correct**                   — is the final `ANSWER: <n>` right?
* **correct_via_composition**   — correct *and* it composed (the outcome we want).

Gated behind an OpenAI-compatible endpoint (Ollama by default); nothing touches the network
until `run_composition`.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from .agent import AgentConfig, make_client, run_episode
from .datasets import load_jsonl
from .library import open_library
from .tools import GRAPH_TOOL, TOOLS

# Composition gets the base cell tools PLUS the graph tool.
COMPOSITION_TOOLS = TOOLS + [GRAPH_TOOL]

# ── steering (HOLD FIXED across runs) ─────────────────────────────────────────────
SYSTEM_PROMPT = (
    "You answer numeric questions that usually need SEVERAL steps. You have a library of "
    "tiny, verified, deterministic cells (e.g. manhattan, weighted_sum, clamp, range_check). "
    "STRONGLY PREFER composing cells over doing the arithmetic yourself — the cells are "
    "verified and you are not. Discover cells with cell_search and read their typed "
    "signatures with cell_inspect, then wire them into ONE graph and run it with "
    "cell_graph_run: each node names a cell; each wire feeds a node input port from a "
    "constant, an external input, or another node's output port (output ports are 'result' "
    "plus any state fields). The host type-checks the whole graph and returns the outputs. "
    "End your reply with a final line exactly of the form 'ANSWER: <integer>'."
)


@dataclass
class TaskResult:
    task_id: str
    prompt: str
    expected: int
    answer: int | None
    composed: bool
    used_graph: bool
    cells_run: list[str]
    correct: bool
    turns: int
    error: str | None = None

    @property
    def correct_via_composition(self) -> bool:
        return self.correct and self.composed

    def as_dict(self) -> dict:
        return {
            "task_id": self.task_id,
            "prompt": self.prompt,
            "expected": self.expected,
            "answer": self.answer,
            "composed": self.composed,
            "used_graph": self.used_graph,
            "cells_run": self.cells_run,
            "correct": self.correct,
            "correct_via_composition": self.correct_via_composition,
            "turns": self.turns,
            "error": self.error,
        }


@dataclass
class CompositionReport:
    model: str
    base_url: str
    tasks: list[TaskResult] = field(default_factory=list)

    def as_dict(self) -> dict:
        n = len(self.tasks)
        frac = lambda pred: (sum(1 for t in self.tasks if pred(t)) / n) if n else 0.0  # noqa: E731
        return {
            "eval": "composition",
            "model": self.model,
            "base_url": self.base_url,
            "overall": {
                "n": n,
                "composed": round(frac(lambda t: t.composed), 4),
                "used_graph": round(frac(lambda t: t.used_graph), 4),
                "correct": round(frac(lambda t: t.correct), 4),
                "correct_via_composition": round(frac(lambda t: t.correct_via_composition), 4),
            },
            "tasks": [t.as_dict() for t in self.tasks],
        }


def _run_one(client, cfg: AgentConfig, lib, task: dict) -> TaskResult:
    ep = run_episode(client, cfg, lib, task["prompt"], SYSTEM_PROMPT, COMPOSITION_TOOLS)
    expected = int(task["expected"])
    used_graph = any(n >= 2 for n in ep.trace.graphs_run)
    composed = used_graph or len(set(ep.trace.cells_run)) >= 2
    return TaskResult(
        task_id=str(task.get("id", task["prompt"])),
        prompt=task["prompt"],
        expected=expected,
        answer=ep.answer,
        composed=composed,
        used_graph=used_graph,
        cells_run=ep.trace.cells_run,
        correct=(ep.answer == expected),
        turns=ep.turns,
        error=ep.error,
    )


def run_composition(
    dataset: str = "composition_tasks",
    library_dir: str | None = None,
    model: str | None = None,
    config: AgentConfig | None = None,
) -> CompositionReport:
    """Run the composition eval (needs an OpenAI-compatible endpoint; client built lazily)."""
    cfg = config or AgentConfig.from_env(model)
    lib = open_library(library_dir)
    client = make_client(cfg)
    report = CompositionReport(model=cfg.model, base_url=cfg.base_url)
    for task in load_jsonl(dataset):
        report.tasks.append(_run_one(client, cfg, lib, task))
    return report
