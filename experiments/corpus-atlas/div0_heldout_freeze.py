#!/usr/bin/env python3
"""DIV-0 §8 held-out confirmation set — FREEZE generator.

The forking-paths protection (corpus-atlas-DRAFT.md §8) requires the
held-out prompts to be committed and hashed BEFORE any distance scoring
touches them. This script composes them BLIND:

  - the only corpus contact is exact-membership / substring checking,
    needed for the class labels to be true (a "fresh operands" row must
    actually be absent; a "trained row" must actually be present). No
    distance measure is ever evaluated here.
  - deterministic: seed 91 (distinct from the seed-90 eval-set family).
  - dev-battery texts (DIV-0's D-B probes) are excluded from sampling.

Four classes, 10 items each, mirroring the dev battery's pinned ordering
  trained_row < fresh_operands < variant_phrasing < off_register
which the gating measure — selected on the dev battery ONLY — must
reproduce here, once, after selection. A failed confirmation is a result
about the instrument, not a license to reselect.

Run: python3 div0_heldout_freeze.py   (writes div0_heldout.json; the
file's sha256 is recorded in the doc and the file is committed same-day)
"""

import hashlib
import json
import math
import random
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
CORPUS = HERE.parent / "cell-native-architectures" / "cn7_corpus_train.jsonl"
SEED = 91

DEV_PROBES = [  # DIV-0 results.json probes — excluded from class 1
    "Tim picked 14 berries and then 7 more, so Tim had 21 berries. 534 sweets were shared fairly between 16 children. The sharing machine said each child gets <call> ⟨safe_div⟩ 534 16 </call> 33 sweets. The children smiled.",
    "243 marbles were shared fairly between 7 friends. The sharing machine said each friend gets <call> ⟨safe_div⟩ 243 7 </call> 34 marbles. The friends smiled.",
    "12345 sweets were shared fairly between 16 children. The sharing machine said each child gets <call> ⟨safe_div⟩ 12345 16 </call> 771 sweets. The children smiled.",
    "What is 25 multiplied by 32? <call> ⟨mul_sat⟩ 25 32 </call> 800",
    "Amy had 47 flowers. Anna gave Amy 38 more. Now Amy has 85 flowers.",
    "Amy had forty-seven flowers. Anna gave Amy thirty-eight more. Now Amy has eighty-five flowers.",
]

TRAINED_NAMES = ["Sam", "Tim", "Sue", "Anna", "Max"]
NOVEL_NAMES = ["Nora", "Felix", "Priya", "Oscar", "Wren"]

C = "<call>"
E = "</call>"


def warm(rng, name):
    p, q = rng.randint(3, 19), rng.randint(3, 19)
    return f"{name} picked {p} berries and then {q} more, so {name} had {p + q} berries."


def s2_rows(rng, names, obj):
    """The six S2 frame templates, verbatim wording, parameterized fillers."""
    n = lambda: rng.choice(names)
    a, b = rng.randint(100, 999), rng.randint(100, 999)
    c, d = rng.randint(100, 999), rng.randint(11, 29)
    e, f = rng.randint(30, 99), rng.randint(11, 59)
    g, m = rng.randint(100, 999), rng.choice([10, 25, 50])
    h, i = rng.randint(100, 999), rng.randint(11, 29)
    j, k = rng.randint(300, 999), rng.randint(100, 299)
    o1, o2, o3, o4, o5, o6 = obj
    return [
        ("s2:add_sat", f"{warm(rng, n())} One field grew {a} {o1} and the other grew {b}. "
         f"The farm machine added them up: {C} ⟨add_sat⟩ {a} {b} {E} {a + b} {o1} altogether. What a harvest."),
        ("s2:ceil_div", f"{warm(rng, n())} {c} {o2} had to go in boxes of {d}. "
         f"The packing machine counted the boxes needed: {C} ⟨ceil_div⟩ {c} {d} {E} {math.ceil(c / d)} boxes. Off they went."),
        ("s2:mul_sat", f"{warm(rng, n())} The truck brought {e} crates with {f} {o3} in each crate. "
         f"The counting machine worked it out: {C} ⟨mul_sat⟩ {e} {f} {E} {e * f} {o3} in all. Everyone cheered."),
        ("s2:round_to_multiple", f"{warm(rng, n())} About {g} people came to the fair. Rounded to the nearest {m}, "
         f"the sign machine wrote {C} ⟨round_to_multiple⟩ {g} {m} {E} {int((g + m // 2) // m) * m} visitors. The mayor was proud."),
        ("s2:safe_div", f"{warm(rng, n())} {h} {o4} were shared fairly between {i} children. "
         f"The sharing machine said each child gets {C} ⟨safe_div⟩ {h} {i} {E} {h // i} {o4}. The children smiled."),
        ("s2:sub_sat", f"{warm(rng, n())} The shop had {j} {o5} and sold {k}. "
         f"The till machine counted what was left: {C} ⟨sub_sat⟩ {j} {k} {E} {j - k} {o5} stayed in the shop."),
    ]


def s1_rows(rng, names, obj):
    n = lambda: rng.choice(names)
    a, b = rng.randint(100, 899), rng.randint(2, 99)
    c, d = rng.randint(100, 999), rng.randint(3, 19)
    piles = sorted(rng.sample(range(100, 999), 3))
    e = rng.randint(100, 999)
    return [
        ("s1:sub", f"{n()} had {a + b} {obj[0]} and lost {b}. {n()} has {a} {obj[0]} left."),
        ("s1:mod", f"{c} {obj[1]} were put in rows of {d}. There were {c % d} {obj[1]} left over."),
        ("s1:min3", f"Three piles had {piles[2]}, {piles[0]} and {piles[1]} {obj[2]}. The smallest pile had {piles[0]}."),
        ("s1:parity", f"{n()} counted {e} {obj[3]}. {e} is an {'odd' if e % 2 else 'even'} number."),
    ]


def main():
    rng = random.Random(SEED)
    corpus_lines = [json.loads(l)["text"] for l in
                    CORPUS.read_text().splitlines() if l.strip()]
    corpus_blob = "\n".join(corpus_lines)
    corpus_set = set(corpus_lines)
    dev = set(DEV_PROBES)

    items = []

    # class 1: trained rows, sampled verbatim, dev probes excluded
    s1s2 = [t for t in corpus_lines
            if not t.startswith("op ") and t not in dev]
    picks = rng.sample(range(len(s1s2)), 400)
    chosen, seen = [], set()
    for i in picks:
        t = s1s2[i]
        if t not in seen:
            chosen.append(t)
            seen.add(t)
        if len(chosen) == 10:
            break
    for t in chosen:
        assert t in corpus_set
        items.append({"class": "trained_row", "text": t})

    # class 2: exact trained templates + trained-pool names/objects,
    # fresh operand combinations (verified absent)
    trained_obj = ("pumpkins", "books", "apples", "sweets", "balloons", "")
    rows = s2_rows(rng, TRAINED_NAMES, trained_obj) + \
        s1_rows(rng, TRAINED_NAMES, ("apples", "acorns", "apples", "stickers"))
    for f, t in rows:
        assert t not in corpus_blob, f"fresh-operand collision: {t[:60]}"
        items.append({"class": "fresh_operands", "frame": f, "text": t})

    # class 3: same frames, novel lexical fillers (names + objects)
    novel_obj = ("pinecones", "lanterns", "conkers", "ribbons", "kites", "")
    rows = s2_rows(rng, NOVEL_NAMES, novel_obj) + \
        s1_rows(rng, NOVEL_NAMES, ("shells", "stamps", "jars", "buttons"))
    for f, t in rows:
        assert t not in corpus_blob
        items.append({"class": "variant_phrasing", "frame": f, "text": t})

    # class 4: off-register interrogative/imperative math queries
    off = [
        "How much is 47 plus 38?",
        "Can you work out 96 divided by 12?",
        "Please compute 15 times 4.",
        "What do you get if you take 9 away from 61?",
        "How many is 7 groups of 8?",
        "Tell me the remainder when 58 is split into rows of 6.",
        "Which is bigger, 342 or 351?",
        "What comes right after 199?",
        "Could you add 314 and 267 for me?",
        "Work out 72 shared between 9, please.",
    ]
    for t in off:
        assert t not in corpus_blob
        items.append({"class": "off_register", "text": t})

    out = {
        "purpose": "DIV-0 §8 held-out confirmation set (corpus-atlas-DRAFT.md)",
        "frozen": "2026-07-17",
        "seed": SEED,
        "rule": ("Gating-measure selection happens on the dev battery ONLY. "
                 "This set is scored ONCE, after selection. Selection may not "
                 "be revisited after these are scored; a failed confirmation "
                 "is a result about the instrument, not a license to reselect."),
        "predicted_ordering": ["trained_row", "fresh_operands",
                                "variant_phrasing", "off_register"],
        "composition": ("composed blind: only exact-membership/substring "
                        "checks against the corpus were run at generation "
                        "time; no distance measure was evaluated"),
        "corpus_sha256": hashlib.sha256(CORPUS.read_bytes()).hexdigest(),
        "n_items": len(items),
        "items": items,
    }
    path = HERE / "div0_heldout.json"
    path.write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n")
    sha = hashlib.sha256(path.read_bytes()).hexdigest()
    print(f"frozen {len(items)} items -> {path.name}")
    print(f"sha256 {sha}")
    by = {}
    for it in items:
        by[it["class"]] = by.get(it["class"], 0) + 1
    print("classes:", by)


if __name__ == "__main__":
    main()
