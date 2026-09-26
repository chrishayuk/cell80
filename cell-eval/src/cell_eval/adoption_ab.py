"""Held-out live-agent A/B for the lexical and hybrid retrieval policies."""

from __future__ import annotations

import hashlib
import pathlib
import re
import time
from collections.abc import Callable
from dataclasses import dataclass, field

from cell80_mcp.library import CellLibrary
from cell80_mcp.retrieval import DEFAULT_MODEL

from .adoption import SYSTEM_PROMPT
from .agent import AgentConfig, make_client, run_episode
from .datasets import load_jsonl
from .library import seed_library_dir
from .tools import TOOLS

AB_SYSTEM_PROMPT = SYSTEM_PROMPT + (
    " cell_search separates ranked `results` from a confidence-gated `selected`. "
    "You may automatically run only `selected`. When it is null, inspect the few top "
    "candidates and run one only if its summary and signature resolve the ambiguity; "
    "otherwise solve without a cell. Never treat results[0] as approved by itself."
)


def _normalise(text: str) -> str:
    return " ".join(re.findall(r"[a-z0-9]+", text.lower()))


def leakage_collisions(dataset: str = "adoption_ab_heldout") -> list[dict]:
    """Exact normalized overlaps with either calibration retrieval or prior adoption."""
    references = []
    for row in load_jsonl("retrieval"):
        references.append(("retrieval", str(row.get("id", "")), row["query"]))
    for row in load_jsonl("tasks"):
        references.append(("tasks", str(row.get("id", "")), row["prompt"]))
    by_text: dict[str, list[tuple[str, str]]] = {}
    for source, row_id, text in references:
        by_text.setdefault(_normalise(text), []).append((source, row_id))
    collisions = []
    for row in load_jsonl(dataset):
        for source, reference_id in by_text.get(_normalise(row["prompt"]), []):
            collisions.append(
                {
                    "heldout_id": row["id"],
                    "source": source,
                    "reference_id": reference_id,
                }
            )
    return collisions


@dataclass
class ABTaskResult:
    arm: str
    task_id: str
    expected: int
    expected_cell: str
    answer: int | None
    correct: bool
    cells_run: list[str]
    inspected: list[str]
    search_decisions: list[dict]
    turns: int
    episode_ms: float
    error: str | None = None

    @property
    def adopted(self) -> bool:
        return bool(self.cells_run)

    @property
    def expected_cell_run(self) -> bool:
        return self.expected_cell in self.cells_run

    @property
    def wrong_runs(self) -> list[str]:
        return [cell for cell in self.cells_run if cell != self.expected_cell]

    @property
    def abstained(self) -> bool:
        return any(decision["abstained"] for decision in self.search_decisions)

    @property
    def recovered_from_abstention(self) -> bool:
        return self.abstained and self.correct and self.expected_cell_run

    @property
    def selected_ids(self) -> list[str]:
        return [
            decision.get("selected")
            for decision in self.search_decisions
            if decision.get("selected") is not None
        ]

    def as_dict(self) -> dict:
        return {
            "arm": self.arm,
            "task_id": self.task_id,
            "expected": self.expected,
            "expected_cell": self.expected_cell,
            "answer": self.answer,
            "correct": self.correct,
            "adopted": self.adopted,
            "expected_cell_run": self.expected_cell_run,
            "cells_run": self.cells_run,
            "inspected": self.inspected,
            "wrong_runs": self.wrong_runs,
            "selected_ids": self.selected_ids,
            "search_decisions": self.search_decisions,
            "turns": self.turns,
            "episode_ms": round(self.episode_ms, 3),
            "error": self.error,
        }


def _summary(results: list[ABTaskResult]) -> dict:
    n = len(results)

    def fraction(predicate):
        return sum(predicate(row) for row in results) / n if n else 0.0

    abstained = [row for row in results if row.abstained]
    search_latencies = [
        decision["latency_ms"]
        for row in results
        for decision in row.search_decisions
    ]
    selected = [
        (row, selected_id) for row in results for selected_id in row.selected_ids
    ]
    return {
        "n": n,
        "correct": round(fraction(lambda row: row.correct), 4),
        "adoption": round(fraction(lambda row: row.adopted), 4),
        "expected_cell_adoption": round(
            fraction(lambda row: row.expected_cell_run), 4
        ),
        "wrong_run_tasks": sum(bool(row.wrong_runs) for row in results),
        "wrong_runs": sum(len(row.wrong_runs) for row in results),
        "selected_decisions": len(selected),
        "correct_selected_decisions": sum(
            selected_id == row.expected_cell for row, selected_id in selected
        ),
        "selection_precision": round(
            sum(selected_id == row.expected_cell for row, selected_id in selected)
            / len(selected),
            4,
        )
        if selected
        else None,
        "abstained_tasks": len(abstained),
        "abstention_recovered": sum(row.recovered_from_abstention for row in abstained),
        "abstention_recovery_rate": round(
            sum(row.recovered_from_abstention for row in abstained) / len(abstained), 4
        )
        if abstained
        else None,
        "abstention_wrong_run_tasks": sum(
            row.abstained and bool(row.wrong_runs) for row in results
        ),
        "mean_search_ms": round(sum(search_latencies) / len(search_latencies), 3)
        if search_latencies
        else None,
        "mean_episode_ms": round(sum(row.episode_ms for row in results) / n, 3)
        if n
        else None,
        "errors": sum(row.error is not None for row in results),
    }


@dataclass
class AdoptionABReport:
    model: str
    base_url: str
    dataset: str
    dataset_sha256: str
    retrieval_methods: dict[str, str] = field(default_factory=dict)
    results: list[ABTaskResult] = field(default_factory=list)

    def as_dict(self) -> dict:
        arms = {
            arm: _summary([result for result in self.results if result.arm == arm])
            for arm in ("lexical", "hybrid")
        }
        keys = ("correct", "adoption", "expected_cell_adoption")
        return {
            "eval": "adoption_ab",
            "model": self.model,
            "base_url": self.base_url,
            "dataset": self.dataset,
            "dataset_sha256": self.dataset_sha256,
            "leakage_collisions": leakage_collisions(self.dataset),
            "retrieval_methods": self.retrieval_methods,
            "arms": arms,
            "hybrid_minus_lexical": {
                key: round(arms["hybrid"][key] - arms["lexical"][key], 4)
                for key in keys
            },
            "tasks": [result.as_dict() for result in self.results],
        }


def _run_one(client, cfg, library, arm: str, task: dict) -> ABTaskResult:
    started = time.perf_counter()
    episode = run_episode(
        client, cfg, library, task["prompt"], AB_SYSTEM_PROMPT, TOOLS
    )
    elapsed = (time.perf_counter() - started) * 1000.0
    expected = int(task["expected"])
    return ABTaskResult(
        arm=arm,
        task_id=str(task["id"]),
        expected=expected,
        expected_cell=str(task["cell"]),
        answer=episode.answer,
        correct=episode.answer == expected,
        cells_run=episode.trace.cells_run,
        inspected=episode.trace.inspected,
        search_decisions=episode.trace.search_decisions,
        turns=episode.turns,
        episode_ms=elapsed,
        error=episode.error,
    )


def run_adoption_ab(
    dataset: str = "adoption_ab_heldout",
    library_dir: str | None = None,
    model: str | None = None,
    config: AgentConfig | None = None,
    client=None,
    progress: Callable[[int, int, ABTaskResult], None] | None = None,
) -> AdoptionABReport:
    """Run interleaved lexical/hybrid episodes against one fixed model and prompt."""
    collisions = leakage_collisions(dataset)
    if collisions:
        raise ValueError(f"held-out dataset has {len(collisions)} exact leakage collision(s)")
    cfg = config or AgentConfig.from_env(model)
    directory = pathlib.Path(library_dir) if library_dir else seed_library_dir()
    libraries = {
        "lexical": CellLibrary(str(directory), retrieval_model="off"),
        "hybrid": CellLibrary(str(directory), retrieval_model=DEFAULT_MODEL),
    }
    expected_methods = {"lexical": "lexical_fallback", "hybrid": "hybrid_margin"}
    retrieval_methods = {}
    for arm, library in libraries.items():
        method = library.search_decision("greatest common divisor", 3)["confidence"][
            "method"
        ]
        retrieval_methods[arm] = method
        if method != expected_methods[arm]:
            raise RuntimeError(
                f"{arm} arm preflight returned {method!r}, expected {expected_methods[arm]!r}"
            )
    active_client = client or make_client(cfg)
    dataset_path = pathlib.Path(dataset)
    if not dataset_path.exists():
        dataset_path = pathlib.Path(__file__).resolve().parents[2] / "datasets" / (
            dataset if dataset.endswith(".jsonl") else f"{dataset}.jsonl"
        )
    report = AdoptionABReport(
        model=cfg.model,
        base_url=cfg.base_url,
        dataset=str(dataset),
        dataset_sha256=hashlib.sha256(dataset_path.read_bytes()).hexdigest(),
        retrieval_methods=retrieval_methods,
    )
    tasks = load_jsonl(dataset)
    total = len(tasks) * 2
    completed = 0
    for index, task in enumerate(tasks):
        order = ("lexical", "hybrid") if index % 2 == 0 else ("hybrid", "lexical")
        for arm in order:
            result = _run_one(active_client, cfg, libraries[arm], arm, task)
            report.results.append(result)
            completed += 1
            if progress:
                progress(completed, total, result)
    return report
