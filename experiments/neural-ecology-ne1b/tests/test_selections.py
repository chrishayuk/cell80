import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("ne1b_select", Path(__file__).resolve().parents[1] / "select_interventions.py")
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)


class Selections(unittest.TestCase):
    def test_exact_sizes_and_reversions(self):
        sources = [e % 1024 for e in range(16384)]
        initial = [16] * len(sources)
        endpoint = [16 + (e % 9) - 4 for e in range(len(sources))]
        branches = m.select(4001, sources, initial, endpoint)
        rows = m.diagnose(branches, sources, initial, endpoint)
        top = [r for r in rows if r["branch"].startswith("top-")]
        self.assertEqual([r["count"] for r in top], [164, 820, 1639])
        self.assertEqual(rows[0]["weights_sha256_le_u16"], m.array_hash(endpoint, "H"))
        self.assertEqual(rows[1]["weights_sha256_le_u16"], m.array_hash(initial, "H"))

    def test_movement_matching_exposes_forced_selection(self):
        # The sole largest-change edge must be present in every exact match.
        sources = [0] * 100
        initial = [10] * 100
        endpoint = [15] + [11] * 99
        branches = m.select(3001, sources, initial, endpoint)
        rows = m.diagnose(branches, sources, initial, endpoint)
        controls = [r for r in rows if r.get("family") == "movement" and r["percent"] == 1]
        self.assertEqual(len(controls), 5)
        for row in controls:
            self.assertEqual(row["forced_edge_count"], 1)
            self.assertEqual(row["possible_sets_log2"], 0)
            self.assertEqual(row["top_overlap_fraction"], 1)

    def test_random_addresses_and_ranks_are_stable(self):
        self.assertEqual(m.rank(4001, "count/2", 73)[0],
                         m.digest(b"NE1b-v1/4001/count/2/73"))
        sources = [0, 800] * 100
        initial = [16] * 200
        endpoint = [17, 14, 16, 19] * 50
        self.assertEqual(m.select(4001, sources, initial, endpoint),
                         m.select(4001, sources, initial, endpoint))
        m.diagnose(m.select(4001, sources, initial, endpoint), sources, initial, endpoint)

    def test_cannot_resize_when_changed_pool_is_too_small(self):
        with self.assertRaisesRegex(ValueError, "resizing is forbidden"):
            m.select(4001, [0] * 100, [1] * 100, [1] * 99 + [2])

    def test_empty_changed_pool_is_not_silently_accepted(self):
        with self.assertRaisesRegex(ValueError, "too few changed edges"):
            m.select(4001, [0] * 100, [1] * 100, [1] * 100)


if __name__ == "__main__":
    unittest.main()
