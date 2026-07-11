"""Offline tests for the cell-potion surface (no LLM, no downloaded model).

Two halves:

* **The banked trainer** (`cell-eval/potion/train.py`, the script behind the
  earned-in artifact) gets a numerical gradient check it didn't have: `Trainer.step`
  computes hand-derived gradients and applies Adam in one call, but at t=1 the Adam
  state gives the raw gradient back exactly (m = 0.1·g and m̂ = m/(1−0.9¹) = g), so
  running one step with lr=0 exposes both the loss and the true gradient without
  touching the banked code. Finite differences then judge the derivation — on both
  the full-softmax path and the λ-weighted hard-negative path.
* **The regeneration CLI** (`cell_eval.potion`, added post-run) is tested against a
  fake chat client: prompt is manifest-only, replies parse, per-cell rows come out
  in the banked corpus shape, and the corpus validation catches what it must.
"""

from __future__ import annotations

import importlib.util
import pathlib
import sys

import numpy as np
import pytest

from cell_eval.potion import (
    _author_cell,
    _extract_json,
    _pairgen_prompt,
    validate_corpus,
)

# ── the banked trainer: gradient check via Adam-state recovery ──────────────────────

_TRAIN_PY = pathlib.Path(__file__).resolve().parents[1] / "potion" / "train.py"


def _load_train_module():
    spec = importlib.util.spec_from_file_location("potion_train", _TRAIN_PY)
    mod = importlib.util.module_from_spec(spec)
    sys.modules.setdefault("potion_train", mod)
    spec.loader.exec_module(mod)
    return mod


@pytest.fixture(scope="module")
def train_mod():
    return _load_train_module()


def _mini_trainer(train_mod, table, tau=0.1, lam=0.0, lr=0.0):
    """A Trainer over a tiny synthetic table, bypassing __init__ (which loads the
    real 63k-token base model) — step() only needs E/tau/lam/lr and Adam state."""
    tr = train_mod.Trainer.__new__(train_mod.Trainer)
    tr.E = np.asarray(table, dtype=np.float64).copy()
    tr.tau, tr.lam, tr.lr = tau, lam, lr
    tr.m = np.zeros_like(tr.E)
    tr.v = np.zeros_like(tr.E)
    tr.t = 0
    return tr


def _tiny_problem(seed=7):
    rng = np.random.default_rng(seed)
    table = rng.normal(size=(12, 5))
    batch = [{"_qtoks": [0, 3, 3]}, {"_qtoks": [2]}, {"_qtoks": [7, 8]}]  # repeat token
    doc_toks = [[1, 4], [5], [9, 10, 11], [6, 0]]
    tgt = [0, 2, 3]
    hard = [[1], [], [0, 2]]  # exercises the restricted-CE path when lam > 0
    return table, batch, doc_toks, tgt, hard


def _loss_and_grad(train_mod, table, tau, lam, batch, doc_toks, tgt, hard):
    """One lr=0 step: E is untouched, loss comes back, and g = m̂ = m/(1−0.9)."""
    tr = _mini_trainer(train_mod, table, tau=tau, lam=lam, lr=0.0)
    loss = tr.step(batch, doc_toks, tgt, hard)
    assert np.array_equal(tr.E, np.asarray(table)), "lr=0 must not move the table"
    return loss, tr.m / 0.1


@pytest.mark.parametrize("lam", [0.0, 0.5])
def test_banked_trainer_gradient_matches_finite_differences(train_mod, lam):
    table, batch, doc_toks, tgt, hard = _tiny_problem()
    loss, grad = _loss_and_grad(train_mod, table, 0.1, lam, batch, doc_toks, tgt, hard)
    assert loss > 0

    eps = 1e-6
    rng = np.random.default_rng(0)
    for _ in range(40):
        i, j = rng.integers(table.shape[0]), rng.integers(table.shape[1])
        t = table.copy()
        t[i, j] += eps
        lp, _ = _loss_and_grad(train_mod, t, 0.1, lam, batch, doc_toks, tgt, hard)
        t[i, j] -= 2 * eps
        lm, _ = _loss_and_grad(train_mod, t, 0.1, lam, batch, doc_toks, tgt, hard)
        num = (lp - lm) / (2 * eps)
        assert grad[i, j] == pytest.approx(num, abs=1e-5), f"coord ({i},{j}), lam={lam}"


def test_banked_trainer_untouched_rows_get_zero_grad(train_mod):
    table, batch, doc_toks, tgt, hard = _tiny_problem()
    used = {t for r in batch for t in r["_qtoks"]} | {t for d in doc_toks for t in d}
    _, grad = _loss_and_grad(train_mod, table, 0.1, 0.5, batch, doc_toks, tgt, hard)
    for row in set(range(table.shape[0])) - used:
        assert np.allclose(grad[row], 0.0)


def test_banked_trainer_descends(train_mod):
    table, batch, doc_toks, tgt, hard = _tiny_problem(seed=3)
    tr = _mini_trainer(train_mod, table, tau=0.1, lam=0.5, lr=0.05)
    l0 = tr.step(batch, doc_toks, tgt, hard)
    for _ in range(60):
        l1 = tr.step(batch, doc_toks, tgt, hard)
    assert l1 < l0


def test_dev_split_is_deterministic_and_roughly_a_quarter(train_mod):
    qs = [f"query number {i} about widgets" for i in range(400)]
    marks = [train_mod.dev_of(q) for q in qs]
    assert marks == [train_mod.dev_of(q) for q in qs]  # stable
    frac = sum(marks) / len(marks)
    assert 0.15 < frac < 0.35


def test_load_corpus_rejects_unknown_ids(train_mod, tmp_path):
    p = tmp_path / "pairs.jsonl"
    p.write_text('{"cell": "ghost", "kind": "paraphrase", "query": "x", "hard_negatives": []}\n')
    with pytest.raises(AssertionError):
        train_mod.load_corpus(p, {"abs_diff"})
    p.write_text(
        '# comment\n{"cell": "abs_diff", "kind": "adversarial", "query": "x", "hard_negatives": ["ghost"]}\n'
    )
    with pytest.raises(AssertionError):
        train_mod.load_corpus(p, {"abs_diff"})


# ── the regeneration CLI (fake client) ──────────────────────────────────────────────


class _FakeClient:
    """Echoes a canned body regardless of prompt."""

    def __init__(self, body: str):
        self._body = body
        outer = self

        class _Completions:
            @staticmethod
            def create(**kwargs):
                outer.last_kwargs = kwargs

                class _Msg:
                    content = outer._body

                class _Choice:
                    message = _Msg()

                class _Resp:
                    choices = [_Choice()]

                return _Resp()

        class _Chat:
            completions = _Completions()

        self.chat = _Chat()


_MANIFEST = {
    "id": "abs_diff",
    "summary": "Absolute difference |a - b| between two values.",
    "tags": ["math", "diff"],
    "signature": "run(a: u16, b: u16) -> u16",
}
_NEIGHBOURS = [
    {"id": "is_le", "summary": "a <= b.", "tags": ["compare"]},
    {"id": "sub_sat", "summary": "Saturating subtract.", "tags": ["math"]},
]


def test_pairgen_prompt_is_manifest_only_and_names_neighbours():
    p = _pairgen_prompt(_MANIFEST, _NEIGHBOURS, 8)
    assert "abs_diff" in p and "is_le" in p and "Saturating subtract." in p
    # the paraphrase instruction bans the cell's own vocabulary
    assert "AVOID" in p and "diff" in p and "math" in p
    # adversarial queries must name a skirted neighbour
    assert '"skirts"' in p


def test_extract_json_tolerates_fences_and_prose():
    body = 'Sure! Here you go:\n```json\n{"paraphrase": ["a"], "adversarial": []}\n```'
    assert _extract_json(body) == {"paraphrase": ["a"], "adversarial": []}
    assert _extract_json("no json here") is None
    assert _extract_json(None) is None
    assert _extract_json('{"broken": ') is None


class _Cfg:
    model = "fake-model"
    temperature = 0.8


def _good_body(n_para=2):
    return (
        '{"paraphrase": ' + str([f"wording {i}" for i in range(n_para)]).replace("'", '"') + ","
        ' "adversarial": ['
        '{"query": "gap, not whether one is at most the other", "skirts": "is_le"},'
        '{"query": "difference, not floored subtraction", "skirts": "sub_sat"}]}'
    )


def test_author_cell_produces_banked_corpus_shape():
    rows = _author_cell(_FakeClient(_good_body()), _Cfg(), _MANIFEST, _NEIGHBOURS, n_para=2)
    kinds = [r["kind"] for r in rows]
    assert kinds == ["paraphrase", "paraphrase", "adversarial", "adversarial", "direct"]
    for r in rows:
        assert r["cell"] == "abs_diff" and r["query"]
    para, adv, direct = rows[0], rows[2], rows[-1]
    assert para["hard_negatives"] == []
    assert adv["hard_negatives"] in (["is_le"], ["sub_sat"])
    assert direct["query"] == _MANIFEST["summary"] and direct["hard_negatives"] == []


def test_author_cell_rejects_short_or_misattributed_replies():
    # too few paraphrases
    assert _author_cell(_FakeClient(_good_body()), _Cfg(), _MANIFEST, _NEIGHBOURS, n_para=8) is None
    # adversarial skirting an id that isn't a neighbour
    bad = '{"paraphrase": ["a", "b"], "adversarial": [{"query": "q", "skirts": "ghost"}]}'
    assert _author_cell(_FakeClient(bad), _Cfg(), _MANIFEST, _NEIGHBOURS, n_para=2) is None
    # unparseable
    assert _author_cell(_FakeClient("nope"), _Cfg(), _MANIFEST, _NEIGHBOURS, n_para=2) is None


def test_validate_corpus_catches_the_banked_checks():
    ok = [
        {"cell": "abs_diff", "kind": "paraphrase", "query": "gap between two", "hard_negatives": []},
        {"cell": "is_le", "kind": "paraphrase", "query": "at most check", "hard_negatives": []},
    ]
    assert validate_corpus(ok, {"abs_diff", "is_le"}) == []
    bad = ok + [
        {"cell": "ghost", "kind": "direct", "query": "x", "hard_negatives": []},
        {"cell": "abs_diff", "kind": "adversarial", "query": "y", "hard_negatives": ["ghost"]},
        {"cell": "is_le", "kind": "paraphrase", "query": "Gap  Between two", "hard_negatives": []},
    ]
    problems = validate_corpus(bad, {"abs_diff", "is_le"})
    assert len(problems) == 3
    assert any("unknown cell" in p for p in problems)
    assert any("unknown hard negative" in p for p in problems)
    assert any("duplicate query" in p for p in problems)


# ── generate_pairs offline (the regeneration loop, no model, no network) ─────────


class _FakeEmbedder:
    """Deterministic char-bucket vectors — enough for a stable confusable map."""

    def __init__(self, model=None):
        self.name = "fake"

    def encode(self, texts):
        import numpy as np

        out = []
        for t in texts:
            v = np.zeros(16, dtype="float32")
            for b in t.lower().encode():
                v[b % 16] += 1.0
            n = float(np.linalg.norm(v)) or 1.0
            out.append(v / n)
        return np.array(out)


class _PairgenClient:
    """A prompt-aware fake: reads the neighbour ids out of the authoring prompt and
    replies with exactly the counts the validator demands — except for `fail_for`
    cells, which always get an unparseable reply (the retry-then-fail path)."""

    def __init__(self, n_para, fail_for=()):
        self.n_para = n_para
        self.fail_for = set(fail_for)
        outer = self

        class _C:
            def create(self, **kw):
                import json as _json
                import re

                prompt = kw["messages"][1]["content"]
                cell = re.search(r"id: (\S+)", prompt).group(1)
                if cell in outer.fail_for:
                    return _make_resp("no json here, sorry")
                neigh = re.findall(r"^  - (\S+):", prompt, re.M)
                reply = _json.dumps(
                    {
                        "paraphrase": [
                            f"reworded ask {i} for {cell}" for i in range(outer.n_para)
                        ],
                        "adversarial": [
                            {"query": f"{cell} not {n} please", "skirts": n} for n in neigh
                        ],
                    }
                )
                return _make_resp(reply)

        self.chat = type("Chat", (), {"completions": _C()})()


def _make_resp(content):
    msg = type("M", (), {"content": content})()
    return type("R", (), {"choices": [type("C", (), {"message": msg})()]})()


def test_generate_pairs_offline_end_to_end(tiny_setup, tmp_path, monkeypatch):
    from cell_eval import tiers
    from cell_eval.potion import generate_pairs, write_pairs

    monkeypatch.setattr(tiers, "Embedder", _FakeEmbedder)
    library, _ = tiny_setup
    rows, stats = generate_pairs(
        model="fake",
        library_dir=library,
        n_para=2,
        n_adv=2,
        client=_PairgenClient(n_para=2, fail_for={"fp_ident"}),
        only_cells=["pick_lo", "f_add", "fp_ident"],
    )
    # Two cells authored (2 para + 2 adv + 1 direct each); the failing one retried out.
    assert stats["cells"] == 3 and stats["failed_cells"] == ["fp_ident"]
    assert len(rows) == 10 and stats["validation_problems"] == []
    kinds = {r["cell"]: [x["kind"] for x in rows if x["cell"] == r["cell"]] for r in rows}
    assert kinds["pick_lo"].count("paraphrase") == 2
    assert kinds["f_add"].count("adversarial") == 2
    # Adversarial rows carry the skirted neighbour as the first hard negative.
    adv = next(r for r in rows if r["kind"] == "adversarial")
    assert len(r["hard_negatives"] if (r := adv) else []) == 1

    out = tmp_path / "pairs.jsonl"
    write_pairs(rows, stats, out)
    text = out.read_text()
    assert text.startswith("#") and "NEVER add rows from datasets/retrieval.jsonl" in text
    assert len([l for l in text.splitlines() if l and not l.startswith("#")]) == 10


def test_extract_json_and_validate_corpus_edges():
    from cell_eval.potion import _extract_json, validate_corpus

    assert _extract_json(None) is None
    assert _extract_json("no braces at all") is None
    assert _extract_json("{not json}") is None
    assert _extract_json('prose {"a": {"b": 1}} more prose') == {"a": {"b": 1}}

    rows = [
        {"cell": "ghost", "query": "q1", "hard_negatives": []},
        {"cell": "min", "query": "q2", "hard_negatives": ["nope"]},
        {"cell": "min", "query": "same words", "hard_negatives": []},
        {"cell": "max", "query": "same  WORDS", "hard_negatives": []},
    ]
    problems = validate_corpus(rows, {"min", "max"})
    assert any("unknown cell" in p for p in problems)
    assert any("unknown hard negative" in p for p in problems)
    assert any("duplicate query" in p for p in problems)
