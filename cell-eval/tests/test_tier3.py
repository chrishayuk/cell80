"""Tier 3 offline: the probe machinery against the real library (deterministic, no
LLM) — same-shape siblings must separate on executed evidence; rung-1 example
matching filters candidate sets; and the A/B loop runs against a scripted model."""

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from _agentfake import FakeClient, _Msg  # noqa: E402

from cell_eval.agent import AgentConfig  # noqa: E402
from cell_eval.library import open_library  # noqa: E402
from cell_eval.tier3 import match_examples, probe_table  # noqa: E402


def test_probes_separate_same_shape_siblings():
    lib = open_library()
    # min/max: identical signature, identical-shape manifests — the measured text
    # ceiling. One executed probe row must tell them apart.
    t = probe_table(lib, ["min", "max"])
    assert t.probes, "min/max must yield discriminating probes"
    assert t.outputs["min"] != t.outputs["max"]
    # gcd/lcm — the number-theory confusables.
    t = probe_table(lib, ["gcd", "lcm"])
    assert t.probes and t.outputs["gcd"] != t.outputs["lcm"]


def test_probe_table_skips_state_cells_and_renders():
    lib = open_library()
    t = probe_table(lib, ["min", "max", "manhattan"])  # manhattan is a state cell
    assert "manhattan" in t.skipped
    r = t.render()
    assert "min" in r and "max" in r and "manhattan" in r


def test_match_examples_is_rung_one_scoped():
    lib = open_library()
    got = match_examples(lib, ["min", "max", "gcd"], [([3, 7], 3), ([10, 2], 2)])
    assert got == ["min"]  # only min reproduces both
    assert match_examples(lib, ["min", "max"], [([3, 7], 99)]) == []


def test_disambiguation_ab_loop_offline():
    from cell_eval.tier3 import run_disambiguation

    # A scripted "model": always picks the first candidate manifests-only, and the
    # right one when evidence is attached — the report must show the lift.
    lib = open_library()
    # Build enough scripted turns: 2 per escalated case; cap the dataset small by
    # reusing the retrieval dataset but a tiny fake client script that repeats.
    class Repeat:
        def __init__(self):
            self.chat = self
            self.completions = self

        def create(self, **kw):
            content = kw["messages"][-1]["content"]
            # With probe evidence: answer the expected id smuggled by the test via
            # candidates order — pick the last candidate line's id; without: the first.
            lines = [l for l in content.splitlines() if l.startswith("- ")]
            has_probes = "outputs on sample inputs" in content
            cid = lines[-1 if has_probes else 0].split(":")[0][2:].strip()
            return _Resp2(cid)

    class _Resp2:
        def __init__(self, cid):
            msg = _Msg(content=f"CELL: {cid}")
            self.choices = [type("C", (), {"message": msg})()]

    rep = run_disambiguation(
        model="scripted",
        client=Repeat(),
        cfg=AgentConfig(model="scripted", base_url="offline"),
        embed_model="minishlab/potion-retrieval-32M",
    )
    d = rep.as_dict()
    assert d["splits"], "the escalated residue must be non-empty"
    for c, s in d["splits"].items():
        assert 0.0 <= s["manifests_only"] <= 1.0
        assert 0.0 <= s["with_probes"] <= 1.0
