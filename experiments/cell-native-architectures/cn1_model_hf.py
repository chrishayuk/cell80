#!/usr/bin/env python3
"""CN-1 base-swap harness (pre-registration amendment 2026-07-13): the same three-way-tied
fingerprint model on a **code/math-pretrained** base — SmolLM2-135M — instead of the TinyStories
v11 base. This is the clean discriminator for the capacity hypothesis: v11's seen-cell top-1 ≈ 6%
is confounded (no arithmetic/symbolic prior vs. no capacity), and scaling v11 can't separate them.
A size-matched (~135M vs 115M) base that has actually seen structured symbolic text does.

Rider 1 (checked): SmolLM2-135M has `tie_word_embeddings=True` (verified same-storage), so the
fingerprint row is not inert — the precondition holds, as it must for any swap base.

Mirrors `cn1_model.CN1Model` exactly, but wraps a HF Llama: the effective embedding matrix (cell
rows = W_f(fingerprint), arm c/s; free rows, arm b) is used for BOTH the input lookup (via
`inputs_embeds`) and a tied output head (`hidden @ W.T`), bypassing the model's own lm_head so the
three-way tie is exact. No sqrt(dim) embedding scaling (Llama, unlike Gemma/v11, does not scale).

Run: python3 cn1_model_hf.py   # loads SmolLM2-135M on CPU, extends vocab, structural self-test
"""
from __future__ import annotations

import json
from pathlib import Path

import torch
import torch.nn as nn
import torch.nn.functional as F

from cn1_model import Wf, load_fingerprint_features, _derangement, SHUFFLE_SEED

HERE = Path(__file__).resolve().parent
BASE_NAME = "HuggingFaceTB/SmolLM2-135M"
TOKENS_FILE = HERE / "cn1_cell_tokens.txt"  # <call>, </call>, <cell:NAME>... in library order
HF_TOKEN_MAP = HERE / "cn1_cell_token_map_smollm2.json"


def load_base_and_extend():
    """Load SmolLM2-135M, add the 792 cell/delimiter tokens as atomic special tokens, resize the
    (tied) embeddings, and return (model, tokenizer, cell_first_id, n_added, base_rows)."""
    from transformers import AutoModelForCausalLM, AutoTokenizer

    tok = AutoTokenizer.from_pretrained(BASE_NAME)
    model = AutoModelForCausalLM.from_pretrained(BASE_NAME, dtype=torch.float32)
    base_rows = model.get_input_embeddings().weight.shape[0]

    tokens = [t for t in TOKENS_FILE.read_text().splitlines() if t]  # order = library order
    n_added = tok.add_tokens(tokens, special_tokens=True)  # atomic, never split
    model.resize_token_embeddings(len(tok))  # re-ties automatically (tie_word_embeddings=True)

    # cell tokens are contiguous at the tail; <call>/</call> precede the <cell:*> block
    ids = {t: tok.convert_tokens_to_ids(t) for t in tokens}
    cell_ids = sorted(v for k, v in ids.items() if k.startswith("<cell:"))
    assert cell_ids == list(range(cell_ids[0], cell_ids[0] + len(cell_ids))), "cell ids not contiguous"
    HF_TOKEN_MAP.write_text(json.dumps(ids, indent=0))
    return model, tok, cell_ids[0], n_added, base_rows


class CN1ModelHF(nn.Module):
    def __init__(self, base, tok, cell_first_id, fp_feats, arm):
        super().__init__()
        assert arm in ("fingerprint", "shuffled", "random", "description")
        self.base = base
        self.arm = arm
        self.tok = tok
        self.dim = base.config.hidden_size
        self.cell_first_id = cell_first_id
        n_cells = fp_feats.shape[0]
        self.register_buffer("fp_feats", fp_feats)
        if arm in ("fingerprint", "shuffled", "description"):
            self.w_f = Wf(fp_feats.shape[1], self.dim)

    @property
    def embed_weight(self):
        return self.base.get_input_embeddings().weight

    def effective_embed_weight(self):
        w = self.embed_weight
        if self.arm == "random":
            return w
        cell_rows = self.w_f(self.fp_feats)
        lo = self.cell_first_id
        hi = lo + cell_rows.shape[0]
        parts = [w[:lo], cell_rows]
        if hi < w.shape[0]:
            parts.append(w[hi:])
        return torch.cat(parts, dim=0)

    def forward(self, input_ids, attention_mask=None):
        w = self.effective_embed_weight()
        emb = F.embedding(input_ids, w)  # Llama: no sqrt(dim) scaling
        hidden = self.base.model(inputs_embeds=emb, attention_mask=attention_mask).last_hidden_state
        return hidden @ w.t()  # tied output head over the SAME effective matrix


def build_hf(arm):
    base, tok, cell_first_id, n_added, base_rows = load_base_and_extend()
    kind = "description" if arm == "description" else "fingerprint"
    feats, _, names, held = load_fingerprint_features(kind=kind)  # order = library order = token order
    if arm == "shuffled":
        perm = _derangement(feats.shape[0], SHUFFLE_SEED)
        feats = feats[perm.tolist()]
    model = CN1ModelHF(base, tok, cell_first_id, feats, arm)
    return model, tok, names, held, cell_first_id, base_rows


def _selftest():
    print(f"== loading {BASE_NAME}, extending vocab ==", flush=True)
    base, tok, cell_first_id, n_added, base_rows = load_base_and_extend()
    trained_snapshot = base.get_input_embeddings().weight[:base_rows].clone()
    print(f"  base rows {base_rows} -> {base.get_input_embeddings().weight.shape[0]} (+{n_added}); cell_first_id {cell_first_id}")

    # 1. resize preserved trained rows + tying holds
    assert torch.equal(base.get_input_embeddings().weight[:base_rows], trained_snapshot), "trained rows changed"
    ie = base.get_input_embeddings().weight
    oe = base.get_output_embeddings().weight
    assert ie.data_ptr() == oe.data_ptr(), "tie broken after resize"
    print("  OK: trained rows preserved, embed/lm_head re-tied")

    feats, _, names, held = load_fingerprint_features()
    held_names = [n for n in names if n in held]
    seen_idx = next(i for i, n in enumerate(names) if n not in held)
    held_idx = names.index(held_names[0])

    model = CN1ModelHF(base, tok, cell_first_id, feats, "fingerprint")
    w_eff = model.effective_embed_weight()
    direct = model.w_f(model.fp_feats)
    for label, idx in [("seen", seen_idx), ("held-out", held_idx)]:
        assert torch.allclose(w_eff[cell_first_id + idx], direct[idx], atol=1e-6), f"{label} row != W_f(FP)"
    print("  OK: arm(c) cell rows == W_f(FP) for seen and held-out; input matrix IS output matrix")

    # 2. gate-(ii) mechanism: a step on a SEEN row moves a HELD-OUT row through shared W_f
    before = model.effective_embed_weight()[cell_first_id + held_idx].detach().clone()
    opt = torch.optim.SGD(model.w_f.parameters(), lr=1e-2)
    loss = model.effective_embed_weight()[cell_first_id + seen_idx].pow(2).sum()
    opt.zero_grad(); loss.backward()
    gnorm = sum(p.grad.norm().item() for p in model.w_f.parameters() if p.grad is not None)
    opt.step()
    moved = (model.effective_embed_weight()[cell_first_id + held_idx].detach() - before).norm().item()
    assert gnorm > 0 and moved > 0, "gate-(ii) mechanism absent"
    print(f"  OK: W_f grad-norm {gnorm:.3f}; step on SEEN row moved HELD-OUT row by {moved:.4e}")

    # 3. forward runs and produces logits over the extended vocab
    ids = torch.tensor([[1, 100, 200, cell_first_id + seen_idx, 2]])
    with torch.no_grad():
        logits = model(ids)
    V = base.get_input_embeddings().weight.shape[0]
    assert logits.shape == (1, 5, V), f"bad logits {tuple(logits.shape)}"
    print(f"  OK: forward -> logits {tuple(logits.shape)} over the extended vocab")
    print("\nself-test: PASS — SmolLM2 base swap wired for three-way tying; ready to train when the M3 frees")


if __name__ == "__main__":
    _selftest()
