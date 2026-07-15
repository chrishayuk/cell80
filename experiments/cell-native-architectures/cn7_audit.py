#!/usr/bin/env python3
"""CN-7.1 audits — the gate before any GPU time (prereg §4, CN-7.1).

Two properties, checked by a code path that shares NOTHING with the generator except the
registered definitions (the SP tokenizer/Enc, the tier function's SPEC — re-tested here against
hand-written vectors — and the cell oracle):

  1. SIGNATURE AUDIT: every numeric claim in the corpus is re-derived FROM THE ROW'S TEXT by
     independent regex parsers (one per template family) and re-executed against its cell.
     Target: 100% of rows parsed, 100% of claims signed. Any mismatch is a pipeline bug.
  2. MASK AUDIT: the expected loss array is rebuilt from the parsed text (S2 injected results
     masked; S3 answers masked unless tier_a_instance passes; S1/S4 all-loss) and compared
     token-for-token against the stored loss. Plus the global §3.3 property: no beyond-tier
     answer token anywhere carries loss. Plus: no held-out cell appears anywhere (token id or
     S3 descriptor).

Exit code 0 = both audits clean (the 7.1 gate opens). Anything else = do not train.

Run: python3 cn7_audit.py
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

from cn1_corpus import Oracle, describe
from cn7_corpus import CALL_ID, CLOSE_ID, CELL_FIRST_ID, Enc, tier_a_instance

HERE = Path(__file__).resolve().parent

# ---- tier-function spec vectors (hand-written; guards the shared definition) -----------
TIER_VECTORS = [
    ("add_sat", [99, 99], True), ("add_sat", [100, 5], False),
    ("sub_sat", [99, 0], True), ("sub_sat", [500, 100], False),
    ("mul_sat", [12, 12], True), ("mul_sat", [13, 13], False),
    ("mul_sat", [9, 99], True), ("mul_sat", [10, 13], False),
    ("safe_mod", [99, 12], True), ("safe_mod", [99, 13], False),
    ("is_lt", [999, 998], True), ("is_lt", [1000, 5], False),
    ("clamp", [999, 0, 999], True),
    ("magic_constants", [3], False), ("safe_div", [10, 2], False),  # not whitelisted
]

S1_PATTERNS = [
    ("add", re.compile(r"^(\d+) \+ (\d+) = (\d+)$")),
    ("add", re.compile(r"^(\w+) had (\d+) (\w+)\. (\w+) gave \1 (\d+) more\. Now \1 has (\d+) \3\.$")),
    ("sub", re.compile(r"^(\d+) - (\d+) = (\d+)$")),
    ("sub", re.compile(r"^(\w+) had (\d+) (\w+) and lost (\d+)\. \1 has (\d+) \3 left\.$")),
    ("mul", re.compile(r"^(\d+) x (\d+) = (\d+)$")),
    ("mul", re.compile(r"^There were (\d+) bags with (\d+) (\w+) in each bag\. That made (\d+) \3 in all\.$")),
    ("mod", re.compile(r"^(\d+) mod (\d+) = (\d+)$")),
    ("mod", re.compile(r"^(\d+) (\w+) were put in rows of (\d+)\. There were (\d+) \2 left over\.$")),
    ("cmp", re.compile(r"^(\d+) < (\d+)$")),
    ("cmp", re.compile(r"^(\w+) found (\d+) (\w+) and (\w+) found (\d+) \3\. (\w+) found more\.$")),
    ("parity", re.compile(r"^(\d+) is (even|odd)$")),
    ("parity", re.compile(r"^(\w+) counted (\d+) (\w+)\. (\d+) is an (even|odd) number\.$")),
    ("min3", re.compile(r"^smallest of (\d+), (\d+), (\d+) is (\d+)$")),
    ("min3", re.compile(r"^Three piles had (\d+), (\d+) and (\d+) (\w+)\. The smallest pile had (\d+)\.$")),
    ("succ", re.compile(r"^after (\d+) comes (\d+)$")),
    ("succ", re.compile(r"^(\w+) counted (\d+), then (\d+)\.$")),
]

S2_RE = re.compile(r"^(?:(\w+) picked (\d+) berries and then (\d+) more, so \1 had (\d+) berries\. )?"
                   r"(.*?)<call> ⟨([a-z0-9_]+)⟩ ((?:\d+ )*\d+) </call> (\d+)(\D*)$")


def sign_s1(kind, m, oracle):
    g = m.groups()
    nums = [int(x) for x in g if x is not None and x.isdigit()]
    if kind == "add":
        a, b, r = nums
        return oracle.run("add_sat", [a, b])["result"] == r
    if kind == "sub":
        a, b, r = nums
        return oracle.run("sub_sat", [a, b])["result"] == r
    if kind == "mul":
        a, b, r = nums
        return oracle.run("mul_sat", [a, b])["result"] == r
    if kind == "mod":
        a, b, r = nums[0], nums[1], nums[2]
        return oracle.run("safe_mod", [a, b])["result"] == r
    if kind == "cmp":
        if len(m.groups()) == 2:  # canonical "small < big"
            return oracle.run("is_lt", [nums[0], nums[1]])["result"] == 1
        a, b = nums  # narrative: two counts; winner named
        names = [x for x in m.groups() if x is not None and not x.isdigit()]
        n1, _, n2, w = names[0], names[1], names[2], names[3]
        winner = n1 if a > b else n2
        return w == winner and a != b
    if kind == "parity":
        n = nums[0]
        word = [x for x in m.groups() if x in ("even", "odd")][0]
        want = 1 if word == "even" else 0
        ok = oracle.run("is_even", [n])["result"] == want
        return ok and (len(nums) == 1 or nums[0] == nums[1])
    if kind == "min3":
        xs, r = nums[:3], nums[3]
        return oracle.run("min3", xs)["result"] == r
    if kind == "succ":
        a, r = nums[-2], nums[-1]
        return oracle.run("add_sat", [a, 1])["result"] == r
    return False


def main():
    for cell, args, want in TIER_VECTORS:
        got = tier_a_instance(cell, args)
        assert got == want, f"tier spec vector failed: {cell}{args} -> {got}, want {want}"
    print(f"tier-function spec vectors: {len(TIER_VECTORS)}/{len(TIER_VECTORS)} pass")

    tokmap = json.load(open(HERE / "cn7_token_map.json"))
    cell_ids = tokmap["cells"]
    id_to_cell = {v: k for k, v in cell_ids.items()}
    lib = {json.loads(l)["name"]: json.loads(l) for l in (HERE / "cn1_library.jsonl").read_text().splitlines() if l.strip()}
    held = {h["name"] for h in json.loads((HERE / "cn1_axis_a_heldout.json").read_text())["held_out_cells"]}
    desc_to_cell = {describe(n, r["pack"]): n for n, r in lib.items()}
    enc = Enc(cell_ids)

    rows = [json.loads(l) for l in (HERE / "cn7_corpus_train.jsonl").read_text().splitlines() if l.strip()]
    need = sorted({"add_sat", "sub_sat", "mul_sat", "safe_mod", "is_lt", "is_even", "min3"}
                  | {n for n in lib if lib[n]["arity"] >= 1 and n not in held})
    oracle = Oracle(need)

    bad_sig, bad_mask, bad_held = [], [], []
    n_claims = n_answers_masked = 0

    for k, r in enumerate(rows):
        sp_, text, ids, loss = r["species"], r["text"], r["ids"], r["loss"]

        # held-out exclusion: no held cell token id, no held descriptor
        for i in ids:
            if i >= CELL_FIRST_ID and id_to_cell.get(i) in held:
                bad_held.append((k, id_to_cell[i]))
        if sp_ == "s4":
            if any(i >= CALL_ID for i in ids):
                bad_mask.append((k, "special id in replay"))
            if not all(loss):
                bad_mask.append((k, "replay not full-loss"))
            continue

        if sp_ == "s1":
            for kind, pat in S1_PATTERNS:
                m = pat.match(text)
                if m:
                    n_claims += 1
                    if not sign_s1(kind, m, oracle):
                        bad_sig.append((k, "s1", text[:60]))
                    break
            else:
                bad_sig.append((k, "s1-unparsed", text[:60]))
            if not all(loss):
                bad_mask.append((k, "s1 not full-loss"))
            continue

        if sp_ == "s2":
            m = S2_RE.match(text)
            if not m:
                bad_sig.append((k, "s2-unparsed", text[:60]))
                continue
            warm_a, warm_b, warm_r = m.group(2), m.group(3), m.group(4)
            story, cell, argstr, res, tail = m.group(5), m.group(6), m.group(7), int(m.group(8)), m.group(9)
            args = [int(x) for x in argstr.split()]
            n_claims += 1
            if oracle.run(cell, args)["result"] != res:
                bad_sig.append((k, "s2-result", text[:60]))
            if warm_a is not None:
                n_claims += 1
                if oracle.run("add_sat", [int(warm_a), int(warm_b)])["result"] != int(warm_r):
                    bad_sig.append((k, "s2-warmup", text[:60]))
            if re.search(r"\d", tail):
                bad_mask.append((k, "digit in s2 tail"))
            # expected mask: rebuild parts
            parts = []
            if warm_a is not None:
                parts.append((text[:m.start(5)], 1))
            parts.append((story, 1))
            parts.append((f"<call> ⟨{cell}⟩ {argstr} </call> ", 1))
            parts.append((str(res), 0))
            parts.append((tail, 1))
            _, e_ids, e_loss = enc.encode(parts)
            if e_ids != ids or e_loss != loss:
                bad_mask.append((k, "s2 mask/ids mismatch"))
            n_answers_masked += 1
            continue

        if sp_ == "s3":
            m = re.match(r"^(.*?) <call>(.*?) </call>$", text)
            if not m or m.group(1) not in desc_to_cell:
                bad_sig.append((k, "s3-unparsed", text[:60]))
                continue
            cell = desc_to_cell[m.group(1)]
            if cell in held:
                bad_held.append((k, f"s3 target {cell}"))
            pairs = []
            for chunk in m.group(2).split(" ;"):
                nums = re.findall(r"\d+", chunk)
                pairs.append(([int(x) for x in nums[:-1]], int(nums[-1])))
            parts = [(f"{m.group(1)} <call>", 1)]
            for i, (a, o) in enumerate(pairs):
                n_claims += 1
                if oracle.run(cell, a)["result"] != o:
                    bad_sig.append((k, f"s3-pair {cell}", text[:60]))
                in_tier = tier_a_instance(cell, a)
                if not in_tier:
                    n_answers_masked += 1
                sep = " ;" if i < len(pairs) - 1 else " </call>"
                parts.append((f" {' '.join(map(str, a))} =", 1))
                parts.append((f" {o}", 1 if in_tier else 0))
                parts.append((sep, 1))
            _, e_ids, e_loss = enc.encode(parts)
            if e_ids != ids or e_loss != loss:
                bad_mask.append((k, "s3 mask/ids mismatch"))
            continue

    print(f"rows audited: {len(rows)} | claims re-executed: {n_claims} | masked answer spans: {n_answers_masked}")
    ok = True
    for label, bad in (("SIGNATURE", bad_sig), ("MASK", bad_mask), ("HELD-OUT", bad_held)):
        if bad:
            ok = False
            print(f"{label} AUDIT FAIL — {len(bad)} problems, first 5: {bad[:5]}")
        else:
            print(f"{label} audit: clean")
    if not ok:
        sys.exit(1)
    print("GATE OPEN: both audits clean.")


if __name__ == "__main__":
    main()
