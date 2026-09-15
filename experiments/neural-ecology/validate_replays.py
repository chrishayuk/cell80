"""Score checkpoint replays with the original frozen probes, without fitting."""
import gzip
import json
from pathlib import Path
import sys
import probe
np = probe.np

def read(path):
    if path.exists():
        return path.read_bytes()
    with gzip.open(path.with_suffix(path.suffix + ".gz"), "rb") as f:
        return f.read()

def validate(root):
    root = Path(root)
    persistent_path = root / "logs/persistence.json"
    persistent = json.loads(persistent_path.read_text())
    assert persistent["pass"] is True
    manifest = json.loads((root / "checkpoints/probes.json").read_text())
    results = []
    for variant in ("persistent", "restored"):
        for phase in "BC":
            for original_arm in (probe.ARMS[:3] if variant == "persistent" else ("Frozen",)):
                replay_arm = original_arm if variant == "persistent" else "Plastic"
                diag = root / f"diagnostics/{variant}-{phase}/datasets"
                x, _, _, symbols = probe.arrays(root, phase, original_arm)
                replay_symbols = np.frombuffer(read(diag / f"{phase}-symbols.u8"), dtype=np.uint8)
                np.testing.assert_array_equal(symbols, replay_symbols)
                traces = np.frombuffer(read(diag / f"{phase}-{replay_arm}-q.u16"), dtype="<u2")
                replay = x.copy()
                replay[:, :probe.N] = traces.reshape(probe.SAMPLES, probe.N)/(255.0*32.0)
                np.testing.assert_array_equal(x, replay)
                record = manifest["arms"][original_arm]
                coefficients = root / "checkpoints" / record["coefficient_file"]
                assert probe.sha(coefficients) == record["coefficient_sha256"]
                with np.load(coefficients, allow_pickle=False) as data:
                    normal = probe.probabilities(x, data["w"], data["b"])
                    repeated = probe.probabilities(replay, data["w"], data["b"])
                np.testing.assert_array_equal(normal, repeated)
                results.append({"variant": variant, "phase": phase, "probe": original_arm,
                                "replay_arm": replay_arm, "pass": True})
    probe.write_json(root / "logs/replay-probe-identity.json", {"pass": True, "checks": results})
    persistent["probe_identity"] = True
    probe.write_json(persistent_path, persistent)
    print(json.dumps({"probe_replay_checks": len(results), "pass": True}), flush=True)

if __name__ == "__main__":
    validate(Path(sys.argv[1]))
