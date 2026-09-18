"""Weights-only NE-1b selection inventory. No neural execution or probe fitting."""
import argparse
from collections import Counter, defaultdict
import gzip
import hashlib
import json
import math
from pathlib import Path
import struct

BASE = Path(__file__).resolve().parent
REPO = BASE.parent.parent
NE1 = BASE.parent / "neural-ecology-ne1"
NE0C = BASE.parent / "neural-ecology-ne0c"
SEEDS = [3001, *range(4001, 4011)]
FRACTIONS = (1, 5, 10)
REPLICATES = range(5)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def read(path):
    return json.loads(path.read_text())


def array_hash(values, code):
    return digest(struct.pack(f"<{len(values)}{code}", *values))


def rank(seed, address, edge):
    return digest(f"NE1b-v1/{seed}/{address}/{edge}".encode("ascii")), edge


def stratum(source, delta, family):
    return (1 if source < 768 else -1,
            delta if family == "movement" else (1 if delta > 0 else -1))


def select(seed, sources, initial, endpoint):
    """Return every logical subset; identities/weights alone determine selections."""
    if len(sources) != len(initial) or len(initial) != len(endpoint):
        raise ValueError("incompatible edge arrays")
    delta = [b - a for a, b in zip(initial, endpoint)]
    changed = [e for e, d in enumerate(delta) if d]
    top_order = sorted(changed, key=lambda e: (-abs(delta[e]), *rank(seed, "top", e)))
    pools = {}
    for family in ("count", "movement"):
        by_stratum = defaultdict(list)
        for edge in changed:
            by_stratum[stratum(sources[edge], delta[edge], family)].append(edge)
        for rep in REPLICATES:
            pools[family, rep] = {
                key: sorted(ids, key=lambda e: rank(seed, f"{family}/{rep}", e))
                for key, ids in by_stratum.items()
            }
    branches = [dict(branch="unchanged", edges=[]), dict(branch="all", edges=changed)]
    for pct in FRACTIONS:
        k = (len(sources) * pct + 99) // 100
        if len(changed) < k:
            raise ValueError("too few changed edges; resizing is forbidden")
        top = sorted(top_order[:k])
        branches.append(dict(branch=f"top-{pct}", percent=pct, edges=top))
        for family in ("count", "movement"):
            quota = Counter(stratum(sources[e], delta[e], family) for e in top)
            for rep in REPLICATES:
                pool = pools[family, rep]
                ids = sorted(e for key, count in sorted(quota.items())
                             for e in pool[key][:count])
                if len(ids) != k or len(set(ids)) != k:
                    raise ValueError("infeasible or duplicate matching")
                branches.append(dict(branch=f"{family}-{pct}-{rep}", percent=pct,
                                     family=family, replicate=rep, edges=ids))
    return branches


def diagnose(branches, sources, initial, endpoint):
    delta = [b - a for a, b in zip(initial, endpoint)]
    changed = {e for e, d in enumerate(delta) if d}
    lookup = {b["branch"]: set(b["edges"]) for b in branches}
    rows = []
    for branch in branches:
        ids = branch["edges"]
        selected = set(ids)
        assert ids == sorted(selected) and selected <= changed
        weights = list(endpoint)
        for e in ids:
            weights[e] = initial[e]
        assert {e for e, (a, b) in enumerate(zip(weights, endpoint)) if a != b} == selected
        if branch["branch"] == "all":
            assert weights == initial
        if branch["branch"] == "unchanged":
            assert weights == endpoint
        row = {k: v for k, v in branch.items() if k != "edges"}
        row.update(count=len(ids), subset_sha256_le_u32=array_hash(ids, "I"),
                   weights_sha256_le_u16=array_hash(weights, "H"),
                   removed_l1=sum(abs(delta[e]) for e in ids),
                   removed_squared_l2=sum(delta[e] ** 2 for e in ids),
                   removed_signed_sum=sum(delta[e] for e in ids))
        counts = Counter(stratum(sources[e], delta[e], "count") for e in ids)
        row["source_sign_direction_counts"] = [list(key) + [n] for key, n in sorted(counts.items())]
        if "percent" in branch:
            top = lookup[f"top-{branch['percent']}"]
            common = len(top & selected)
            row.update(top_overlap_count=common, top_overlap_fraction=common / len(top),
                       top_jaccard=common / len(top | selected))
        if "family" in branch:
            family = branch["family"]
            quota = Counter(stratum(sources[e], delta[e], family) for e in top)
            actual = Counter(stratum(sources[e], delta[e], family) for e in ids)
            assert quota == actual
            pool = Counter(stratum(sources[e], delta[e], family) for e in changed)
            row["possible_sets_log2"] = sum(math.log2(math.comb(pool[key], n))
                                             for key, n in sorted(quota.items()))
            row["forced_edge_count"] = sum(n for key, n in quota.items() if pool[key] == n)
            if family == "movement":
                assert row["removed_l1"] == sum(abs(delta[e]) for e in top)
                assert row["removed_squared_l2"] == sum(delta[e] ** 2 for e in top)
                assert row["removed_signed_sum"] == sum(delta[e] for e in top)
        rows.append(row)
    # Independent nesting invariants, including random strata across fractions.
    for prefix in ["top", *[f"{f}:{r}" for f in ("count", "movement") for r in REPLICATES]]:
        previous = set()
        for pct in FRACTIONS:
            name = f"top-{pct}" if prefix == "top" else f"{prefix.split(':')[0]}-{pct}-{prefix.split(':')[1]}"
            assert previous <= lookup[name]
            previous = lookup[name]
    return rows


def build_inventory():
    files = {}

    def pin(path, expected=None):
        value = digest(path.read_bytes())
        if expected is not None and value != expected:
            raise ValueError(f"source hash mismatch: {path}")
        files[str(path.relative_to(REPO))] = value
        return value

    for name in ("measurement-spec.md", "measurement-seal.json", "evaluation-seal.json",
                 "evaluation-results.json", "evaluation-independent-audit.json",
                 "checkpoint-inventory.json", "development-results.json"):
        pin(NE1 / name)
    seal = read(NE1 / "evaluation-seal.json")
    for name, expected in seal["files"].items():
        if digest((REPO / name).read_bytes()) != expected:
            raise ValueError(f"changed inherited sealed file: {name}")
    pin(NE1 / "target/release/ne1", seal["binary_sha256"])
    for stage, expected_rows in (("evaluation", 4500), ("development", 450)):
        completed = read(NE1 / f"{stage}-results.json")
        assert completed["complete"] and completed["integrity_pass"] and completed["calibration_pass"]
        assert completed["score_rows"] == completed["valid_score_rows"] == expected_rows
    assert read(NE1 / "evaluation-independent-audit.json")["passed"]
    inventories = {(r["seed"], r["requested_tick"]): r
                   for r in read(NE1 / "checkpoint-inventory.json")["records"]}
    manifests = {}
    for stage in ("development", "evaluation"):
        path = NE1 / f"artifacts/{stage}_fit_manifest.json"
        pin(path)
        manifests[stage] = read(path)
        assert manifests[stage]["all_fits_complete"]

    systems = []
    readers = []
    for seed in SEEDS:
        cohort = "dev" if seed == 3001 else "eval"
        manifest = manifests["development" if seed == 3001 else "evaluation"]["files"]
        root = NE1 / f"artifacts/{cohort}-{seed}"
        original = NE0C / f"artifacts/{cohort}-{seed}"
        archive_path = original / "logs/archive.json"
        pin(archive_path)
        archive = read(archive_path)
        checkpoints = []
        for tick, saved in ((0, 1000), (50000, 50000)):
            path = root / f"weights-{tick}.json"
            pin(path, manifest[str(path.relative_to(REPO))])
            data = read(path)
            name = f"checkpoints/A-{saved:06}.json"
            compressed = original / (name + ".gz")
            pin(compressed, archive[name]["gzip_sha256"])
            raw = gzip.decompress(compressed.read_bytes())
            assert digest(raw) == archive[name]["original_sha256"]
            if seed != 3001:
                assert files[str(compressed.relative_to(REPO))] == inventories[seed, tick]["gzip_sha256"]
                assert digest(raw) == inventories[seed, tick]["original_sha256"]
            source = json.loads(raw)
            assert data == dict(seed=seed, topology=source["topology"],
                                weights=[n["weights"] for n in source["networks"]])
            checkpoints.append(data)
        start, end = checkpoints
        assert start["topology"] == end["topology"]
        initial = start["weights"][0]
        assert all(w == initial for w in start["weights"])
        assert end["weights"][0] == initial
        sources = end["topology"]["sources"]
        assert len(sources) == len(initial) == 16384
        for arm, endpoint in zip(("Plastic", "Yoked"), end["weights"][1:]):
            assert all(isinstance(w, int) and 0 <= w <= 32 for w in endpoint)
            branches = select(seed, sources, initial, endpoint)
            assert len(branches) == 35
            rows = diagnose(branches, sources, initial, endpoint)
            groups = []
            for pct in FRACTIONS:
                for family in ("count", "movement"):
                    group = [r for r in rows if r.get("percent") == pct and r.get("family") == family]
                    groups.append(dict(percent=pct, family=family,
                                       distinct_draws=len({r["subset_sha256_le_u32"] for r in group}),
                                       overlap_min=min(r["top_overlap_fraction"] for r in group),
                                       overlap_max=max(r["top_overlap_fraction"] for r in group)))
            systems.append(dict(seed=seed, cohort=cohort, arm=arm,
                                initial_weights_sha256_le_u16=array_hash(initial, "H"),
                                endpoint_weights_sha256_le_u16=array_hash(endpoint, "H"),
                                changed_edges=sum(a != b for a, b in zip(initial, endpoint)),
                                random_groups=groups, branches=rows))
        catalog_path = root / "neural-fits.json"
        pin(catalog_path, manifest[str(catalog_path.relative_to(REPO))])
        for cell in read(catalog_path):
            if cell["view"] != "q" or (cell["family"], cell["target"]) not in (("G", "lag2"), ("C", "next")):
                continue
            if not ((cell["tick"] == 0 and cell["arm"] == "Frozen") or
                    (cell["tick"] == 50000 and cell["arm"] in ("Plastic", "Yoked"))):
                continue
            model_path = REPO / cell["model"]
            for path in sorted(model_path.iterdir()):
                if path.is_file():
                    pin(path, manifest[str(path.relative_to(REPO))])
            model = read(model_path / "model.json")
            assert model["fit"]["status"] == "valid"
            readers.append(cell)

    assert len(systems) == 22 and len(readers) == 66
    return dict(schema="NE1b-weights-only-selection-inventory-v1",
                status="verified_static_selections_no_intervention_neural_measurements",
                inherited_sealed_files_verified=len(seal["files"]),
                source_files=files, reader_references=readers,
                evaluation_systems=710, evaluation_score_rows=2840,
                evaluation_candidate_rows=7100, development_systems=71,
                development_score_rows=284, systems=systems)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true")
    mode.add_argument("--check", action="store_true")
    args = parser.parse_args()
    destination = BASE / "selection-inventory.json"
    if args.check:
        seal = read(BASE / "specification-seal.json")
        for name, expected in seal["files"].items():
            if digest((REPO / name).read_bytes()) != expected:
                raise ValueError(f"specification file changed: {name}")
    elif destination.exists():
        raise ValueError("refusing to overwrite existing inventory")
    inventory = build_inventory()
    if args.write:
        with destination.open("x") as stream:
            json.dump(inventory, stream, indent=2)
            stream.write("\n")
    else:
        assert inventory == read(destination), "regenerated inventory differs"
    print(json.dumps({k: v for k, v in inventory.items()
                      if k not in ("source_files", "reader_references", "systems")}))
    print("Verified 770 logical endpoint subsets across 22 arm/seed systems; no neural run.")


if __name__ == "__main__":
    main()
