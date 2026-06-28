"""Human-readable rendering of eval reports. JSON is just `report.as_dict()`."""

from __future__ import annotations

from .metrics import Aggregate


def _agg_line(label: str, a: Aggregate) -> str:
    d = a.as_dict()
    return (
        f"  {label:<14} n={d['n']:<3}  "
        f"P@1={d['precision@1']:.2f}  hit@3={d['hit@3']:.2f}  "
        f"hit@5={d['hit@5']:.2f}  MRR={d['mrr']:.2f}"
    )


def render_retrieval(report) -> str:
    lines = [
        f"retrieval eval — library={report.library}  k={report.k}",
        "",
        _agg_line("OVERALL", report.overall),
        "",
        "by category:",
    ]
    for cat, agg in report.by_category.items():
        lines.append(_agg_line(cat, agg))

    misses = report.misses()
    if misses:
        lines += ["", f"misses (top-1 wrong) — {len(misses)}/{len(report.cases)}:"]
        for c in misses:
            where = "not in top-k" if c.rank is None else f"rank {c.rank}"
            got = ", ".join(c.returned[:3]) or "—"
            lines.append(
                f"  [{c.category}] {c.query!r}"
                f"\n      want {c.expected}  got [{got}]  ({where})"
            )
    else:
        lines += ["", "no misses — every query put an acceptable cell at rank 1."]
    return "\n".join(lines)


def render_adoption(report) -> str:
    o = report.as_dict()["overall"]
    lines = [
        f"adoption eval — model={report.model}  endpoint={report.base_url}",
        "",
        f"  n={o['n']}  adoption={o['adoption']:.2f}  "
        f"correct={o['correct']:.2f}  correct_via_cell={o['correct_via_cell']:.2f}",
        "",
        "per task:",
    ]
    for t in report.tasks:
        mark = "✓" if t.correct else "✗"
        used = "cell" if t.used_cell else "no-cell"
        cells = ("+" + ",".join(t.cells_run)) if t.cells_run else ""
        lines.append(
            f"  {mark} [{used}{cells}] {t.task_id}: got={t.answer} want={t.expected}"
        )
    return "\n".join(lines)


def render_composition(report) -> str:
    o = report.as_dict()["overall"]
    lines = [
        f"composition eval — model={report.model}  endpoint={report.base_url}",
        "",
        f"  n={o['n']}  composed={o['composed']:.2f}  used_graph={o['used_graph']:.2f}  "
        f"correct={o['correct']:.2f}  correct_via_composition={o['correct_via_composition']:.2f}",
        "",
        "per task:",
    ]
    for t in report.tasks:
        mark = "✓" if t.correct else "✗"
        how = (
            "graph"
            if t.used_graph
            else (f"chain:{len(set(t.cells_run))}" if t.composed else "no-compose")
        )
        cells = ("+" + ",".join(t.cells_run)) if t.cells_run else ""
        lines.append(
            f"  {mark} [{how}{cells}] {t.task_id}: got={t.answer} want={t.expected}"
        )
    return "\n".join(lines)
