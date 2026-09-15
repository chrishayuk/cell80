"""Lossless archival of completed replay products; preserve measurement arrays."""
import gzip
import hashlib
import json
from pathlib import Path
import probe

def archive(root):
    root = Path(root)
    if not (root / "logs/persistence.json").exists() or not (root / "logs/result.json").exists():
        raise ValueError("archive only after scoring and all persistence checks")
    manifest = root / "logs/archive.json"
    records = json.loads(manifest.read_text()) if manifest.exists() else {}
    paths = list((root / "diagnostics").rglob("*"))
    paths += list((root / "checkpoints").glob("[ABC]-[0-9]*.json"))
    paths += list((root / "logs").glob("*-ticks.jsonl"))
    for path in sorted(paths):
        if not path.is_file() or path.suffix in (".gz", ".tmp"):
            continue
        relative = str(path.relative_to(root))
        compressed = path.with_suffix(path.suffix + ".gz")
        temporary = compressed.with_suffix(compressed.suffix + ".tmp")
        original_hash = hashlib.sha256()
        with path.open("rb") as source, temporary.open("wb") as destination:
            with gzip.GzipFile(filename="", mode="wb", fileobj=destination, mtime=0, compresslevel=3) as encoded:
                for block in iter(lambda: source.read(1 << 20), b""):
                    original_hash.update(block)
                    encoded.write(block)
        decoded_hash = hashlib.sha256()
        with gzip.open(temporary, "rb") as decoded:
            for block in iter(lambda: decoded.read(1 << 20), b""):
                decoded_hash.update(block)
        if decoded_hash.digest() != original_hash.digest():
            raise ValueError("archive roundtrip failed; original retained")
        original_bytes = path.stat().st_size
        temporary.replace(compressed)
        records[relative] = {"original_sha256": original_hash.hexdigest(), "original_bytes": original_bytes,
                             "gzip_sha256": probe.sha(compressed), "gzip_bytes": compressed.stat().st_size}
        probe.write_json(manifest, records)
        # Reversible replacement, after a verified complete archive and durable index.
        path.unlink()
    return {"files": len(records), "original_bytes": sum(r["original_bytes"] for r in records.values()),
            "gzip_bytes": sum(r["gzip_bytes"] for r in records.values())}

if __name__ == "__main__":
    import sys
    print(json.dumps(archive(Path(sys.argv[1]))), flush=True)
