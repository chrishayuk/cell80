#!/usr/bin/env python3
"""CN-1 real build, step 2c (`cell-native-architectures-cn1-preregistration.md`): the
three-way-tied fingerprint-projection model, plus the embedding resize that grows pretrained
TinyModel v11 (71261 rows) to the extended vocabulary (72052 rows) without disturbing a
single trained row.

The architectural core, and the one the slice-0 pilot's whole lesson points at. The pilot
proved that a fingerprint-placed embedding is inert unless the output projection shares
weights with the input embedding (two-way tying). Library-size invariance needs one more
turn of the screw: the cell-token rows must not be free parameters at all — they must be a
shared function `W_f(fingerprint)`, used as BOTH the input embedding row AND the output-head
row. Then a cell never seen called still has an address: its row is `W_f(its fingerprint)`,
computed from the same projection the seen cells trained. That is the only mechanism by which
gate (ii) can pass, and if the emission side were a free softmax over seen cells a held-out
cell would be unspeakable regardless of how good its input embedding was — so both sides are
`W_f`.

Two arms share every non-cell weight and all tying; they differ only in where cell rows come
from:
  - arm (c) `fingerprint`: cell rows = `W_f(FP)`, `W_f` learned on seen cells, `FP` frozen.
  - arm (b) `random`:      cell rows = free learned params (ordinary embedding rows).
Held-out (axis-A) cells are in the vocabulary and the decode grammar either way; in arm (c)
their row is derivable (`W_f(FP_heldout)`), in arm (b) it was never trained — the ablation.

This module is model-definition + a structural self-test only; it does no training. Run it to
validate the apparatus before the corpus exists (the pilot's discipline: find the bugs in the
harness before trusting any number).

Run: python3 cn1_model.py    # loads the real v11 checkpoint on CPU, ~seconds, no training
"""
from __future__ import annotations

import json
import math
import sys
from pathlib import Path

import torch
import torch.nn as nn
import torch.nn.functional as F

HERE = Path(__file__).resolve().parent
TINY_MODEL = HERE.parent.parent.parent / "tiny-model"
LIBRARY = HERE / "cn1_library.jsonl"
TOKEN_MAP = HERE / "cn1_cell_token_map.json"
AXIS_A = HERE / "cn1_axis_a_heldout.json"

sys.path.insert(0, str(TINY_MODEL / "model" / "v11-core" / "src"))

FP_DIM = 20  # DEFAULT_PROBES length (see cell80 fingerprint.rs); each probe -> one Option<u16>
BASE_VOCAB_ROWS = 71261  # pretrained v11 embedding rows (config.vocab_size)
EXTENDED_VOCAB = 72052  # after appending 792 cell/delimiter tokens (ids 0..72051)


# ---- fingerprint feature encoding -------------------------------------------------

def encode_fingerprint(raw: list) -> list[float]:
    """20 Option<u16> probe outputs -> a 40-d feature vector: 20 scaled values (None->0) and a
    20-d "ran cleanly" mask. None is a real distinguishing signal (a trap/halt outcome, per
    Fingerprint's own doc), so the mask is a feature, not padding — dropping it would collapse
    'returned 0' and 'did not return' onto the same point.
    """
    vals = [0.0 if v is None else (float(v) / 65535.0) for v in raw]
    mask = [0.0 if v is None else 1.0 for v in raw]
    return vals + mask


# ---- the three-way-tied model ------------------------------------------------------

class Wf(nn.Module):
    """fingerprint (40-d features) -> d_model row. A small MLP; the projection whose learned
    structure a held-out cell inherits. Default shape is a starting point, not a tuned choice."""

    def __init__(self, in_dim: int, d_model: int, hidden: int = 256):
        super().__init__()
        self.net = nn.Sequential(
            nn.Linear(in_dim, hidden),
            nn.GELU(),
            nn.Linear(hidden, d_model),
        )

    def forward(self, fp_feats: torch.Tensor) -> torch.Tensor:
        return self.net(fp_feats)


class CN1Model(nn.Module):
    """Wraps a resized TinyModel v11. Overrides the forward so that the effective embedding
    matrix used for BOTH the input lookup and the tied output head has cell-token rows supplied
    by `W_f(FP)` (arm c) or by the base free params (arm b). Every non-cell row and every
    transformer block is the pretrained base, unchanged.
    """

    def __init__(self, base, cell_first_id: int, fp_feats: torch.Tensor, arm: str):
        super().__init__()
        assert arm in ("fingerprint", "shuffled", "random", "description")
        self.base = base
        self.arm = arm
        self.dim = base.dim
        self.cell_first_id = cell_first_id  # first cell token id (delimiters excluded)
        n_cells = fp_feats.shape[0]
        self.cell_ids = torch.arange(cell_first_id, cell_first_id + n_cells)
        # FP is frozen data, not a parameter — a held-out cell's features must be identical
        # whether or not it was ever called.
        self.register_buffer("fp_feats", fp_feats)
        # arm "shuffled" = the control: same W_f, same geometry, same SET of fingerprint vectors,
        # but each cell is assigned a DIFFERENT cell's fingerprint (permuted at build time). If
        # held-out ranking survives the shuffle, the signal is the projection layer, not the
        # behaviour; if it collapses toward random, behaviour is doing independent work.
        if arm in ("fingerprint", "shuffled", "description"):
            self.w_f = Wf(fp_feats.shape[1], self.dim)

    def effective_embed_weight(self) -> torch.Tensor:
        """The (V, dim) matrix used for input lookup and output projection — three-way tied.

        Cell tokens are a contiguous tail range, so the effective matrix is assembled by
        `torch.cat` of [pretrained+delimiter rows | W_f(FP) cell rows | any rows above]. cat of
        contiguous slices is autograd-clean (gradient flows to w_f via cell_rows, to the base
        params elsewhere; the base's own cell rows get no gradient, as intended) and — unlike
        index_copy — is implemented on MPS.
        """
        w = self.base.embed.weight
        if self.arm == "random":
            return w
        cell_rows = self.w_f(self.fp_feats)  # (n_cells, dim)
        lo = self.cell_first_id
        hi = lo + cell_rows.shape[0]
        parts = [w[:lo], cell_rows]
        if hi < w.shape[0]:
            parts.append(w[hi:])
        return torch.cat(parts, dim=0)

    def forward(self, input_ids: torch.Tensor) -> torch.Tensor:
        w = self.effective_embed_weight()
        x = F.embedding(input_ids, w) * math.sqrt(self.dim)
        for layer in self.base.layers:
            x = layer(x, self.base.rope_freqs)
        x = self.base.norm(x)
        return x @ w.t()  # tied output head over the SAME effective matrix


# ---- embedding resize --------------------------------------------------------------

def resize_embedding(base, new_vocab: int) -> None:
    """Grow the tied embed/lm_head to `new_vocab` rows in place, preserving every existing row
    (ids 0..old-1) and re-tying. New rows get the model's own xavier init as a placeholder;
    the training script overwrites cell rows (arm b: trained free; arm c: via W_f)."""
    old = base.embed.weight
    old_rows, dim = old.shape
    assert new_vocab >= old_rows, "resize only grows"
    new_embed = nn.Embedding(new_vocab, dim)
    nn.init.xavier_uniform_(new_embed.weight)
    with torch.no_grad():
        new_embed.weight[:old_rows] = old.data
    base.embed = new_embed
    base.lm_head = nn.Linear(dim, new_vocab, bias=False)
    base.lm_head.weight = base.embed.weight  # re-tie
    base.vocab_size = new_vocab


# ---- load helpers ------------------------------------------------------------------

DESC_FEATURES = HERE / "cn1_desc_features.json"


def load_fingerprint_features(kind="fingerprint"):
    """Returns (feats, cell_first_id, cell_names in id order, held_set).
    kind="fingerprint": behavioural fingerprint (40-d) — the behaviour address (arm c/s).
    kind="description": bge-small sentence-encoding of the descriptor (384-d) — the *language*
      address (arm d, the mandatory CoTools-style baseline)."""
    lib = {json.loads(l)["name"]: json.loads(l) for l in LIBRARY.read_text().splitlines() if l.strip()}
    tok_map = json.loads(TOKEN_MAP.read_text())
    held = {h["name"] for h in json.loads(AXIS_A.read_text())["held_out_cells"]}
    cell_entries = sorted(
        ((k[len("<cell:"):-1], v) for k, v in tok_map.items() if k.startswith("<cell:")),
        key=lambda kv: kv[1],
    )
    cell_first_id = cell_entries[0][1]
    ids = [i for _, i in cell_entries]
    assert ids == list(range(cell_first_id, cell_first_id + len(ids))), "cell ids must be contiguous"
    names = [n for n, _ in cell_entries]
    if kind == "description":
        desc = json.loads(DESC_FEATURES.read_text())
        feats = torch.tensor([desc[n] for n in names], dtype=torch.float32)
    else:
        feats = torch.tensor([encode_fingerprint(lib[n]["fingerprint"]) for n in names], dtype=torch.float32)
    return feats, cell_first_id, names, held


SHUFFLE_SEED = 1234  # fixed derangement for the "shuffled" control arm


def _derangement(n, seed):
    """A seeded permutation with NO fixed points, so no cell keeps its own fingerprint."""
    import numpy as np

    rng = np.random.default_rng(seed)
    perm = rng.permutation(n)
    for i in range(n):
        if perm[i] == i:
            j = (i + 1) % n
            perm[i], perm[j] = perm[j], perm[i]
    assert not any(perm[i] == i for i in range(n)), "derangement has a fixed point"
    return perm


def build(arm: str):
    from tiny_model_v11.loader import load_from_artifacts

    base, cfg = load_from_artifacts(str(TINY_MODEL / "model" / "v11"), device="cpu")
    resize_embedding(base, EXTENDED_VOCAB)
    kind = "description" if arm == "description" else "fingerprint"
    feats, cell_first_id, names, held = load_fingerprint_features(kind=kind)
    if arm == "shuffled":
        perm = _derangement(feats.shape[0], SHUFFLE_SEED)
        feats = feats[perm.tolist()]  # each cell now carries a DIFFERENT cell's fingerprint
    model = CN1Model(base, cell_first_id, feats, arm)
    return model, names, held


# ---- structural self-test ----------------------------------------------------------

def _selftest():
    print("== loading v11, resizing 71261 -> 72052 ==", flush=True)
    from tiny_model_v11.loader import load_from_artifacts

    base, cfg = load_from_artifacts(str(TINY_MODEL / "model" / "v11"), device="cpu")
    trained_snapshot = base.embed.weight[:BASE_VOCAB_ROWS].clone()
    resize_embedding(base, EXTENDED_VOCAB)

    # 1. resize preserved every trained row, and tying holds after resize
    assert base.embed.weight.shape[0] == EXTENDED_VOCAB
    assert torch.equal(base.embed.weight[:BASE_VOCAB_ROWS], trained_snapshot), "trained rows changed!"
    assert base.embed.weight.data_ptr() == base.lm_head.weight.data_ptr(), "tying broken after resize"
    print(f"  OK: {EXTENDED_VOCAB} rows, {BASE_VOCAB_ROWS} trained rows byte-preserved, lm_head re-tied")

    feats, cell_first_id, names, held = load_fingerprint_features()
    n_cells = len(names)
    held_names = [n for n in names if n in held]
    seen_names = [n for n in names if n not in held]
    print(f"  cells: {n_cells} ({len(seen_names)} seen, {len(held_names)} held-out), first id {cell_first_id}")

    # 2. arm (c): cell row == W_f(FP), input row == output row (three-way tying), for both a
    #    seen and a held-out cell; and gradient reaches W_f.
    model_c = CN1Model(base, cell_first_id, feats, "fingerprint")
    w_eff = model_c.effective_embed_weight()
    direct = model_c.w_f(model_c.fp_feats)
    held_idx = names.index(held_names[0])
    seen_idx = names.index(seen_names[0])
    for label, idx in [("seen", seen_idx), ("held-out", held_names and held_idx)]:
        row_id = cell_first_id + idx
        assert torch.allclose(w_eff[row_id], direct[idx], atol=1e-6), f"{label} row != W_f(FP)"
    print("  OK: arm(c) cell rows == W_f(FP) for seen and held-out; input matrix IS output matrix (tied)")

    # 3. THE load-bearing property: a gradient step on shared W_f moves a HELD-OUT cell's row.
    #    This is the mechanism gate (ii) rides on — held-out cells have no free params, so if
    #    they move at all, it is only through the shared projection the seen cells trained.
    held_row_before = model_c.effective_embed_weight()[cell_first_id + held_idx].detach().clone()
    opt = torch.optim.SGD(model_c.w_f.parameters(), lr=1e-2)
    # A toy objective on a SEEN cell's row only; the held-out cell is not in the loss at all.
    loss = model_c.effective_embed_weight()[cell_first_id + seen_idx].pow(2).sum()
    opt.zero_grad(); loss.backward()
    gnorm = sum(p.grad.norm().item() for p in model_c.w_f.parameters() if p.grad is not None)
    opt.step()
    held_row_after = model_c.effective_embed_weight()[cell_first_id + held_idx].detach().clone()
    moved = (held_row_after - held_row_before).norm().item()
    assert gnorm > 0, "no gradient reached W_f"
    assert moved > 0, "held-out row did not move when shared W_f was updated — gate (ii) mechanism absent"
    print(f"  OK: W_f grad-norm {gnorm:.4f}; a step optimizing only a SEEN row moved the HELD-OUT row by {moved:.4e}")
    print("      (held-out cells share the seen cells' projection — the only way an unseen cell gets an address)")

    # 4. arm (b): cell rows are the base free params, independent of any W_f.
    model_b = CN1Model(base, cell_first_id, feats, "random")
    assert not hasattr(model_b, "w_f"), "random arm must have no W_f"
    w_eff_b = model_b.effective_embed_weight()
    assert w_eff_b.data_ptr() == base.embed.weight.data_ptr(), "arm(b) should use base embed directly"
    print("  OK: arm(b) cell rows are free base params (no W_f) — the ablation")

    # 5. a forward pass runs and produces logits over the extended vocab
    ids = torch.tensor([[2, 388, 21221, cell_first_id + seen_idx, 3]])  # bos ... <cell> eos-ish
    with torch.no_grad():
        logits = model_c(ids)
    assert logits.shape == (1, 5, EXTENDED_VOCAB), f"bad logits shape {tuple(logits.shape)}"
    print(f"  OK: forward pass -> logits {tuple(logits.shape)} over the extended vocab")

    print("\nstructural self-test: PASS — three-way tying, resize, and the gate-(ii) mechanism are wired correctly")


if __name__ == "__main__":
    _selftest()
