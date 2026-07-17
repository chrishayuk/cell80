#!/usr/bin/env python3
"""CN-7.4n — the weight-noise probe (prereg §9, registered outcome-blind pre-midtrain).

Hinton–van-Camp-style noise as MEASUREMENT: perturb every trained tensor with iid Gaussian
noise at relative scale σ·std(tensor), 3 draws per σ, and watch which capabilities die first.
The readable signal is ORDERINGS within a checkpoint (shared sharpness confound cancels);
the matched-pair masked-vs-no-mask comparison (N3) is the strongest read — the two arms
differ by exactly the ~245k answer-tokens of gradient the mask withheld.

Measures per (σ, draw), NLL-domain versions of the registered capabilities (generation
probes can be added if orderings come out ambiguous — noted trim from §9's list):
  tier_a   : s1 fresh-instance answer NLL          (N1: most robust task capability)
  beyond   : s2 injected + s3 masked-value NLL     (N2/N3: the leak/memorisation signal)
  grammar  : s2 call-token NLL                     (N4: predicted most robust of all)
  fluency  : TinyStories val NLL (100-row slice)   (context line)

Run: python3 cn7_noise.py --ckpt cn7_ckpt_midtrain.pt
     python3 cn7_noise.py --ckpt cn7_ckpt_midtrain_nomask.pt
"""
from __future__ import annotations

import argparse
import json
import random
import time
from pathlib import Path

from artifact_paths import checkpoint_input, dataset_input

import torch
import torch.nn.functional as F

import cn1_model
from cn1_model import resize_embedding
from cn1_corpus import Oracle
from cn7_corpus import Enc, CELL_KIND, S2_BEYOND
from cn7_species_nll import role_rows_s1, role_rows_s2, role_rows_s3, FRESH_SEED

HERE = Path(__file__).resolve().parent
SIGMAS = [0.0, 0.005, 0.01, 0.02, 0.04, 0.08]
DRAWS = 3


def measure(base, rows, val, device):
    acc = {}
    with torch.no_grad():
        for i in range(0, len(rows), 16):
            chunk = rows[i:i + 16]
            m = max(len(ids) for _, ids, _ in chunk)
            x = torch.zeros((len(chunk), m), dtype=torch.long)
            for k, (_, ids, _) in enumerate(chunk):
                x[k, :len(ids)] = torch.tensor(ids)
            x = x.to(device)
            lg = base(x)[:, :-1]
            ce = F.cross_entropy(lg.reshape(-1, lg.shape[-1]), x[:, 1:].reshape(-1),
                                 reduction="none").reshape(len(chunk), -1).cpu()
            for k, (sp_, ids, roles) in enumerate(chunk):
                for pos in range(1, len(ids)):
                    r = roles[pos]
                    key = ("tier_a" if (sp_.startswith("s1") and r == "answer") else
                           "beyond" if r in ("injected", "value_masked") else
                           "grammar" if r == "call" else None)
                    if key:
                        a = acc.setdefault(key, [0.0, 0])
                        a[0] += float(ce[k, pos - 1]); a[1] += 1
        vt, vn = 0.0, 0
        for i in range(0, len(val), 8):
            chunk = val[i:i + 8]
            m = max(len(r["ids"]) for r in chunk)
            x = torch.zeros((len(chunk), m), dtype=torch.long)
            am = torch.zeros((len(chunk), m))
            for k, r in enumerate(chunk):
                x[k, :len(r["ids"])] = torch.tensor(r["ids"])
                am[k, :len(r["ids"])] = 1
            x = x.to(device)
            lg = base(x)[:, :-1]
            ce = F.cross_entropy(lg.reshape(-1, lg.shape[-1]), x[:, 1:].reshape(-1),
                                 reduction="none").reshape(len(chunk), -1).cpu()
            vt += float((ce * am[:, 1:]).sum()); vn += int(am[:, 1:].sum())
    out = {k: round(v / n, 4) for k, (v, n) in acc.items()}
    out["fluency"] = round(vt / max(1, vn), 4)
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ckpt", required=True)
    ap.add_argument("--device", default="cpu")
    args = ap.parse_args()
    t0 = time.time()
    device = args.device

    tokmap = json.load(open(HERE / "cn7_token_map.json"))
    enc = Enc(tokmap["cells"])
    lib = {json.loads(l)["name"]: json.loads(l) for l in (HERE / "cn1_library.jsonl").read_text().splitlines() if l.strip()}
    held = {h["name"] for h in json.loads((HERE / "cn1_axis_a_heldout.json").read_text())["held_out_cells"]}
    train_cells = sorted(n for n, r in lib.items() if r["arity"] >= 1 and n not in held)
    oracle = Oracle(sorted(set(list(CELL_KIND) + [c for c, *_ in S2_BEYOND] + train_cells)))
    rng = random.Random(FRESH_SEED)
    r1, _ = role_rows_s1(rng, oracle, enc, 200)
    r2, _ = role_rows_s2(rng, oracle, enc, 100)
    r3, _ = role_rows_s3(rng, oracle, enc, lib, train_cells, 1)
    rows = r1 + r2 + r3
    corpus = [json.loads(l) for l in dataset_input("cn7_corpus_train.jsonl").read_text().splitlines() if l.strip()]
    val = [r for r in corpus if r["species"] == "s4"][-100:]
    print(f"  probe rows {len(rows)} | val rows {len(val)}", flush=True)

    from tiny_model_v11.loader import load_from_artifacts
    base, cfg = load_from_artifacts(str(cn1_model.TINY_MODEL / "model" / "v11"), device="cpu")
    ck = torch.load(checkpoint_input(args.ckpt), map_location="cpu")
    resize_embedding(base, ck["vocab"])
    base.load_state_dict(ck["state"])
    clean = {k: v.clone() for k, v in base.state_dict().items()}
    stds = {k: float(v.float().std()) for k, v in clean.items() if v.dtype.is_floating_point}
    base = base.to(device).eval()

    results = []
    for sigma in SIGMAS:
        for draw in range(1 if sigma == 0 else DRAWS):
            g = torch.Generator().manual_seed(4000 + draw)
            sd = {}
            for k, v in clean.items():
                if sigma > 0 and v.dtype.is_floating_point and stds.get(k, 0) > 0:
                    sd[k] = v + sigma * stds[k] * torch.randn(v.shape, generator=g)
                else:
                    sd[k] = v
            base.load_state_dict({k: v.to(device) for k, v in sd.items()})
            m = measure(base, rows, val, device)
            m.update({"sigma": sigma, "draw": draw})
            results.append(m)
            print(f"  σ={sigma:<6} draw {draw}: tier_a {m['tier_a']:.3f}  beyond {m['beyond']:.3f}  "
                  f"grammar {m['grammar']:.4f}  fluency {m['fluency']:.3f}  ({time.time()-t0:.0f}s)", flush=True)

    path = HERE / f"cn7_noise_{Path(args.ckpt).stem}.json"
    path.write_text(json.dumps({"ckpt": args.ckpt, "sigmas": SIGMAS, "draws": DRAWS,
                                "results": results}, indent=1))
    print(f"wrote {path.name} ({time.time()-t0:.0f}s)")


if __name__ == "__main__":
    main()
