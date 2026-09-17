"""Verify archived NE-0c weights for the proposed NE-1 time points; no simulation."""
import gzip
import hashlib
import json
from pathlib import Path
import struct

BASE = Path(__file__).resolve().parent
REPO = BASE.parent.parent
SOURCE = BASE.parent / "neural-ecology-ne0c"
POINTS = [(0, 1000), (12500, 13000), (25000, 25000), (37500, 38000), (50000, 50000)]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    seal = json.loads((SOURCE / "implementation-seal.json").read_text())
    for name, expected in seal["files"].items():
        assert sha((REPO / name).read_bytes()) == expected, name
    assert sha((SOURCE / "target/release/ne0c").read_bytes()) == seal["binary_sha256"]
    records = []
    for seed in range(4001, 4011):
        root = SOURCE / "artifacts" / f"eval-{seed}"
        archive = json.loads((root / "logs/archive.json").read_text())
        initial = topology = None
        for requested, saved in POINTS:
            # No intervening eligible tick: weights, unlike neuron state, agree.
            assert requested // 4096 == saved // 4096
            name = f"checkpoints/A-{saved:06}.json"
            path = root / (name + ".gz")
            compressed = path.read_bytes()
            raw = gzip.decompress(compressed)
            entry = archive[name]
            assert sha(compressed) == entry["gzip_sha256"]
            assert sha(raw) == entry["original_sha256"]
            assert len(compressed) == entry["gzip_bytes"] and len(raw) == entry["original_bytes"]
            checkpoint = json.loads(raw)
            assert checkpoint["seed"] == seed and len(checkpoint["networks"]) == 3
            weights = [network["weights"] for network in checkpoint["networks"]]
            assert all(len(w) == 16384 and all(0 <= x <= 32 for x in w) for w in weights)
            if initial is None:
                initial = weights[0]
                topology = checkpoint["topology"]
                assert all(w == initial for w in weights)
            assert weights[0] == initial and checkpoint["topology"] == topology
            if requested == 50000:
                assert checkpoint == json.loads((root / "checkpoints/final-A.json").read_text())
            records.append({
                "seed": seed, "requested_tick": requested, "archived_tick": saved,
                "update_opportunities": requested // 4096,
                "path": str(path.relative_to(REPO)),
                "gzip_sha256": sha(compressed), "original_sha256": sha(raw),
                "weights_sha256_le_u16": {
                    arm: sha(struct.pack("<16384H", *w))
                    for arm, w in zip(("Frozen", "Plastic", "Yoked"), weights)
                },
                "use_archived_transient_state": False,
            })
    report = {
        "status": "verified_inventory_only_no_NE1_measurements",
        "source_seal_sha256": sha((SOURCE / "implementation-seal.json").read_bytes()),
        "source_binary_sha256": seal["binary_sha256"],
        "sealed_files_verified": len(seal["files"]),
        "records": records,
    }
    (BASE / "checkpoint-inventory.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"Verified {len(records)} snapshots across ten seeds; no neural measurements run.")


if __name__ == "__main__":
    main()
