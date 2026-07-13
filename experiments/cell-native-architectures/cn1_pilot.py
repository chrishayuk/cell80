#!/usr/bin/env python3
"""CN-1 slice-0 pilot, iteration 3 — cell tokens with fingerprint embeddings, toy scale
(`experiments/cell-native-architectures.md`'s CN-1, following CN-0's gate not being met and
CN-3 scoping out for Gemma-class models: the redirect to depth 1).

Iteration 1 held out `is_ge`/`argmax3`, each with its OWN never-trained input token: both
scored 0.000 for both embedding-init arms, because the model never processed that token at
all, regardless of embedding placement. Iteration 2 tried recombining already-trained
tokens ("discount"+"->", "discount"+"?") into a never-seen-together sequence: STILL 0.000
for both arms — a deeper failure than iteration 1's. Both failures are "the model can't
reach a hidden state where the embedding could matter" wearing different clothes: iteration
1's token was never processed; iteration 2's *combination* was never processed, even though
each token was. Neither is a fingerprint result — both are capacity/compositional-
generalization results, and a from-scratch toy model may simply have none to speak of.

**Iteration 3, the last one (pre-registered, hard stop either way):** a genuine 3x2
compositional GRID -- CATEGORY in {cat1,cat2,cat3} x VARIANT in {var1,var2}, each
combination mapping to one of 6 pilot cells, template `"{a} {cat} {b} {var} ->"` uniform
across the whole grid. Train on 5 of 6 combinations with many examples each, so every
category AND every variant token gets heavy exposure across MULTIPLE partners (a genuine
basis for learning that category and variant compose independently to select a cell, not
just memorizing 5 point facts). Hold out exactly 1 combination entirely.

**Pre-registered bar, stated before this iteration was run:**
PASS: fingerprint-init's accuracy on the held-out combination exceeds 0.5, while
      random-init's stays <=0.25 (near the ~1/6 chance level for 6 candidate cells) -- a
      clear qualitative gap, not noise.
FAIL: both arms land in the same range (both near-chance, or both similarly elevated) --
      gate (ii) is not demonstrated at toy scale. Per the user's own fork: a FAIL here means
      toy scale cannot test this question at all (no model with any compositional
      generalization to modulate), not "redesign the corpus a third time." The real CN-1
      build (TinyModel v11 + the H1 factory's actual training spend) is where gate (ii)
      gets tested next, not another toy iteration.

Run: python3 cn1_pilot.py
"""
from __future__ import annotations

import json
import subprocess
import time
from pathlib import Path

import cell80_py
import mlx.core as mx
import mlx.nn as nn
import mlx.optimizers as optim
import numpy as np

CELLS_DIR = Path(__file__).resolve().parent.parent.parent / "cell80" / "cells"
DUMP_FINGERPRINTS = (
    Path(__file__).resolve().parent.parent.parent / "target" / "release" / "examples" / "dump_fingerprints"
)

CATEGORIES = ["cat1", "cat2", "cat3"]
VARIANTS = ["var1", "var2"]

# The 3x2 grid: every (category, variant) pair maps to one distinct pilot cell, all arity-2.
# Assignment is arbitrary (the category/variant tokens are synthetic slot labels, not
# semantically meaningful words) -- the point is uniform template structure across all 6, so
# category and variant are independently learnable factors, not per-cell idiosyncrasies.
CELL_GRID = {
    ("cat1", "var1"): "add_sat",
    ("cat1", "var2"): "sub_sat",
    ("cat2", "var1"): "is_gt",
    ("cat2", "var2"): "is_ge",
    ("cat3", "var1"): "discount_percent",
    ("cat3", "var2"): "mul_sat",  # HELD OUT: never trained, both its category (cat3) and
}                                  # variant (var2) tokens are heavily trained via other cells.
HELD_OUT_COMBO = ("cat3", "var2")
ALL_CELLS = list(CELL_GRID.values())

N_PER_CELL = 300
SEED = 7
EMBED_DIM = 64
N_LAYERS = 3
N_HEADS = 4
FFN_DIM = 256
EPOCHS = 60
LR = 3e-3
BATCH = 64


def cell_host():
    """Returns (host, handles) -- `handles[name]` is the warm handle `run` needs, per
    `CellHost.add_source`/`load`'s real signature (`add_source(id, src_text)` compiles from
    raw source text, not a directory; `load(id)` returns an integer handle; `run(handle,
    args)` returns `{result, regs, cycles, trapped_ops, halt}`, not a bare scalar)."""
    host = cell80_py.CellHost()
    handles = {}
    for name in ALL_CELLS:
        src_path = next(CELLS_DIR.rglob(f"{name}.rs"))
        host.add_source(name, src_path.read_text())
        handles[name] = host.load(name)
    return host, handles


def run_cell(host, handles, name, args):
    return host.run(handles[name], list(args))["result"]


def template(cat, var, a, b):
    return f"{a} {cat} {b} {var} ->"


def grid_examples(host, handles, cat, var, n, rng):
    """cell80's own execution IS the label -- there's no separate ground truth for any of
    these 6 cells to diverge from; the grid's job is testing compositional generalization
    over the (category, variant) -> cell association, not verifying arithmetic.
    """
    cell = CELL_GRID[(cat, var)]
    out = []
    for _ in range(n):
        a, b = int(rng.integers(1, 100)), int(rng.integers(1, 100))
        result = run_cell(host, handles, cell, [a, b])
        out.append((a, b, result, template(cat, var, a, b), cell))
    return out


# ---- toy vocabulary + tokenizer (trivial to extend -- defined here, not loaded) ----

def build_vocab():
    vocab = ["<pad>", "<bos>", "<eos>"]
    vocab += [str(d) for d in range(10)]  # digit-by-digit number encoding
    vocab += CATEGORIES + VARIANTS + ["->", "<call>", "</call>"]
    vocab += ALL_CELLS
    stoi = {t: i for i, t in enumerate(vocab)}
    return vocab, stoi


def tokenize_number(n, stoi):
    return [stoi[d] for d in str(n)]


def tokenize_example(text, cell_name, stoi):
    """`text` is e.g. "12 cat1 7 var1 ->"; split on whitespace, numbers go digit-by-digit,
    everything else is a single token. Target: <call> CELL_TOKEN </call>.
    """
    ids = [stoi["<bos>"]]
    for piece in text.split():
        if piece.lstrip("-").isdigit():
            ids += tokenize_number(piece, stoi)
        else:
            ids.append(stoi[piece])
    call_pos = len(ids) + 1  # position of the CELL_TOKEN itself, 1 after <call>
    ids += [stoi["<call>"], stoi[cell_name], stoi["</call>"], stoi["<eos>"]]
    return ids, call_pos


# ---- toy causal transformer (MLX) ----

class Block(nn.Module):
    def __init__(self, dim, n_heads, ffn_dim):
        super().__init__()
        self.attn = nn.MultiHeadAttention(dim, n_heads)
        self.ln1 = nn.LayerNorm(dim)
        self.ffn = nn.Sequential(nn.Linear(dim, ffn_dim), nn.GELU(), nn.Linear(ffn_dim, dim))
        self.ln2 = nn.LayerNorm(dim)

    def __call__(self, x, mask):
        h = self.ln1(x)
        x = x + self.attn(h, h, h, mask=mask)
        x = x + self.ffn(self.ln2(x))
        return x


class ToyTransformer(nn.Module):
    """Tied embeddings/output projection (matching TinyModel v11's own `lm_head.weight =
    embed.weight` -- Gemma-style). Iteration 1's own finding, restated: this tying is the
    only mechanism by which a fingerprint-placed embedding could influence a prediction at
    all -- an untied output head has no reason to reflect embedding-space geometry.
    """

    def __init__(self, vocab_size, dim, n_layers, n_heads, ffn_dim, max_len=32):
        super().__init__()
        self.embed = nn.Embedding(vocab_size, dim)
        self.pos = nn.Embedding(max_len, dim)
        self.blocks = [Block(dim, n_heads, ffn_dim) for _ in range(n_layers)]
        self.ln_f = nn.LayerNorm(dim)

    def __call__(self, ids):
        seq_len = ids.shape[1]
        x = self.embed(ids) + self.pos(mx.arange(seq_len))
        mask = nn.MultiHeadAttention.create_additive_causal_mask(seq_len)
        for b in self.blocks:
            x = b(x, mask)
        # Tied projection: logits = hidden @ embed.weight^T, not a separate learned head.
        return self.ln_f(x) @ self.embed.weight.T


def pad_batch(seqs, pad_id):
    max_len = max(len(s) for s in seqs)
    return np.array([s + [pad_id] * (max_len - len(s)) for s in seqs])


def train_arm(vocab, stoi, train_seqs, init_embeddings, epochs, lr):
    model = ToyTransformer(len(vocab), EMBED_DIM, N_LAYERS, N_HEADS, FFN_DIM)
    mx.eval(model.parameters())
    if init_embeddings is not None:
        model.embed.weight = mx.array(init_embeddings)

    opt = optim.Adam(learning_rate=lr)

    def loss_fn(model, ids):
        logits = model(ids[:, :-1])
        targets = ids[:, 1:]
        pad_mask = (targets != stoi["<pad>"]).astype(mx.float32)
        losses = nn.losses.cross_entropy(logits, targets)
        return (losses * pad_mask).sum() / pad_mask.sum()

    lv_and_grad = nn.value_and_grad(model, loss_fn)
    rng = np.random.default_rng(SEED)
    for epoch in range(epochs):
        perm = rng.permutation(len(train_seqs))
        total_loss = 0.0
        n_batches = 0
        for i in range(0, len(train_seqs), BATCH):
            idx = perm[i : i + BATCH]
            batch = pad_batch([train_seqs[j] for j in idx], stoi["<pad>"])
            ids = mx.array(batch)
            loss, grads = lv_and_grad(model, ids)
            opt.update(model, grads)
            mx.eval(model.parameters(), opt.state)
            total_loss += float(loss)
            n_batches += 1
        if (epoch + 1) % 20 == 0:
            print(f"    epoch {epoch+1}/{epochs}  loss={total_loss/n_batches:.4f}", flush=True)
    return model


def eval_accuracy(model, examples, stoi):
    """Accuracy of the argmax next-token prediction at the CELL_TOKEN position, given
    everything up to and including <call> as the prompt -- not full generation.
    """
    correct = 0
    for a, b, result, text, cell in examples:
        ids, call_pos = tokenize_example(text, cell, stoi)
        prompt = mx.array([ids[:call_pos]])
        logits = model(prompt)
        pred = int(mx.argmax(logits[0, -1]).item())
        if pred == stoi[cell]:
            correct += 1
    return correct / len(examples) if examples else float("nan")


def main():
    t0 = time.time()
    rng = np.random.default_rng(SEED)

    print("== loading cell80-py CellHost (the exact oracle) ==", flush=True)
    host, handles = cell_host()

    print("== generating toy corpus (3x2 compositional grid) ==", flush=True)
    train_seqs = []
    test_by_combo = {}
    vocab, stoi = build_vocab()
    print(f"vocab size: {len(vocab)}", flush=True)

    for cat in CATEGORIES:
        for var in VARIANTS:
            cell = CELL_GRID[(cat, var)]
            examples = grid_examples(host, handles, cat, var, N_PER_CELL, rng)
            print(f"  {cat} x {var} -> {cell:<18} {len(examples)} examples", flush=True)
            if (cat, var) == HELD_OUT_COMBO:
                test_by_combo[(cat, var)] = examples  # ALL held out, zero training
                continue
            idx = rng.permutation(len(examples))
            n_test = max(1, len(examples) // 5)
            te_idx, tr_idx = idx[:n_test], idx[n_test:]
            for i in tr_idx:
                a, b, result, text, c = examples[i]
                ids, _ = tokenize_example(text, c, stoi)
                train_seqs.append(ids)
            test_by_combo[(cat, var)] = [examples[i] for i in te_idx]

    print(f"\ntraining examples: {len(train_seqs)}", flush=True)

    print("\n== computing fingerprints (dump_fingerprints subprocess) ==", flush=True)
    proc = subprocess.run(
        [str(DUMP_FINGERPRINTS), *ALL_CELLS], capture_output=True, text=True, check=True
    )
    fingerprints = json.loads(proc.stdout)

    proj_rng = np.random.default_rng(SEED + 1)
    fp_len = len(next(iter(fingerprints.values())))
    projection = proj_rng.normal(0, 1.0 / np.sqrt(fp_len), size=(fp_len, EMBED_DIM))

    def fingerprint_vec(cell):
        raw = np.array([0 if v is None else v for v in fingerprints[cell]], dtype=np.float32)
        return raw @ projection

    base_scale = 0.02
    random_init = proj_rng.normal(0, base_scale, size=(len(vocab), EMBED_DIM)).astype(np.float32)
    fingerprint_init = random_init.copy()
    for cell in ALL_CELLS:
        vec = fingerprint_vec(cell)
        vec = vec / (np.linalg.norm(vec) + 1e-6) * base_scale * np.sqrt(EMBED_DIM)
        fingerprint_init[stoi[cell]] = vec

    print(f"\n== arm (b): random-init cell embeddings ({time.time()-t0:.1f}s) ==", flush=True)
    model_b = train_arm(vocab, stoi, train_seqs, random_init, EPOCHS, LR)

    print(f"\n== arm (c): fingerprint-init cell embeddings ({time.time()-t0:.1f}s) ==", flush=True)
    model_c = train_arm(vocab, stoi, train_seqs, fingerprint_init, EPOCHS, LR)

    print(f"\n== results ({time.time()-t0:.1f}s total) ==")
    print(f"{'combo':<16} {'held?':<10} {'(b) random':>12} {'(c) fingerprint':>16}")
    acc_b_by_combo, acc_c_by_combo = {}, {}
    for cat in CATEGORIES:
        for var in VARIANTS:
            examples = test_by_combo[(cat, var)]
            acc_b_by_combo[(cat, var)] = eval_accuracy(model_b, examples, stoi)
            acc_c_by_combo[(cat, var)] = eval_accuracy(model_c, examples, stoi)
            held = "held-out" if (cat, var) == HELD_OUT_COMBO else "trained"
            print(
                f"{cat}+{var:<10} {held:<10} {acc_b_by_combo[(cat,var)]:>12.3f} "
                f"{acc_c_by_combo[(cat,var)]:>16.3f}"
            )

    trained_combos = [k for k in CELL_GRID if k != HELD_OUT_COMBO]
    trained_b = np.mean([acc_b_by_combo[k] for k in trained_combos])
    trained_c = np.mean([acc_c_by_combo[k] for k in trained_combos])
    held_b = acc_b_by_combo[HELD_OUT_COMBO]
    held_c = acc_c_by_combo[HELD_OUT_COMBO]
    print(f"\n{'mean (trained combos)':<24} {trained_b:>12.3f} {trained_c:>16.3f}")
    print(f"{'held-out combo':<24} {held_b:>12.3f} {held_c:>16.3f}")

    # The pre-registered bar, stated before this iteration was run (see module docstring).
    passed = held_c > 0.5 and held_b <= 0.25
    print(
        f"\n== pre-registered gate (ii) verdict: {'PASS' if passed else 'FAIL'} ==\n"
        f"   bar: fingerprint-init > 0.5 AND random-init <= 0.25 on the held-out combo\n"
        f"   actual: fingerprint-init={held_c:.3f}, random-init={held_b:.3f}"
    )
    if passed:
        print(
            "   Gate (ii) demonstrated at toy scale: a fingerprint-placed embedding gave a "
            "never-trained cell a meaningful address a random one couldn't."
        )
    else:
        print(
            "   Gate (ii) NOT demonstrated. Per the pre-registered fork: this is the last toy "
            "iteration -- the real CN-1 build (TinyModel v11 + the H1 factory's actual "
            "training spend) is where gate (ii) gets tested next, not a fourth corpus redesign."
        )

    results = {
        "vocab_size": len(vocab),
        "n_per_cell": N_PER_CELL,
        "epochs": EPOCHS,
        "held_out_combo": list(HELD_OUT_COMBO),
        "held_out_cell": CELL_GRID[HELD_OUT_COMBO],
        "acc_random_by_combo": {f"{k[0]}+{k[1]}": v for k, v in acc_b_by_combo.items()},
        "acc_fingerprint_by_combo": {f"{k[0]}+{k[1]}": v for k, v in acc_c_by_combo.items()},
        "mean_trained_random": trained_b,
        "mean_trained_fingerprint": trained_c,
        "held_out_random": held_b,
        "held_out_fingerprint": held_c,
        "gate_ii_pass": passed,
    }
    out_path = Path(__file__).resolve().parent / "cn1_pilot_results.json"
    out_path.write_text(json.dumps(results, indent=2))
    print(f"\nwrote {out_path}")


if __name__ == "__main__":
    main()
