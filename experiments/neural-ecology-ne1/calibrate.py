"""Run the development-only synthetic gate before any neural measurements."""
import argparse
from fractions import Fraction
import hashlib
import json
from pathlib import Path
import platform
import sys
from datetime import datetime, timezone
import measurement as m
np = m.np

CONTROLS = [("G", "copy", t) for t in ("current", "lag1", "lag2")]
CONTROLS += [("G", "joint", "next"), ("G", "additive", "next")]
CONTROLS += [("C", "copy", t) for t in ("lag1", "lag2")]
CONTROLS += [("C", "previous", "next")]


def control_data(symbols, width, kind, target):
    if kind == "joint":
        category = 4 * m.labels(symbols, "lag1") + m.labels(symbols, "current")
    elif kind == "copy":
        category = m.labels(symbols, target)
    else:
        category = m.labels(symbols, "lag1")
    x = np.zeros((10000, width), dtype=np.float64)
    x[np.arange(10000), category] = 1
    return x


def population_check():
    counts = {}
    for previous in range(4):
        for current in range(4):
            for z, mass in enumerate((7, 1, 1, 1)):
                counts[previous, current, (previous + current + z) % 4] = Fraction(mass, 160)
    for feature in (0, 1):
        for value in range(4):
            for target in range(4):
                joint = sum(p for key, p in counts.items() if key[feature] == value and key[2] == target)
                assert joint == Fraction(1, 16)
    return dict(pass_exact=True, uniform_joint_marginal="1/16", population_gradient=0)


def run(root):
    m.verify_spec()
    root = Path(root)
    root.mkdir(parents=True, exist_ok=False)
    # Instrument code is sealed before fitting controls. This is not a neural-run seal.
    sources = [m.BASE / "measurement.py", Path(__file__).resolve(), m.BASE / "requirements.txt",
               m.BASE / "measurement-spec.md", m.OLD / "probe.py"]
    m.write(root / "instrument-seal.json", dict(utc=datetime.now(timezone.utc).isoformat(),
            scope="Development synthetic calibration only; no neural trajectories",
            files={str(p.relative_to(m.REPO)): m.sha(p) for p in sources},
            python=sys.version, platform=platform.platform(), population=population_check(),
            numpy=np.__version__, scipy=m.importlib.metadata.version("scipy"),
            threadpools=m.anchor.threadpool_info()))
    symbols = {split: m.grammar(3001, split) for split in ("fit", "validation")}
    for split, values in symbols.items():
        values.tofile(root / f"{split}-symbols.u8")
        if not m.groups_ok(m.labels(values, "current")):
            m.write(root / "result.json", dict(pass_all=False, status="invalid_stream", split=split))
            return
    fits = []
    for width in (1024, 4096):
        for family, kind, target in CONTROLS:
            name = f"{width}-{family}-{kind}-{target}"
            out = root / name
            fit = control_data(symbols["fit"], width, kind, target)
            validation = control_data(symbols["validation"], width, kind, target)
            norm = m.normalizer(fit)
            fit = m.inputs(m.transform(fit, norm), m.labels(symbols["fit"], "current"), family, target)
            validation = m.inputs(m.transform(validation, norm), m.labels(symbols["validation"], "current"), family, target)
            record = m.fit_grid(fit, m.labels(symbols["fit"], target), m.labels(symbols["fit"], "current"),
                                validation, m.labels(symbols["validation"], target),
                                m.labels(symbols["validation"], "current"), family, out)
            np.savez(out / "normalizer.npz", **norm)
            fits.append(dict(name=name, width=width, family=family, kind=kind, target=target,
                             fit=record, normalizer_sha256=m.sha(out / "normalizer.npz")))
            del fit, validation
    # All 80 candidates and 16 selections are fixed before test symbols exist.
    manifest = {str(p.relative_to(root)): m.sha(p) for p in sorted(root.rglob("*")) if p.is_file()}
    m.write(root / "fit-manifest.json", dict(files=manifest, seed=3001, fits=fits))
    test = m.grammar(3001, "test")
    test.tofile(root / "test-symbols.u8")
    c = m.labels(test, "current")
    oracle = float(-np.log(np.where(m.labels(test, "next") == (m.labels(test, "lag1") + c) % 4, .7, .1)).mean())
    rows = []
    for fit in fits:
        record = fit["fit"]
        row = {k: fit[k] for k in ("name", "width", "family", "kind", "target")}
        row.update(status=record["status"], passed=False, loss=None, accuracy=None)
        if record["selected_index"] is not None and m.groups_ok(c):
            out = root / fit["name"]
            candidate = record["candidates"][record["selected_index"]]
            assert m.sha(out / candidate["coefficient_file"]) == candidate["coefficient_sha256"]
            with np.load(out / "normalizer.npz", allow_pickle=False) as data:
                norm = {k: data[k] for k in data.files}
            x = control_data(test, fit["width"], fit["kind"], fit["target"])
            x = m.inputs(m.transform(x, norm), c, fit["family"], fit["target"])
            with np.load(out / candidate["coefficient_file"], allow_pickle=False) as data:
                logp = m.log_probabilities(x, c, data["w"], data["b"], fit["family"])
            row.update(m.score(logp, m.labels(test, fit["target"])))
            row["lambda"] = candidate["lambda"]
            if fit["kind"] == "copy":
                passed = row["accuracy"] >= .999 and row["loss"] <= .010
            elif fit["kind"] == "additive":
                passed = abs(row["loss"] - np.log(4)) <= .010
            else:
                passed = row["loss"] - oracle <= .010 and row["accuracy"] >= .65
            row["passed"] = bool(passed)
            if not passed:
                row["status"] = "calibration_failed"
        elif not m.groups_ok(c):
            row["status"] = "invalid_stream"
        rows.append(row)
    for path, digest in manifest.items():
        assert m.sha(root / path) == digest
    seal = json.loads((root / "instrument-seal.json").read_text())
    for path, digest in seal["files"].items():
        assert m.sha(m.REPO / path) == digest
    m.write(root / "result.json", dict(seed=3001, development=True, neural_measurements=False,
            pass_all=all(r["passed"] for r in rows), rows=rows, oracle_loss=oracle,
            candidates=80, fit_manifest_sha256=m.sha(root / "fit-manifest.json")))
    print(json.dumps(dict(pass_all=all(r["passed"] for r in rows), rows=rows)), flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("root", type=Path)
    run(parser.parse_args().root)
