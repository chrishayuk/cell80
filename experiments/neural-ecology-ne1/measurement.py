"""NE-1a frozen measurement instrument. No simulation or data generation at import."""
import os
for _key in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS", "VECLIB_MAXIMUM_THREADS", "NUMEXPR_NUM_THREADS"):
    os.environ[_key] = "1"

import hashlib
import importlib.metadata
import json
from pathlib import Path
import sys
import time
import numpy as np
from scipy.optimize import minimize
from threadpoolctl import threadpool_limits

_threads = threadpool_limits(limits=1)
BASE = Path(__file__).resolve().parent
REPO = BASE.parent.parent
OLD = BASE.parent / "neural-ecology"
sys.path.insert(0, str(OLD))
import probe as anchor

LAMBDAS = (0.0001, 0.001, 0.01, 0.1, 1.0)
OPTIONS = dict(maxiter=2000, maxfun=20000, maxcor=20, maxls=50, gtol=1e-6, ftol=0.0)
TARGETS = {"current": 0, "lag1": -1, "lag2": -2, "next": 1}
TIMES = (0, 12500, 25000, 37500, 50000)
ARCHIVED = (1000, 13000, 25000, 38000, 50000)
ARMS = ("Frozen", "Plastic", "Yoked")
READOUTS = [(v, "G", t) for v in ("q", "full") for t in TARGETS]
READOUTS += [(v, "C", t) for v in ("q", "full") for t in ("lag1", "lag2", "next")]
READOUTS += [("q", "anchor", "next")]


def sha(path):
    return anchor.sha(path)


def write(path, value):
    def finite_json(item):
        if isinstance(item, dict):
            return {k: finite_json(v) for k, v in item.items()}
        if isinstance(item, (list, tuple)):
            return [finite_json(v) for v in item]
        if isinstance(item, float) and not np.isfinite(item):
            return None
        return item
    anchor.write_json(path, finite_json(value))


def verify_spec():
    if sys.version_info[:2] != (3, 12):
        raise ValueError("spec requires Python 3.12")
    for name, version in (("numpy", "2.4.2"), ("scipy", "1.17.0"), ("threadpoolctl", "3.6.0")):
        if importlib.metadata.version(name) != version:
            raise ValueError(f"dependency mismatch: {name}")
    seal = json.loads((BASE / "measurement-seal.json").read_text())
    for path, digest in seal["files"].items():
        if sha(REPO / path) != digest:
            raise ValueError(f"changed specification file: {path}")
    inherited = json.loads((REPO / seal["inherited"]["NE0c_implementation_seal"]).read_text())
    for path, digest in inherited["files"].items():
        if sha(REPO / path) != digest:
            raise ValueError(f"changed inherited file: {path}")
    return seal


def grammar(seed, split):
    if split not in ("fit", "validation", "test"):
        raise ValueError("invalid split")
    namespace = f"grammar/NE1-v1/{split}"
    symbols = [anchor.bounded(seed, namespace, i, 4) for i in (0, 1)]
    for t in range(2, 10257):
        r = anchor.bounded(seed, namespace, t, 10)
        symbols.append((symbols[-2] + symbols[-1] + (0 if r < 7 else r - 6)) % 4)
    return np.array(symbols, dtype=np.uint8)


def labels(symbols, target):
    offset = TARGETS[target]
    return symbols[256 + offset:10256 + offset].astype(np.int64)


def groups_ok(current):
    return bool(len(current) == 10000 and np.all(np.bincount(current, minlength=4) >= 128))


def normalizer(x):
    mu = x.mean(axis=0)
    sigma = np.sqrt(np.mean((x - mu) ** 2, axis=0))
    return dict(mu=mu, sigma=sigma, constant=sigma <= 1e-12)


def transform(x, norm):
    z = (x - norm["mu"]) / np.where(norm["constant"], 1.0, norm["sigma"])
    z[:, norm["constant"]] = 0
    return z


def inputs(x, current, family, target):
    if family in ("G", "anchor") and target == "next":
        return np.column_stack((x, np.eye(4, dtype=np.float64)[current]))
    return x


def objective(theta, x, y, lam):
    w = theta[:-4].reshape(x.shape[1], 4)
    b = theta[-4:]
    logp = anchor.probabilities(x, w, b)
    value = float(-logp[np.arange(len(y)), y].mean() + .5 * lam * np.sum(w * w))
    residual = np.exp(logp)
    residual[np.arange(len(y)), y] -= 1
    residual /= len(y)
    gradient = np.concatenate(((x.T @ residual + lam * w).ravel(), residual.sum(axis=0)))
    return value, gradient


def solve(x, y, lam):
    started = time.monotonic()
    theta = np.zeros(x.shape[1] * 4 + 4, dtype=np.float64)
    initial, _ = objective(theta, x, y, lam)
    result = minimize(objective, theta, args=(x, y, lam), method="L-BFGS-B", jac=True,
                      bounds=None, options=OPTIONS)
    final, grad = objective(result.x, x, y, lam)
    norm = float(np.max(np.abs(grad)))
    valid = bool(result.success and np.all(np.isfinite(result.x)) and np.isfinite(final)
                 and np.isfinite(norm) and norm <= 1e-6 and final <= initial + 1e-10)
    record = dict(valid=valid, solver_success=bool(result.success), status=int(result.status),
                  message=str(result.message), iterations=int(result.nit), evaluations=int(result.nfev),
                  initial_objective=initial, final_objective=final, gradient_inf=norm,
                  weight_norm=float(np.linalg.norm(result.x[:-4])),
                  intercept_norm=float(np.linalg.norm(result.x[-4:])), seconds=time.monotonic() - started)
    return result.x[:-4].reshape(x.shape[1], 4), result.x[-4:], record


def log_probabilities(x, current, w, b, family):
    if family != "C":
        return anchor.probabilities(x, w[0], b[0])
    logp = np.empty((len(x), 4), dtype=np.float64)
    for head in range(4):
        indices = current == head
        logp[indices] = anchor.probabilities(x[indices], w[head], b[head])
    return logp


def selected_index(candidates):
    if not all(c["valid"] for c in candidates):
        return None
    minimum = min(c["validation_loss"] for c in candidates)
    return max((i for i, c in enumerate(candidates) if c["validation_loss"] <= minimum + 1e-8),
               key=lambda i: candidates[i]["lambda"])


def fit_grid(xfit, yfit, cfit, xval, yval, cval, family, output):
    output = Path(output)
    output.mkdir(parents=True, exist_ok=False)
    candidates = []
    for lam in LAMBDAS:
        weights, intercepts, records = [], [], []
        for head in (range(4) if family == "C" else range(1)):
            mask = cfit == head if family == "C" else slice(None)
            w, b, record = solve(xfit[mask], yfit[mask], lam)
            weights.append(w); intercepts.append(b); records.append(record)
        w = np.array(weights); b = np.array(intercepts)
        path = output / f"lambda-{lam:g}.npz"
        np.savez(path, w=w, b=b)
        lp = log_probabilities(xval, cval, w, b, family)
        loss = float(-lp[np.arange(len(yval)), yval].mean())
        record = dict(**{"lambda": lam}, valid=bool(all(r["valid"] for r in records) and np.isfinite(loss)),
                      validation_loss=loss, heads=records, coefficient_file=path.name, coefficient_sha256=sha(path))
        candidates.append(record)
        write(output / f"lambda-{lam:g}.json", record)
        print(json.dumps({"fit": str(output), "lambda": lam, "valid": record["valid"],
                          "validation_loss": loss}), flush=True)
    selected = selected_index(candidates)
    record = dict(status="valid" if selected is not None else "invalid_fit", family=family,
                  selected_index=selected, candidates=candidates)
    write(output / "fit.json", record)
    return record


def score(logp, y):
    return dict(loss=float(-logp[np.arange(len(y)), y].mean()),
                accuracy=float((logp.argmax(axis=1) == y).mean()),
                logp_sha256=hashlib.sha256(logp.astype("<f8").tobytes()).hexdigest())
