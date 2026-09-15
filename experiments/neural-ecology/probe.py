"""NE-0's fixed-budget measurement instrument. No C-based fitting or selection."""
import os
for _key in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS", "VECLIB_MAXIMUM_THREADS", "NUMEXPR_NUM_THREADS"):
    os.environ[_key] = "1"

import argparse
import hashlib
import json
import math
from pathlib import Path
import platform
import sys
import time
import numpy as np
from threadpoolctl import threadpool_info, threadpool_limits
_thread_limit = threadpool_limits(limits=1)

N, WARM, SAMPLES = 1024, 256, 10000
ARMS = ("Frozen", "Plastic", "Yoked", "Memoryless")

def sha(path):
    h = hashlib.sha256()
    with Path(path).open("rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()

def write_json(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, allow_nan=False) + "\n")
    temporary.replace(path)

def arrays(root, phase, arm):
    root = Path(root)
    symbols = np.fromfile(root / f"datasets/{phase}-symbols.u8", dtype=np.uint8)
    if symbols.shape != (WARM + SAMPLES + 1,) or np.any(symbols > 3):
        raise ValueError("incorrect stream length or alphabet")
    current = symbols[WARM:WARM+SAMPLES].astype(np.int64)
    target = symbols[WARM+1:WARM+SAMPLES+1].astype(np.int64)
    x = np.zeros((SAMPLES, N + 4), dtype=np.float64)
    if arm != "Memoryless":
        q = np.fromfile(root / f"datasets/{phase}-{arm}-q.u16", dtype="<u2")
        if q.shape != (SAMPLES*N,) or np.any(q > 255):
            raise ValueError("incorrect trace array")
        x[:, :N] = q.reshape(SAMPLES, N) / (255.0 * 32.0)
    x[np.arange(SAMPLES), N + current] = 1.0
    return x, target, current, symbols

def probabilities(x, w, b):
    logits = x @ w + b
    shifted = logits - logits.max(axis=1, keepdims=True)
    logp = shifted - np.log(np.exp(shifted).sum(axis=1, keepdims=True))
    return logp

def objective_gradient(x, y, w, b):
    logp = probabilities(x, w, b)
    objective = -logp[np.arange(len(y)), y].mean() + .005 * np.sum(w*w)
    residual = np.exp(logp)
    residual[np.arange(len(y)), y] -= 1.0
    residual /= len(y)
    return float(objective), x.T @ residual + .01*w, residual.sum(axis=0)

def fit_array(x, y):
    w = np.zeros((x.shape[1], 4), dtype=np.float64)
    b = np.zeros(4, dtype=np.float64)
    initial = objective_gradient(x, y, w, b)[0]
    for _ in range(2000):
        _, gw, gb = objective_gradient(x, y, w, b)
        w -= .5 * gw
        b -= .5 * gb
    final, gw, gb = objective_gradient(x, y, w, b)
    norm = float(np.sqrt(np.sum(gw*gw) + np.sum(gb*gb)))
    if not np.isfinite(final) or not np.isfinite(norm) or final > initial+1e-10:
        raise ValueError("invalid fixed-budget probe optimization")
    return w, b, {"initial_objective": initial, "final_objective": final,
                  "final_gradient_norm": norm, "steps": 2000, "learning_rate": .5, "l2": .01}

def fit(root):
    root = Path(root)
    manifest = root / "checkpoints/probes.json"
    if manifest.exists():
        raise ValueError("refusing to overwrite frozen probes")
    if (root / "datasets/C-symbols.u8").exists():
        raise ValueError("C must not be generated before probes are frozen")
    records = {}
    for arm in ARMS:
        started = time.monotonic()
        x, y, _, _ = arrays(root, "B", arm)
        w, b, record = fit_array(x, y)
        path = root / f"checkpoints/probe-{arm}.npz"
        np.savez(path, w=w, b=b)
        record.update(seconds=time.monotonic()-started, coefficient_file=path.name,
                      coefficient_sha256=sha(path), B_symbols_sha256=sha(root / "datasets/B-symbols.u8"),
                      B_features_sha256=None if arm == "Memoryless" else sha(root / f"datasets/B-{arm}-q.u16"))
        records[arm] = record
        print(json.dumps({"fit_arm": arm, **record}), flush=True)
    write_json(manifest, {"schema": "NE0-probes-v1", "arms": records,
                         "python": sys.version, "numpy": np.__version__, "platform": platform.platform(),
                         "numerical_libraries": threadpool_info(),
                         "thread_environment": {k: os.environ[k] for k in os.environ if k.endswith("NUM_THREADS") or k == "VECLIB_MAXIMUM_THREADS"},
                         "probe_source_sha256": sha(__file__)})

def bounded(seed, namespace, counter, m):
    limit = (1 << 64) // m * m
    attempt = 0
    while True:
        key = f"NE0-v1/{seed}/{namespace}/{counter}/{attempt}".encode()
        word = int.from_bytes(hashlib.sha256(key).digest()[:8], "little")
        if word < limit:
            return word % m
        attempt += 1

def conditional_permutation(seed, arm, current):
    rows = np.arange(len(current), dtype=np.int64)
    for symbol in range(4):
        ids = np.flatnonzero(current == symbol)
        shuffled = ids.copy()
        for counter, i in enumerate(range(len(ids)-1, 0, -1)):
            j = bounded(seed, f"shuffle/{arm}/{symbol}", counter, i+1)
            shuffled[i], shuffled[j] = shuffled[j], shuffled[i]
        rows[ids] = shuffled
    return rows

def score(root):
    root = Path(root)
    manifest = json.loads((root / "checkpoints/probes.json").read_text())
    if manifest["probe_source_sha256"] != sha(__file__):
        raise ValueError("probe implementation changed after fitting")
    phases = {p: json.loads((root / f"logs/{p}-summary.json").read_text()) for p in "ABC"}
    seed = phases["A"]["seed"]
    result = {"seed": seed, "development": phases["A"]["development"], "arms": {}}
    for arm in ARMS:
        record = manifest["arms"][arm]
        path = root / "checkpoints" / record["coefficient_file"]
        if sha(path) != record["coefficient_sha256"] or sha(root / "datasets/B-symbols.u8") != record["B_symbols_sha256"]:
            raise ValueError("frozen coefficients or B changed")
        if arm != "Memoryless" and sha(root / f"datasets/B-{arm}-q.u16") != record["B_features_sha256"]:
            raise ValueError("B features changed")
        with np.load(path, allow_pickle=False) as coefficients:
            w, b = coefficients["w"], coefficients["b"]
        x, y, current, symbols = arrays(root, "C", arm)
        logp = probabilities(x, w, b)
        loss = float(-logp[np.arange(SAMPLES), y].mean())
        accuracy = float((logp.argmax(axis=1) == y).mean())
        if not math.isfinite(loss):
            raise ValueError("nonfinite C loss")
        row = {"loss": loss, "accuracy": accuracy,
               "logp_sha256": hashlib.sha256(logp.astype("<f8").tobytes()).hexdigest()}
        if arm != "Memoryless":
            rows = conditional_permutation(seed, arm, current)
            np.save(root / f"datasets/C-{arm}-shuffle.npy", rows)
            shuffled = x.copy()
            shuffled[:, :N] = x[rows, :N]
            shuffled_logp = probabilities(shuffled, w, b)
            row["shuffled_loss"] = float(-shuffled_logp[np.arange(SAMPLES), y].mean())
        result["arms"][arm] = row
    expected = (symbols[WARM-1:WARM+SAMPLES-1].astype(np.int64) + symbols[WARM:WARM+SAMPLES]) % 4
    result["oracle_loss"] = float(-np.log(np.where(y == expected, .7, .1)).mean())
    result["oracle_accuracy"] = float((y == expected).mean())
    result["memoryless_reference"] = math.log(4)
    losses = {a: result["arms"][a]["loss"] for a in ARMS}
    result["d_F"] = losses["Frozen"] - losses["Plastic"]
    result["d_Y"] = losses["Yoked"] - losses["Plastic"]
    result["d_M"] = math.log(4) - losses["Plastic"]
    audits = []
    for sign in range(2):
        p = phases["A"]["arms"]["Plastic"]["movement"][sign]
        yoke = phases["A"]["arms"]["Yoked"]["movement"][sign]
        positive = min(p["requested"], p["realised"], yoke["requested"], yoke["realised"]) > 0
        movement_gap = abs(p["realised"]-yoke["realised"])/max(p["realised"], yoke["realised"], 1)
        clipping_gap = abs(p["clipped"]/max(p["requested"], 1)-yoke["clipped"]/max(yoke["requested"], 1))
        audits.append({"sign": 1 if sign == 0 else -1, "realised_relative_gap": movement_gap,
                       "clipping_fraction_gap": clipping_gap, "pass": positive and movement_gap <= .05 and clipping_gap <= .05})
    result["movement_audit"] = audits
    result["gates_per_seed"] = {
        "G0_execution": all(phases[p]["backend_agreement"] and phases[p]["metal_replay"] for p in "ABC")
            and phases["A"]["ticks"] == 50000 and all(phases[p]["ticks"] == WARM+SAMPLES for p in "BC"),
        "G1_activity": all(phases[p]["arms"][a]["activity"]["pass"] for p in "ABC" for a in ARMS[:3])
            and all(phases["A"]["arms"][a]["endpoint_pass"] for a in ARMS[:3]),
        "G2_adaptation": phases["A"]["arms"]["Plastic"]["adaptation_pass"],
        "G4_movement_audit": all(a["pass"] for a in audits),
        "G5_persistence": json.loads((root / "logs/persistence.json").read_text())["pass"]
            if (root / "logs/persistence.json").exists() else None,
    }
    write_json(root / "logs/result.json", result)
    print(json.dumps(result), flush=True)
    return result

def contrast(values):
    n = len(values)
    k = sum(v > 0 for v in values)
    p = sum(math.comb(n, j) for j in range(k, n+1)) / 2**n
    mean = float(np.mean(values))
    return {"n": n, "values": values, "positive_seeds": k, "mean": mean,
            "median": float(np.median(values)), "sample_sd": float(np.std(values, ddof=1)) if n > 1 else None,
            "p_raw": p, "p_bonferroni": min(1, 3*p),
            "pass": n == 10 and 3*p <= .05 and mean >= .020}

def report(roots, path):
    results = [json.loads((Path(r) / "logs/result.json").read_text()) for r in roots]
    development = all(r["development"] for r in results)
    if not development and any(r["development"] for r in results):
        raise ValueError("cannot mix development and evaluation")
    if len({r["seed"] for r in results}) != len(results):
        raise ValueError("duplicate replicates")
    results.sort(key=lambda r: r["seed"])
    complete = not development and [r["seed"] for r in results] == list(range(2001, 2011))
    comparisons = {key: contrast([r[key] for r in results]) for key in ("d_F", "d_Y", "d_M")}
    gates = {
        "G0": complete and all(r["gates_per_seed"]["G0_execution"] for r in results),
        "G1": all(r["gates_per_seed"]["G1_activity"] for r in results),
        "G2": all(r["gates_per_seed"]["G2_adaptation"] for r in results),
        "G3": comparisons["d_F"]["pass"],
        "G4": comparisons["d_Y"]["pass"] and all(r["gates_per_seed"]["G4_movement_audit"] for r in results),
        "G5": all(r["gates_per_seed"]["G5_persistence"] is True for r in results),
        "G6": comparisons["d_M"]["pass"],
    }
    verdict = "DEVELOPMENT_ONLY" if development else "INCOMPLETE" if not complete else "INVALID" if not gates["G0"] or not gates["G5"] else "PASS" if all(gates.values()) else "CRITERIA_NOT_MET"
    document = {"verdict": verdict, "development": development, "complete": complete,
                "gates": gates, "contrasts": comparisons, "seeds": results}
    write_json(path, document)
    print(json.dumps({k: v for k, v in document.items() if k != "seeds"}), flush=True)

def main():
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    for command in ("fit", "score"):
        sub.add_parser(command).add_argument("root", type=Path)
    p = sub.add_parser("report")
    p.add_argument("output", type=Path)
    p.add_argument("roots", nargs="+", type=Path)
    args = parser.parse_args()
    if args.command == "fit": fit(args.root)
    elif args.command == "score": score(args.root)
    else: report(args.roots, args.output)

if __name__ == "__main__":
    main()
