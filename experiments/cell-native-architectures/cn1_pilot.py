#!/usr/bin/env python3
"""CN-1 slice-0 pilot — cell tokens with fingerprint embeddings, toy scale
(`experiments/cell-native-architectures.md`'s CN-1, following CN-0's gate not being met and
CN-3 scoping out for Gemma-class models: the redirect to depth 1).

Full CN-1 needs TinyModel v11 + its tokenizer, the H1 factory (three corpus sources,
strict-improvement filter, admission-style dedup), a ~800-cell vocabulary, and ported
constrained decoding — real infrastructure work across five repos. This pilot tests the one
question that actually decides whether that investment is worth making: does a small model
learn to associate a problem with the *right* cell-identity token at all, and does
fingerprint-init give it an addressing advantage over random-init specifically on cells
never invoked during training (the only mechanism by which an unseen cell could have a
meaningful address — CN-1's own novelty gate, in miniature).

**A scope adjustment found during research, not assumed going in**: TinyModel v11
(`~/chris-source/tiny-model`) is PyTorch, not MLX, and its tokenizer loads a pre-built,
immutable `.vocab.bin` with no `add_tokens` API -- extending it means rebuilding the vocab
and recompiling a Rust/PyO3 extension, a real detour for a pilot. This script builds a
small, self-contained MLX transformer + toy vocabulary from scratch instead -- trivial to
add cell tokens to, since the vocab is defined here, not loaded from a fixed file. The real
TinyModel v11 integration is deferred to the full CN-1 build.

Corpus: `chuk_math_gym`'s arithmetic generator (VERY_EASY difficulty) for the three
arithmetic cells (add_sat/sub_sat/mul_sat), filtered to simple `a op b` expressions (VERY_EASY
still occasionally chains 2-3 operators; anything not matching a single binary op is
discarded, not force-parsed) and cross-checked against its own independently-computed
`gold_answer`. The four non-arithmetic pilot cells (is_gt/is_ge/discount_percent/argmax3)
have no independent domain generator to check against, so their own cell80 execution *is*
the label (there's no separate ground truth for "is 12 >= 7" to diverge from). Every
example, regardless of source, is verified against a REAL cell80 run via `cell80-py`'s
`CellHost` before being admitted -- the exact-oracle discipline, not trusting either
chuk_math_gym's arithmetic or hand-written Python to match cell80's own execution.

Run: python3 cn1_pilot.py
"""
from __future__ import annotations

import json
import re
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path.home() / "chris-source/chuk-math/src"))

import cell80_py
import mlx.core as mx
import mlx.nn as nn
import mlx.optimizers as optim
import numpy as np
from chuk_math_gym.domains.arithmetic.generator import ArithmeticGenerator
from chuk_math_gym.schemas.problem import DifficultyLevel

CELLS_DIR = Path(__file__).resolve().parent.parent.parent / "cell80" / "cells"
DUMP_FINGERPRINTS = (
    Path(__file__).resolve().parent.parent.parent / "target" / "release" / "examples" / "dump_fingerprints"
)

# Pilot cells: 5 trained-on, 2 held out entirely (never invoked in the training corpus) --
# the direct toy analogue of CN-1's novelty gate. Arity per cell (for calling CellHost.run).
# Second iteration, after the first attempt (holding out `is_ge` with its own `>=` token,
# `argmax3` with its own `max` token) found BOTH scored 0.000 regardless of embedding
# strategy -- because each held-out cell's defining input token never appeared in training
# at all, so the model had no hidden state to work from, full stop. This iteration's
# held-out cells (`mul_sat`, `is_ge`) instead use TEMPLATES that recombine tokens already
# trained elsewhere ("discount"/"->"/"?") in a never-seen-together combination -- every
# individual token has real gradient signal; only the specific input-to-cell association is
# withheld, the constraint the first iteration derived. `argmax3` moved to trained (so "max"
# gets real gradient signal too, instead of being a second untestable held-out case).
TRAIN_CELLS = {
    "add_sat": 2,
    "sub_sat": 2,
    "argmax3": 3,
    "is_gt": 2,
    "discount_percent": 2,
}
HELD_OUT_CELLS = {
    "mul_sat": 2,
    "is_ge": 2,
}
ALL_CELLS = {**TRAIN_CELLS, **HELD_OUT_CELLS}

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


# ---- corpus: arithmetic cells via chuk_math_gym, verified against cell80 ----

SIMPLE_EXPR = re.compile(r"^(\d+)\s*([+\-*])\s*(\d+)$")
OP_TO_CELL = {"+": "add_sat", "-": "sub_sat"}

# Per-cell prompt template. The two held-out cells (`mul_sat`, `is_ge`) deliberately do NOT
# get a novel operator symbol -- they REUSE tokens already trained elsewhere ("discount" via
# discount_percent's own training, "->" via add_sat/sub_sat, "?" via is_gt) in a combination
# never seen during training. Every individual token has real gradient signal; only the
# specific input-to-cell ASSOCIATION is held out -- the constraint the first pilot iteration
# found the hard way (an unseen operator token like the original `>=` gives the model no
# hidden state to work from at all, regardless of embedding placement).
TEMPLATES = {
    "add_sat": lambda a, b: f"{a} + {b} ->",
    "sub_sat": lambda a, b: f"{a} - {b} ->",
    "is_gt": lambda a, b: f"{a} > {b} ?",
    "discount_percent": lambda a, b: f"{a} discount {b % 100} % ->",
    "argmax3": lambda a, b, c: f"max {a} {b} {c} ->",
    "mul_sat": lambda a, b: f"{a} discount {b} ->",  # "discount"+"->" recombined, never seen
    "is_ge": lambda a, b: f"{a} discount {b} ?",  # "discount"+"?" recombined, never seen
}


def arithmetic_examples(host, handles, n_per_op, rng):
    """chuk_math_gym VERY_EASY problems, filtered to a single binary op (VERY_EASY still
    occasionally chains 2-3 operators; anything else is discarded, not force-parsed), then
    verified against a real cell80 run -- if chuk_math_gym's gold_answer and cell80's own
    add_sat/sub_sat disagree (e.g. saturation at the u16 boundary), the example is discarded,
    not silently trusted either way. (`mul_sat` is generated directly, not via chuk_math_gym
    -- it's a held-out cell now, rendered with a deliberately recombined template, not its
    "natural" `a * b ->` one; see `direct_examples`.)
    """
    gen = ArithmeticGenerator()
    by_cell = {c: [] for c in OP_TO_CELL.values()}
    seed = int(rng.integers(0, 2**31))
    tries = 0
    while any(len(v) < n_per_op for v in by_cell.values()) and tries < n_per_op * len(OP_TO_CELL) * 20:
        tries += 1
        seed += 1
        problem, _ = gen.generate(seed=seed, difficulty=DifficultyLevel.VERY_EASY)
        m = SIMPLE_EXPR.match(problem.expression.strip())
        if not m:
            continue
        a, op, b = int(m.group(1)), m.group(2), int(m.group(3))
        if op not in OP_TO_CELL:
            continue  # "*" -- mul_sat is held-out now, generated directly (see main())
        cell = OP_TO_CELL[op]
        if len(by_cell[cell]) >= n_per_op or a > 65535 or b > 65535:
            continue
        gold = int(problem.gold_answer)
        if gold < 0 or gold > 65535:
            continue
        result = run_cell(host, handles, cell, [a, b])
        if result != gold:
            continue  # disagreement between chuk_math_gym and cell80 -- discard, don't force
        by_cell[cell].append((a, b, result, TEMPLATES[cell](a, b)))
    return by_cell


def direct_examples(host, handles, cell, arity, n, rng):
    """Cells with no independent domain generator (or, for `mul_sat`, deliberately not using
    one -- see `TEMPLATES`) -- cell80's own execution IS the label (there's no separate spec
    for "is 12 >= 7" or "12 discount 7" to diverge from).
    """
    out = []
    for _ in range(n):
        args = [int(rng.integers(1, 100)) for _ in range(arity)]
        result = run_cell(host, handles, cell, args)
        text = TEMPLATES[cell](*args)
        out.append((*args, result, text))
    return out


# ---- toy vocabulary + tokenizer (trivial to extend -- defined here, not loaded) ----

def build_vocab():
    vocab = ["<pad>", "<bos>", "<eos>"]
    vocab += [str(d) for d in range(10)]  # digit-by-digit number encoding
    vocab += ["+", "-", "*", ">", ">=", "discount", "%", "max", "->", "?", "<call>", "</call>"]
    vocab += list(ALL_CELLS.keys())
    stoi = {t: i for i, t in enumerate(vocab)}
    return vocab, stoi


def tokenize_number(n, stoi):
    return [stoi[d] for d in str(n)]


def tokenize_example(text, cell_name, stoi):
    """`text` is e.g. "12 + 7 ->"; split on whitespace, numbers go digit-by-digit, symbols
    are single tokens. Target: <call> CELL_TOKEN </call>.
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
    """Tied embeddings/output projection (`head`'s weight IS `embed`'s weight, matching
    TinyModel v11's own `self.lm_head.weight = self.embed.weight` -- Gemma-style). This
    tying is not a detail: it's the *only* mechanism by which a fingerprint-placed embedding
    could give a held-out cell a non-zero prediction probability without ever training on
    it -- an untied output head has no reason to reflect the embedding-space geometry at
    all, and would silently make the fingerprint-vs-random comparison meaningless.
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


def eval_cell_accuracy(model, examples_by_cell, stoi):
    """Accuracy of the argmax next-token prediction at the CELL_TOKEN position, given
    everything up to and including <call> as the prompt -- not full generation.
    """
    out = {}
    for cell, examples in examples_by_cell.items():
        correct = 0
        for ex in examples:
            *args, result, text = ex
            ids, call_pos = tokenize_example(text, cell, stoi)
            prompt = mx.array([ids[:call_pos]])
            logits = model(prompt)
            pred = int(mx.argmax(logits[0, -1]).item())
            if pred == stoi[cell]:
                correct += 1
        out[cell] = correct / len(examples) if examples else float("nan")
    return out


def main():
    t0 = time.time()
    rng = np.random.default_rng(SEED)

    print("== loading cell80-py CellHost (the exact oracle) ==", flush=True)
    host, handles = cell_host()

    print("== generating toy corpus ==", flush=True)
    arith = arithmetic_examples(host, handles, N_PER_CELL, rng)
    corpus = dict(arith)
    for cell, arity in list(TRAIN_CELLS.items()) + list(HELD_OUT_CELLS.items()):
        if cell in corpus:
            continue
        corpus[cell] = direct_examples(host, handles, cell, arity, N_PER_CELL, rng)
    for cell, examples in corpus.items():
        print(f"  {cell:<18} {len(examples)} examples", flush=True)

    vocab, stoi = build_vocab()
    print(f"vocab size: {len(vocab)}", flush=True)

    # Train/test split per trained-on cell; held-out cells are NEVER in the training set.
    train_seqs = []
    test_by_cell = {}
    for cell in TRAIN_CELLS:
        examples = corpus[cell]
        idx = rng.permutation(len(examples))
        n_test = max(1, len(examples) // 5)
        te_idx, tr_idx = idx[:n_test], idx[n_test:]
        for i in tr_idx:
            *args, result, text = examples[i]
            ids, _ = tokenize_example(text, cell, stoi)
            train_seqs.append(ids)
        test_by_cell[cell] = [examples[i] for i in te_idx]
    for cell in HELD_OUT_CELLS:
        test_by_cell[cell] = corpus[cell]  # entirely held out -- zero training examples

    print(f"\ntraining examples: {len(train_seqs)}", flush=True)

    print("\n== computing fingerprints (dump_fingerprints subprocess) ==", flush=True)
    names = list(ALL_CELLS.keys())
    proc = subprocess.run(
        [str(DUMP_FINGERPRINTS), *names], capture_output=True, text=True, check=True
    )
    fingerprints = json.loads(proc.stdout)

    # Project each cell's fingerprint (a vector of Option<u16>, None -> 0) into EMBED_DIM via
    # a FIXED random projection (same matrix for every cell, not learned) -- deterministic,
    # not trained, matching "embedding rows projected from behavioural fingerprint" plainly.
    proj_rng = np.random.default_rng(SEED + 1)
    fp_len = len(next(iter(fingerprints.values())))
    projection = proj_rng.normal(0, 1.0 / np.sqrt(fp_len), size=(fp_len, EMBED_DIM))

    def fingerprint_vec(cell):
        raw = np.array([0 if v is None else v for v in fingerprints[cell]], dtype=np.float32)
        return raw @ projection

    base_scale = 0.02  # matches a typical small nn.Embedding init scale
    random_init = proj_rng.normal(0, base_scale, size=(len(vocab), EMBED_DIM)).astype(np.float32)
    fingerprint_init = random_init.copy()
    for cell in ALL_CELLS:
        vec = fingerprint_vec(cell)
        vec = vec / (np.linalg.norm(vec) + 1e-6) * base_scale * np.sqrt(EMBED_DIM)
        fingerprint_init[stoi[cell]] = vec

    print(f"\n== arm (b): random-init cell embeddings ({time.time()-t0:.1f}s) ==", flush=True)
    model_b = train_arm(vocab, stoi, train_seqs, random_init, EPOCHS, LR)
    acc_b = eval_cell_accuracy(model_b, test_by_cell, stoi)

    print(f"\n== arm (c): fingerprint-init cell embeddings ({time.time()-t0:.1f}s) ==", flush=True)
    model_c = train_arm(vocab, stoi, train_seqs, fingerprint_init, EPOCHS, LR)
    acc_c = eval_cell_accuracy(model_c, test_by_cell, stoi)

    print(f"\n== results ({time.time()-t0:.1f}s total) ==")
    print(f"{'cell':<18} {'held?':<10} {'(b) random':>12} {'(c) fingerprint':>16}")
    for cell in list(TRAIN_CELLS) + list(HELD_OUT_CELLS):
        held = "held-out" if cell in HELD_OUT_CELLS else "trained"
        print(f"{cell:<18} {held:<10} {acc_b[cell]:>12.3f} {acc_c[cell]:>16.3f}")

    trained_b = np.mean([acc_b[c] for c in TRAIN_CELLS])
    trained_c = np.mean([acc_c[c] for c in TRAIN_CELLS])
    held_b = np.mean([acc_b[c] for c in HELD_OUT_CELLS])
    held_c = np.mean([acc_c[c] for c in HELD_OUT_CELLS])
    print(f"\n{'mean (trained cells)':<24} {trained_b:>12.3f} {trained_c:>16.3f}")
    print(f"{'mean (held-out cells)':<24} {held_b:>12.3f} {held_c:>16.3f}")
    print(
        f"\nfingerprint - random, held-out: {held_c - held_b:+.3f} "
        "(the toy analogue of CN-1's novelty gate: positive here is the only evidence "
        "a fingerprint-derived address does something a random one couldn't)"
    )

    results = {
        "vocab_size": len(vocab),
        "n_per_cell": N_PER_CELL,
        "epochs": EPOCHS,
        "acc_random_init": acc_b,
        "acc_fingerprint_init": acc_c,
        "mean_trained_random": trained_b,
        "mean_trained_fingerprint": trained_c,
        "mean_held_out_random": held_b,
        "mean_held_out_fingerprint": held_c,
    }
    out_path = Path(__file__).resolve().parent / "cn1_pilot_results.json"
    out_path.write_text(json.dumps(results, indent=2))
    print(f"\nwrote {out_path}")


if __name__ == "__main__":
    main()
