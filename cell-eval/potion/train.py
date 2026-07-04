"""cell-potion trainer — domain-trained static embedder (docs/cell-potion-training-spec.md).

Trains the model2vec token table directly on (query ↔ manifest-doc) pairs with
InfoNCE over ALL 100 manifest docs as the negative set (the full candidate space —
strictly stronger than in-batch negatives at this library size), plus a weighted
restricted-CE term on authored near-miss hard negatives.

Pooling replicates model2vec inference exactly (verified bit-exact against
StaticModel.encode): plain mean of token vectors, then L2 normalise. So the saved
artifact behaves in the harness Embedder precisely as it behaved in training.

Protocol invariants:
- Input corpus is generated from manifests only. This script NEVER reads
  datasets/retrieval.jsonl (the frozen eval).
- Dev split is carved from the GENERATED corpus (deterministic hash of the query
  text). Hyperparameters are selected on dev only; the frozen eval is touched once,
  by the harness, after the artifact is final.
- Deterministic: fixed seed, no wall-clock anywhere.

Usage:
  python train.py --pairs ../datasets/potion-train-pairs.jsonl --out ./model [--sweep]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "src"))

from cell_eval.tiers import _doc, open_library  # noqa: E402

BASE_MODEL = "minishlab/potion-retrieval-32M"
SEED = 80


def dev_of(query: str) -> bool:
    """Deterministic ~25% dev split by query-text hash (stable across runs)."""
    return int(hashlib.sha256(query.encode()).hexdigest(), 16) % 4 == 0


def load_corpus(path: Path, cell_ids: set[str]) -> list[dict]:
    rows = []
    for line in path.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        r = json.loads(line)
        assert r["cell"] in cell_ids, f"unknown cell {r['cell']!r}"
        for n in r.get("hard_negatives", []):
            assert n in cell_ids, f"unknown hard negative {n!r}"
        rows.append(r)
    return rows


class Trainer:
    # v2 margin hinge (PROTOCOL.md "v2"): mu * max(0, gamma - (s_pos - max_other)),
    # on RAW cosine scores. Class defaults keep mu=0 == the banked v1 math exactly
    # (the gradient-check tests build Trainer via __new__ and rely on these).
    mu = 0.0
    gamma = 0.0

    def __init__(self, tau: float, lam: float, lr: float,
                 mu: float = 0.0, gamma: float = 0.0):
        from model2vec import StaticModel

        self.base = StaticModel.from_pretrained(BASE_MODEL)
        self.E = self.base.embedding.astype(np.float64).copy()
        self.tok = self.base.tokenizer
        self.tau, self.lam, self.lr = tau, lam, lr
        self.mu, self.gamma = mu, gamma
        # Adam state over the full table (sparse rows touched, dense state kept)
        self.m = np.zeros_like(self.E)
        self.v = np.zeros_like(self.E)
        self.t = 0

    def ids(self, text: str) -> list[int]:
        out = self.tok.encode(text, add_special_tokens=False).ids
        return out if out else [0]

    @staticmethod
    def pool(E: np.ndarray, toks: list[int]) -> tuple[np.ndarray, np.ndarray, float]:
        raw = E[toks].mean(axis=0)
        n = float(np.linalg.norm(raw))
        n = n if n > 0 else 1.0
        return raw / n, raw, n

    @staticmethod
    def unpool_grad(g_unit: np.ndarray, unit: np.ndarray, n: float) -> np.ndarray:
        # d/d raw of raw/||raw||, applied to upstream grad g_unit
        return (g_unit - unit * float(unit @ g_unit)) / n

    def encode_batch(self, tok_lists: list[list[int]]):
        units, raws, norms = [], [], []
        for toks in tok_lists:
            u, r, n = self.pool(self.E, toks)
            units.append(u)
            raws.append(r)
            norms.append(n)
        return np.array(units), raws, norms

    def step(self, batch: list[dict], doc_toks: list[list[int]], target_idx: list[int],
             hard_idx: list[list[int]]) -> float:
        B, C = len(batch), len(doc_toks)
        Q, q_units_raw, q_norms = self.encode_batch([r["_qtoks"] for r in batch])
        D, d_units_raw, d_norms = self.encode_batch(doc_toks)

        logits = (Q @ D.T) / self.tau
        # main CE over all docs
        p = np.exp(logits - logits.max(axis=1, keepdims=True))
        p /= p.sum(axis=1, keepdims=True)
        onehot = np.zeros_like(p)
        onehot[np.arange(B), target_idx] = 1.0
        loss = -np.log(p[np.arange(B), target_idx] + 1e-12).mean()
        dlogits = (p - onehot) / B

        # restricted CE on {target} ∪ authored hard negatives (adversarial rows)
        for i, negs in enumerate(hard_idx):
            if not negs:
                continue
            cols = [target_idx[i]] + negs
            sub = logits[i, cols]
            sp = np.exp(sub - sub.max())
            sp /= sp.sum()
            loss += self.lam * -np.log(sp[0] + 1e-12) / B
            sub_grad = sp.copy()
            sub_grad[0] -= 1.0
            for c, g in zip(cols, sub_grad):
                dlogits[i, c] += self.lam * g / B

        dlogits /= self.tau

        # v2 margin hinge on raw cosine scores (subgradient through the max;
        # ties broken by argmax, kinks measure zero under float scores)
        if self.mu > 0:
            S = logits * self.tau
            for i in range(B):
                pos = target_idx[i]
                srow = S[i].copy()
                s_pos = srow[pos]
                srow[pos] = -np.inf
                comp = int(np.argmax(srow))
                viol = self.gamma - (s_pos - srow[comp])
                if viol > 0:
                    loss += self.mu * viol / B
                    dlogits[i, pos] -= self.mu / B
                    dlogits[i, comp] += self.mu / B

        dQ = dlogits @ D
        dD = dlogits.T @ Q

        grad = {}  # row -> accumulated gradient

        def scatter(g_unit, unit, n, toks):
            g_raw = self.unpool_grad(g_unit, unit, n) / len(toks)
            for tk in toks:
                if tk in grad:
                    grad[tk] = grad[tk] + g_raw
                else:
                    grad[tk] = g_raw.copy()

        for i, r in enumerate(batch):
            scatter(dQ[i], Q[i], q_norms[i], r["_qtoks"])
        for c in range(C):
            scatter(dD[c], D[c], d_norms[c], doc_toks[c])

        # Adam on touched rows
        self.t += 1
        b1, b2, eps = 0.9, 0.999, 1e-8
        for row, g in grad.items():
            self.m[row] = b1 * self.m[row] + (1 - b1) * g
            self.v[row] = b2 * self.v[row] + (1 - b2) * g * g
            mh = self.m[row] / (1 - b1**self.t)
            vh = self.v[row] / (1 - b2**self.t)
            self.E[row] -= self.lr * mh / (np.sqrt(vh) + eps)
        return float(loss)

    def eval_rows(self, rows: list[dict], doc_toks: list[list[int]],
                  cell_pos: dict[str, int]) -> dict:
        if not rows:
            return {"n": 0}
        Q, _, _ = self.encode_batch([r["_qtoks"] for r in rows])
        D, _, _ = self.encode_batch(doc_toks)
        S = Q @ D.T
        out = {}
        for kind in ("paraphrase", "adversarial", "direct"):
            idx = [i for i, r in enumerate(rows) if r["kind"] == kind]
            if not idx:
                continue
            hits = sum(int(np.argmax(S[i]) == cell_pos[rows[i]["cell"]]) for i in idx)
            srt = np.sort(S[idx], axis=1)
            out[kind] = {"n": len(idx), "acc": round(hits / len(idx), 4),
                         "margin": round(float((srt[:, -1] - srt[:, -2]).mean()), 4)}
        return out

    M0 = 0.15  # fixed dev margin threshold; harness scale: blended theta ~= 0.75 x
    # cosine margin, so the real operating band (theta 0.11-0.14) sits near 0.15-0.19
    # in pure cosine. Fixed across configs, never tuned per run.

    def eval_gate_proxy(self, rows: list[dict], doc_toks: list[list[int]],
                        cell_pos: dict[str, int]) -> dict:
        """Dev analogue of the frozen-eval judge (PROTOCOL.md v2, amended): net
        coverage at a FIXED cosine-margin threshold M0, per split:
        P(correct AND margin >= M0) - P(wrong AND margin >= M0), summed. The
        original precision-calibrated theta_dev was degenerate on dev (the authored
        near-misses are harder than eval adversarial; theta_dev saturated at 0.30
        with zero adversarial coverage for every config) — see sweep2-results.jsonl."""
        Q, _, _ = self.encode_batch([r["_qtoks"] for r in rows])
        D, _, _ = self.encode_batch(doc_toks)
        S = Q @ D.T
        top = np.argmax(S, axis=1)
        correct = np.array([top[i] == cell_pos[r["cell"]] for i, r in enumerate(rows)])
        srt = np.sort(S, axis=1)
        confident = (srt[:, -1] - srt[:, -2]) >= self.M0
        kinds = np.array([r["kind"] for r in rows])

        out = {"m0": self.M0, "score": 0.0}
        for kind in ("paraphrase", "adversarial", "direct"):
            m = kinds == kind
            if not m.any():
                continue
            good = float((m & confident & correct).sum() / m.sum())
            bad = float((m & confident & ~correct).sum() / m.sum())
            out[kind] = {"net": round(good - bad, 4), "covered_correct": round(good, 4),
                         "covered_wrong": round(bad, 4)}
            out["score"] += good - bad
        out["score"] = round(out["score"], 4)
        return out

    def save(self, out: Path):
        from model2vec import StaticModel

        model = StaticModel(
            vectors=self.E.astype(np.float32),
            tokenizer=self.tok,
            config=dict(self.base.config),
            normalize=self.base.normalize,
            base_model_name=BASE_MODEL,
            language=getattr(self.base, "language", None),
        )
        model.save_pretrained(str(out))


def run(pairs: Path, out: Path | None, tau: float, lam: float, lr: float,
        epochs: int, batch_size: int = 256, log=print,
        mu: float = 0.0, gamma: float = 0.0, select: str = "acc") -> dict:
    lib = open_library(None)
    mans = lib.list()
    mans.sort(key=lambda m: m["id"])
    cell_ids = {m["id"] for m in mans}
    cell_pos = {m["id"]: i for i, m in enumerate(mans)}

    tr = Trainer(tau=tau, lam=lam, lr=lr, mu=mu, gamma=gamma)
    doc_toks = [tr.ids(_doc(m)) for m in mans]

    rows = load_corpus(pairs, cell_ids)
    for r in rows:
        r["_qtoks"] = tr.ids(r["query"])
    train = [r for r in rows if not dev_of(r["query"])]
    dev = [r for r in rows if dev_of(r["query"])]
    log(f"corpus: {len(rows)} rows → train {len(train)} / dev {len(dev)}")

    rng = np.random.default_rng(SEED)
    best = None
    for ep in range(epochs):
        order = rng.permutation(len(train))
        ep_loss = 0.0
        nb = 0
        for s in range(0, len(train), batch_size):
            batch = [train[j] for j in order[s:s + batch_size]]
            tgt = [cell_pos[r["cell"]] for r in batch]
            hn = [[cell_pos[n] for n in r.get("hard_negatives", [])] for r in batch]
            ep_loss += tr.step(batch, doc_toks, tgt, hn)
            nb += 1
        dv = tr.eval_rows(dev, doc_toks, cell_pos)
        if select == "gate":
            gp = tr.eval_gate_proxy(dev, doc_toks, cell_pos)
            score = gp["score"]
            log(f"epoch {ep + 1}: loss {ep_loss / nb:.4f} gate-proxy {json.dumps(gp)}")
        else:
            gp = None
            score = sum(dv[k]["acc"] for k in dv if k != "n" and isinstance(dv[k], dict))
            log(f"epoch {ep + 1}: loss {ep_loss / nb:.4f} dev {json.dumps(dv)}")
        if best is None or score >= best["score"]:
            best = {"score": score, "epoch": ep + 1, "dev": dv, "gate_proxy": gp,
                    "E": tr.E.copy() if out else None}

    result = {"tau": tau, "lam": lam, "lr": lr, "mu": mu, "gamma": gamma,
              "select": select, "epochs": epochs, "best_epoch": best["epoch"],
              "dev": best["dev"], "gate_proxy": best["gate_proxy"],
              "score": round(best["score"], 4)}
    if out:
        tr.E = best["E"]
        tr.save(out)
        result["artifact"] = str(out)
        log(f"saved artifact (best epoch {best['epoch']}) → {out}")
    return result


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--pairs", type=Path, default=HERE.parent / "datasets" / "potion-train-pairs.jsonl")
    ap.add_argument("--out", type=Path, default=None)
    ap.add_argument("--tau", type=float, default=0.1)
    ap.add_argument("--lam", type=float, default=0.5)
    ap.add_argument("--lr", type=float, default=5e-3)
    ap.add_argument("--epochs", type=int, default=30)
    ap.add_argument("--mu", type=float, default=0.0,
                    help="v2 margin-hinge weight (0 = banked v1 objective)")
    ap.add_argument("--gamma", type=float, default=0.0, help="v2 margin-hinge target")
    ap.add_argument("--select", choices=("acc", "gate"), default=None,
                    help="best-epoch criterion (default: acc, or gate when --mu > 0)")
    ap.add_argument("--sweep", action="store_true",
                    help="dev-split hyperparameter sweep (no artifact saved)")
    ap.add_argument("--sweep2", action="store_true",
                    help="v2 margin sweep: mu x gamma at v1's tau/lr, gate-proxy selection")
    a = ap.parse_args()

    if a.sweep2:
        results = []
        for mu in (0.5, 1.0, 2.0):
            for gamma in (0.2, 0.3, 0.5):
                r = run(a.pairs, None, tau=0.05, lam=0.0, lr=0.05, epochs=a.epochs,
                        mu=mu, gamma=gamma, select="gate", log=lambda *_: None)
                results.append(r)
                print(json.dumps(r), flush=True)
        results.sort(key=lambda r: (-r["score"],
                                    -(r["gate_proxy"] or {}).get("adversarial", {}).get("answer_rate", 0)))
        print("\nBEST:", json.dumps(results[0], indent=1))
    elif a.sweep:
        results = []
        # lr grid extended upward per the HF static-embeddings recipe: lookup
        # tables tolerate ~100x transformer learning rates (they use 0.2 SGD;
        # we warm-start with Adam, so the equivalent aggressive band is lower).
        for tau in (0.05, 0.1):
            for lam in (0.0, 0.5, 1.0):
                for lr in (5e-3, 2e-2, 5e-2):
                    r = run(a.pairs, None, tau, lam, lr, a.epochs,
                            log=lambda *_: None)
                    results.append(r)
                    print(json.dumps(r))
        results.sort(key=lambda r: -r["score"])
        print("\nBEST:", json.dumps(results[0], indent=1))
    else:
        select = a.select or ("gate" if a.mu > 0 else "acc")
        r = run(a.pairs, a.out, a.tau, a.lam, a.lr, a.epochs,
                mu=a.mu, gamma=a.gamma, select=select)
        print(json.dumps({k: v for k, v in r.items() if k != "E"}, indent=1))


if __name__ == "__main__":
    main()
