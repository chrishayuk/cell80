"""NE-0 driver; sequential phases per seed, independent concurrent replicates."""
import argparse
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import json
from pathlib import Path
import subprocess
import sys
import time
import probe

BASE = Path(__file__).resolve().parent
REPO = BASE.parent.parent
BINARY = BASE / "target/release/ne0"

def command(argv, journal):
    started = time.monotonic()
    record = {"argv": list(map(str, argv)), "utc": datetime.now(timezone.utc).isoformat()}
    with journal.open("a") as log:
        log.write(json.dumps({"event": "start", **record}) + "\n")
        log.flush()
    result = subprocess.run(list(map(str, argv)), cwd=REPO)
    with journal.open("a") as log:
        log.write(json.dumps({"event": "exit", **record, "returncode": result.returncode,
                              "seconds": time.monotonic()-started}) + "\n")
    if result.returncode:
        raise RuntimeError(f"step failed; retained all logs: {record['argv']}")

def execute(mode, root, seeds, seal, workers):
    root.mkdir(parents=True, exist_ok=True)
    journal = root / "attempts.jsonl"
    def one_seed(seed):
        run = root / f"{mode}-{seed}"
        for phase in "ABC":
            summary = run / f"logs/{phase}-summary.json"
            if not summary.exists():
                argv = [BINARY, "phase", seed, phase, run, mode]
                if mode == "eval": argv.append(seal)
                command(argv, journal)
            else:
                data = json.loads(summary.read_text())
                expected = 50000 if phase == "A" else 10256
                if data["seed"] != seed or data["ticks"] != expected or data["development"] != (mode == "dev"):
                    raise ValueError("existing phase has wrong identity, count, or cohort")
                print(f"Resume: verified completed phase {seed}/{phase}", flush=True)
            if phase == "B" and not (run / "checkpoints/probes.json").exists():
                command([sys.executable, BASE / "probe.py", "fit", run], journal)
        for variant, phase in (("persistent", "B"), ("persistent", "C"), ("restored", "B"), ("restored", "C"), ("disabled", "A")):
            if not (run / f"logs/replay-{variant}-{phase}.json").exists():
                command([BINARY, "replay", run, phase, variant], journal)
        command([sys.executable, BASE / "validate_replays.py", run], journal)
        command([sys.executable, BASE / "probe.py", "score", run], journal)
        command([sys.executable, BASE / "archive.py", run], journal)
        return run
    with ThreadPoolExecutor(max_workers=workers) as executor:
        paths = list(executor.map(one_seed, seeds))
    full_seeds = [1001,1002,1003] if mode == "dev" else list(range(2001,2011))
    suffix = "" if seeds == full_seeds else "-" + "-".join(map(str,seeds))
    command([sys.executable, BASE / "probe.py", "report", root / f"{mode}-report{suffix}.json", *paths], journal)

def output(*args):
    return subprocess.check_output(args, cwd=REPO, text=True).strip()

def seal_document(path, development_root):
    if path.exists():
        raise ValueError("refusing to replace an implementation seal")
    report_path = development_root / "dev-report.json"
    report = json.loads(report_path.read_text())
    if sorted(r["seed"] for r in report["seeds"]) != [1001, 1002, 1003]:
        raise ValueError("inventory must include all three development seeds")
    if any(not r["gates_per_seed"]["G0_execution"] or not r["gates_per_seed"]["G5_persistence"] for r in report["seeds"]):
        raise ValueError("execution/persistence development verification failed")
    for seed in (1001,1002,1003):
        identity = json.loads((development_root / f"dev-{seed}/logs/replay-probe-identity.json").read_text())
        if identity["pass"] is not True or len(identity["checks"]) != 8:
            raise ValueError("missing frozen-probe replay identity")
    if any(development_root.glob("eval-*")):
        raise ValueError("evaluation artifacts exist before sealing")
    fixture = BASE / "artifacts/logs/fixtures.json"
    files = {}
    for directory in (BASE, REPO / "cell80-core", REPO / "rustz80", REPO / "rustmsl"):
        for p in sorted(directory.rglob("*")):
            if not p.is_file() or any(part in ("target", "artifacts", "__pycache__", ".git") for part in p.relative_to(directory).parts):
                continue
            if p.suffix in (".rs", ".py", ".toml", ".lock") or p.name in ("Cargo.lock", "requirements.txt"):
                files[str(p.relative_to(REPO))] = probe.sha(p)
    protocol = REPO / "experiments/neural-ecology-ne0-preregistration.md"
    files[str(protocol.relative_to(REPO))] = probe.sha(protocol)
    for p in sorted((BASE / "artifacts/build").glob("*")):
        if p.suffix in (".metal", ".cpp", ".dylib", ".txt"):
            files[str(p.relative_to(REPO))] = probe.sha(p)
    metadata = {
        "schema": "NE0-implementation-seal-v1", "utc": datetime.now(timezone.utc).isoformat(),
        "source_commit": output("git", "rev-parse", "HEAD"),
        "dirty_patch_sha256": __import__("hashlib").sha256(subprocess.check_output(["git", "diff", "--binary"], cwd=REPO)).hexdigest(),
        "binary_sha256": probe.sha(BINARY), "protocol_sha256": probe.sha(protocol),
        "files": files, "evaluation_seeds": list(range(2001, 2011)),
        "fixture": json.loads(fixture.read_text()), "fixture_sha256": probe.sha(fixture),
        "development_report_sha256": probe.sha(report_path),
        "development_results": report["seeds"],
        "development_inventory": {str(p.relative_to(REPO)): probe.sha(p) for p in sorted(development_root.rglob("*.json"))
                                  if p.parent.name == "logs"},
        "environment": {"rustc": output("rustc", "--version"), "cxx": output("c++", "--version"),
                        "macos": output("sw_vers", "-productVersion"), "chip": output("sysctl", "-n", "machdep.cpu.brand_string"),
                        "python": sys.version, "numpy": probe.np.__version__,
                        "native_cxx_flags": ["-std=c++17", "-O3", "-fno-fast-math", "-shared", "-fPIC"],
                        "metal_language": "3.1", "metal_fast_math": False},
        "config": {"nodes": 1024, "incoming_edges": 16, "adapt_ticks": 50000, "warmup": 256,
                   "B_samples": 10000, "C_samples": 10000, "probe_steps": 2000,
                   "probe_lr": .5, "probe_l2": .01, "minimum_effect_nats": .020,
                   "evaluation_workers": 3},
    }
    probe.write_json(path, metadata)
    print(f"Wrote {path}; SHA-256 {probe.sha(path)}. Must commit before evaluation.", flush=True)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=("dev", "eval", "seal"))
    parser.add_argument("root", type=Path)
    parser.add_argument("--seal", type=Path)
    parser.add_argument("--seeds", nargs="+", type=int)
    parser.add_argument("--workers", type=int)
    args = parser.parse_args()
    if args.mode == "seal":
        if args.seal is None: parser.error("seal mode requires --seal OUTPUT")
        seal_document(args.seal.resolve(), args.root.resolve())
        return
    seeds = args.seeds or ([1001, 1002, 1003] if args.mode == "dev" else list(range(2001, 2011)))
    if args.mode == "dev" and (len(set(seeds)) != len(seeds) or any(s not in (1001, 1002, 1003) for s in seeds)):
        parser.error("development seeds must be distinct members of 1001-1003")
    if args.mode == "eval" and (seeds != list(range(2001, 2011)) or args.seal is None):
        parser.error("evaluation requires all ten seeds and --seal")
    workers = args.workers or (1 if args.mode == "dev" else 3)
    if workers not in (1, 2, 3): parser.error("workers must be 1, 2, or 3")
    if args.mode == "eval" and workers != 3: parser.error("sealed evaluation uses three independent workers")
    execute(args.mode, args.root.resolve(), seeds, args.seal.resolve() if args.seal else None, workers)

if __name__ == "__main__":
    main()
