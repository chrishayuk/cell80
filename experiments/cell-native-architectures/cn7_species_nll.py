#!/usr/bin/env python3
"""CN-7 per-species / per-token-role NLL — the rule-acquisition instrument.

The blend loss (and even per-species loss) is contaminated by template-scaffold credit: any
model that has seen a hundred templates crashes scaffold NLL without learning arithmetic.
This decomposes by TOKEN ROLE on FRESH instances (generator re-run with an unused seed, so
nothing here was a training row — answer NLL falling toward the generator's conditional
entropy, which is ~0 bits for deterministic answers, is rule acquisition by definition):

  s1: scaffold vs ANSWER (per-op breakdown too — the P-a precursor)
  s2: story vs CALL (delimiters+cell+args) vs INJECTED (beyond-tier answers: never trained,
      zero loss by design — their NLL is a free mask-leak reading, with s3's in-tier values
      as the "does it put mass on numerals at all" control)
  s3: grammar vs VALUE-in-tier vs VALUE-masked (the P-b precursor; grade any over/under-budget
      claim on THIS, not on s1 drill, which was always going to saturate first)

Tokenization identity is asserted, not assumed: each s1 row's role-split encoding must equal
its training-style single-part encoding token-for-token, else the row is skipped and counted.

Run: python3 cn7_species_nll.py --ckpt cn7_ckpt_midtrain.pt   (or --raw for the R0 floor)
"""
from __future__ import annotations

import argparse
import json
import random
import time
from pathlib import Path

from artifact_paths import checkpoint_input

import torch
import torch.nn.functional as F

import cn1_model
from cn1_model import resize_embedding
from cn1_corpus import Oracle
from cn7_corpus import (CALL_ID, CLOSE_ID, CELL_FIRST_ID, Enc, s1_item, s2_item, s3_item,
                        CELL_KIND, S2_BEYOND, tier_a_instance)

HERE = Path(__file__).resolve().parent
FRESH_SEED = 981  # never used by any corpus build


def role_rows_s1(rng, oracle, enc, n):
    import re
    rows, skipped = [], 0
    for _ in range(n):
        parts, meta = s1_item(rng, oracle)
        text = parts[0][0]
        nums = re.findall(r"\d+|even|odd", text)
        ans = nums[-1] if meta["op"] != "cmp" else None
        if ans is None or ans not in text:
            skipped += 1
            continue
        pre, _, post = text.rpartition(ans)
        rparts = [(pre, "scaffold"), (ans, "answer"), (post, "scaffold")]
        _, ids_r, roles = enc_roles(enc, rparts)
        _, ids_t, _ = enc_roles(enc, [(text, "x")])
        if ids_r != ids_t:  # boundary tokenization drifted — measurement would be off-protocol
            skipped += 1
            continue
        rows.append(("s1:" + meta["op"], ids_r, roles))
    return rows, skipped


def role_rows_s2(rng, oracle, enc, n):
    rows = []
    for _ in range(n):
        parts, meta = s2_item(rng, oracle, enc)
        rparts = []
        for t, fl in parts:
            if fl == 0:
                rparts.append((t, "injected"))
            elif "<call>" in t:
                rparts.append((t, "call"))
            else:
                rparts.append((t, "story"))
        _, ids, roles = enc_roles(enc, rparts)
        rows.append(("s2", ids, roles))
    return rows, 0


def role_rows_s3(rng, oracle, enc, lib, cells, per_cell):
    rows = []
    for name in cells:
        made = 0
        for _ in range(per_cell * 3):
            if made >= per_cell:
                break
            item = s3_item(rng, oracle, lib, name)
            if not item:
                continue
            parts, meta = item
            rparts = []
            for t, fl in parts:
                if t.startswith(" ") and t[1:].isdigit() and "=" not in t and ";" not in t and "call" not in t:
                    rparts.append((t, "value_in_tier" if fl == 1 else "value_masked"))
                else:
                    rparts.append((t, "grammar"))
            _, ids, roles = enc_roles(enc, rparts)
            rows.append(("s3", ids, roles))
            made += 1
    return rows, 0


def enc_roles(enc, parts):
    text, ids, roles = [], [], []
    for t, role in parts:
        text.append(t)
        import re as _re
        for seg in filter(None, _re.split(r"(<call>|</call>|⟨[a-z0-9_]+⟩)", t)):
            si = enc.seg_ids(seg)
            ids.extend(si)
            roles.extend([role] * len(si))
    return "".join(text), ids, roles


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ckpt", default=None)
    ap.add_argument("--raw", action="store_true")
    ap.add_argument("--n-s1", type=int, default=800)
    ap.add_argument("--n-s2", type=int, default=400)
    ap.add_argument("--s3-per-cell", type=int, default=2)
    ap.add_argument("--device", default=None)
    args = ap.parse_args()
    device = args.device or ("mps" if torch.backends.mps.is_available() else "cpu")
    t0 = time.time()

    tokmap = json.load(open(HERE / "cn7_token_map.json"))
    enc = Enc(tokmap["cells"])
    lib = {json.loads(l)["name"]: json.loads(l) for l in (HERE / "cn1_library.jsonl").read_text().splitlines() if l.strip()}
    held = {h["name"] for h in json.loads((HERE / "cn1_axis_a_heldout.json").read_text())["held_out_cells"]}
    train_cells = sorted(n for n, r in lib.items() if r["arity"] >= 1 and n not in held)
    oracle = Oracle(sorted(set(list(CELL_KIND) + [c for c, *_ in S2_BEYOND] + train_cells)))
    rng = random.Random(FRESH_SEED)

    from tiny_model_v11.loader import load_from_artifacts
    base, cfg = load_from_artifacts(str(cn1_model.TINY_MODEL / "model" / "v11"), device="cpu")
    if args.raw:
        tag = "r0_raw_v11"
        resize_embedding(base, tokmap["vocab"])  # appended rows are xavier noise: the R0 floor
    else:
        ck = torch.load(checkpoint_input(args.ckpt), map_location="cpu")
        resize_embedding(base, ck["vocab"])
        base.load_state_dict(ck["state"])
        tag = Path(args.ckpt).stem
    base = base.to(device).eval()

    rows, sk1 = role_rows_s1(rng, oracle, enc, args.n_s1)
    r2, _ = role_rows_s2(rng, oracle, enc, args.n_s2)
    r3, _ = role_rows_s3(rng, oracle, enc, lib, train_cells, args.s3_per_cell)
    rows += r2 + r3
    print(f"  fresh rows: {len(rows)} (s1 skipped for tokenization drift or cmp: {sk1})", flush=True)

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
                    key = (sp_.split(":")[0], roles[pos])
                    a = acc.setdefault(key, [0.0, 0])
                    a[0] += float(ce[k, pos - 1]); a[1] += 1
                    if sp_.startswith("s1:") and roles[pos] == "answer":
                        a2 = acc.setdefault((sp_, "answer"), [0.0, 0])
                        a2[0] += float(ce[k, pos - 1]); a2[1] += 1

    out = {"tag": tag, "fresh_seed": FRESH_SEED,
           "nll": {f"{s}|{r}": {"nll": round(v / n, 4), "tokens": n}
                   for (s, r), (v, n) in sorted(acc.items()) if n}}
    print(json.dumps(out["nll"], indent=1))
    path = HERE / f"cn7_species_nll_{tag}.json"
    path.write_text(json.dumps(out, indent=1))
    print(f"wrote {path.name} ({time.time()-t0:.0f}s)")


if __name__ == "__main__":
    main()
