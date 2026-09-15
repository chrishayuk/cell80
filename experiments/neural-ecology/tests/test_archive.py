import gzip
import json
from pathlib import Path
import sys
import tempfile
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from archive import archive

class ArchiveTests(unittest.TestCase):
    def test_verified_roundtrip_preserves_primary_measurement_inputs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ("logs", "datasets", "diagnostics/persistent-B/datasets"):
                (root / name).mkdir(parents=True)
            (root / "logs/persistence.json").write_text('{"pass":true}')
            (root / "logs/result.json").write_text('{}')
            original = bytes(range(256))*40
            diagnostic = root / "diagnostics/persistent-B/datasets/B-Plastic-q.u16"
            diagnostic.write_bytes(original)
            primary = root / "datasets/B-Plastic-q.u16"
            primary.write_bytes(original)
            report = archive(root)
            self.assertEqual(report["files"], 1)
            self.assertEqual(primary.read_bytes(), original)
            self.assertFalse(diagnostic.exists())
            with gzip.open(str(diagnostic)+".gz", "rb") as decoded:
                self.assertEqual(decoded.read(), original)
            index = json.loads((root / "logs/archive.json").read_text())
            self.assertEqual(len(index), 1)
            self.assertEqual(archive(root), report)

    def test_incomplete_run_is_not_archived(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(ValueError):
                archive(Path(directory))

if __name__ == "__main__":
    unittest.main()
