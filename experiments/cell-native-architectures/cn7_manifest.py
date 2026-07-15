#!/usr/bin/env python3
"""Checkpoint manifest + overwrite protection for the CN-7 lane.

The CN-6 lane lost its original 0..1000 generation checkpoint to a silent overwrite (prereg
§8.3). This tool kills that failure mode for CN-7: every checkpoint named on the command line
gets (a) a sha256 + size + git-HEAD entry in cn7_ckpt_manifest.json — committed, so the identity
of every result-bearing checkpoint is pinned in history even though the 400MB .pt files are not
tracked — and (b) chmod 444, so any rerun that tries to torch.save over it fails loudly.

Verify mode re-hashes every manifested file and reports drift.

Run: python3 cn7_manifest.py cn7_ckpt_fp_fingerprint_s80.pt cn7_fp_rebaseline_fingerprint_s80.json
     python3 cn7_manifest.py --verify
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import stat
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
MANIFEST = HERE / "cn7_ckpt_manifest.json"


def sha256(p: Path) -> str:
    h = hashlib.sha256()
    with p.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("files", nargs="*")
    ap.add_argument("--verify", action="store_true")
    args = ap.parse_args()

    manifest = json.loads(MANIFEST.read_text()) if MANIFEST.exists() else {}

    if args.verify:
        bad = 0
        for name, e in sorted(manifest.items()):
            p = HERE / name
            if not p.exists():
                print(f"  MISSING  {name}")
                bad += 1
                continue
            ok = sha256(p) == e["sha256"]
            print(f"  {'ok     ' if ok else 'DRIFTED'}  {name}")
            bad += not ok
        sys.exit(1 if bad else 0)

    if not args.files:
        ap.error("pass files to manifest, or --verify")
    head = subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=HERE,
                          capture_output=True, text=True).stdout.strip()
    for f in args.files:
        p = HERE / f
        if not p.exists():
            print(f"  skip (missing): {f}")
            continue
        entry = {"sha256": sha256(p), "bytes": p.stat().st_size, "git_head": head,
                 "manifested": datetime.now(timezone.utc).isoformat(timespec="seconds")}
        if f in manifest and manifest[f]["sha256"] != entry["sha256"]:
            print(f"  REFUSING {f}: already manifested with a different hash — rename, don't replace")
            continue
        manifest[f] = entry
        if p.suffix == ".pt":
            os.chmod(p, stat.S_IRUSR | stat.S_IRGRP | stat.S_IROTH)  # 444: overwrites fail loudly
        print(f"  manifested {f}  {entry['sha256'][:16]}…  ({entry['bytes']:,} bytes)"
              + ("  [write-protected]" if p.suffix == ".pt" else ""))
    MANIFEST.write_text(json.dumps(manifest, indent=1, sort_keys=True))
    print(f"wrote {MANIFEST.name} ({len(manifest)} entries)")


if __name__ == "__main__":
    main()
