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


def render_authored(report) -> str:
    """The agent-authored-example lane: authoring quality, the behavioural
    equivalence classes the probes pin down, and retrieval three ways (plain /
    oracle-equipped / authored) over the same cases."""
    d = report.as_dict()
    lines = [
        f"authored-examples eval — model={d['model']}  library={d['library']}  k={d['k']}",
        "",
        f"population: {d['population']} authorable (value cells, arity 1-3) of "
        f"{d['total_cases']} cases ({d['population_fraction']:.1%}); "
        f"{d['overall']['n']} asked",
        "",
    ]

    def block(label: str, s: dict) -> list[str]:
        return [
            f"  {label}: n={s['n']}  well_formed={s['well_formed']:.2f}  "
            f"valid={s['valid']:.2f}  false_unique={s['false_unique_rate']:.3f}  "
            f"ambiguous={s['ambiguity_rate']:.2f}",
            f"    P@1  plain={s['plain']['precision@1']:.2f}  "
            f"oracle={s['oracle']['precision@1']:.2f}  "
            f"authored={s['authored']['precision@1']:.2f}  "
            f"(correlated failure {s['correlated_failure']:.3f})",
        ]

    lines += block("OVERALL", d["overall"])
    for cat, s in d["by_category"].items():
        lines += [""] + block(cat, s)
    return "\n".join(lines)


def render_retrieval_examples(report) -> str:
    """The plain-vs-fused split (WS-F/F2): per category, text-only P@1 next to the
    example-equipped fused P@1 over the same equipped subset."""
    lines = [
        f"retrieval eval (example-equipped) — library={report.library}  k={report.k}  "
        f"sidecar={report.examples_dataset}",
        "",
        f"coverage: {report.coverage():.1%} of {len(report.cases)} cases equipped",
        "",
        _agg_line("OVERALL plain", report.plain()),
        _agg_line("OVERALL deployed", report.deployed()),
        "",
        "by category (equipped subset, plain vs fused):",
    ]
    for cat in report.categories():
        lines.append(
            f"  {cat}: coverage {report.coverage(cat):.1%}"
        )
        lines.append(_agg_line("  plain", report.plain(cat, True)))
        lines.append(_agg_line("  fused", report.fused(cat)))
    regs = report.regressions()
    if regs:
        lines += [
            "",
            f"REGRESSIONS (fused worse than plain — should be impossible): {len(regs)}",
        ]
        for c in regs[:10]:
            lines.append(
                f"  [{c.category}] {c.query!r} plain={c.plain_rank} fused={c.fused_rank}"
            )
    return "\n".join(lines)


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
        f"used_pipeline={o['used_pipeline']:.2f}  "
        f"correct={o['correct']:.2f}  correct_via_composition={o['correct_via_composition']:.2f}",
        "",
        "per task:",
    ]
    for t in report.tasks:
        mark = "✓" if t.correct else "✗"
        how = (
            "pipeline"
            if t.used_pipeline
            else "graph"
            if t.used_graph
            else (f"chain:{len(set(t.cells_run))}" if t.composed else "no-compose")
        )
        cells = ("+" + ",".join(t.cells_run)) if t.cells_run else ""
        lines.append(
            f"  {mark} [{how}{cells}] {t.task_id}: got={t.answer} want={t.expected}"
        )
    return "\n".join(lines)


def render_repair(report) -> str:
    d = report.as_dict()
    o = d["overall"]
    lines = [
        f"repair eval — model={report.model}  (one shot: broken source + compiler error)",
        "",
        f"  n={o['n']}  compiled={o['compiled']}  repair@1={o['repair_at_1']:.2f}",
        "",
        "per diagnostic class:",
    ]
    for klass, s_ in d["by_class"].items():
        lines.append(
            f"  {klass:<18} n={s_['n']}  compiled={s_['compiled']}  "
            f"repair@1={s_['repair_at_1']:.2f}"
        )
    misses = [r for r in d["results"] if not r["correct"]]
    if misses:
        lines.append("")
        lines.append("misses:")
        for r in misses:
            lines.append(f"  ✗ {r['id']} [{r['class']}] — {r['note']}")
    return "\n".join(lines)


def render_tiers(report, calibration=None) -> str:
    d = report.as_dict()
    lines = [
        f"tiered retrieval — embed={d['embed_model']}  gate θ={d['theta']} "
        f"(margin on the blended score; below → escalate)",
        "",
        f"  {'split':<12}{'n':>4}{'tier1 P@1':>11}{'tier2 P@1':>11}"
        f"{'answered':>10}{'prec@answered':>15}",
    ]
    for c, s_ in d["splits"].items():
        lines.append(
            f"  {c:<12}{s_['n']:>4}{s_['tier1_p1']:>11.2f}{s_['tier2_p1']:>11.2f}"
            f"{s_['answer_rate']:>10.2f}{s_['precision_on_answered']:>15.2f}"
        )
    if calibration:
        lines += [
            "",
            f"calibration: chosen θ={calibration['chosen_theta']} "
            f"(smallest margin with adversarial precision-on-answered ≥ "
            f"{calibration['floor']}); full curve in the JSON output",
        ]
    return "\n".join(lines)


def render_tier3(report) -> str:
    d = report.as_dict()
    lines = [
        f"tier-3 disambiguation — model={d['model']}  embed={d['embed_model']} "
        f"(the escalated residue at θ={d['theta']}; pick accuracy A/B)",
        "",
        f"  {'split':<12}{'n':>4}{'manifests-only':>16}{'with probes':>13}{'lift':>8}",
    ]
    for c, s_ in d["splits"].items():
        lift = s_["with_probes"] - s_["manifests_only"]
        lines.append(
            f"  {c:<12}{s_['n']:>4}{s_['manifests_only']:>16.2f}"
            f"{s_['with_probes']:>13.2f}{lift:>+8.2f}"
        )
    return "\n".join(lines)
