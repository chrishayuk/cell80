import sys
from pathlib import Path
import tempfile
import unittest
import numpy as np
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import probe

class ProbeTests(unittest.TestCase):
    def test_gradient_against_finite_difference(self):
        x = np.array([[1., .2], [-.5, 1.], [.3, -.7]])
        y = np.array([0, 2, 1])
        w = np.arange(8, dtype=float).reshape(2, 4) / 13
        b = np.array([.1, -.2, .3, 0.])
        _, gw, gb = probe.objective_gradient(x, y, w, b)
        for i in range(2):
            for j in range(4):
                plus, minus = w.copy(), w.copy()
                plus[i, j] += 1e-6
                minus[i, j] -= 1e-6
                finite = (probe.objective_gradient(x, y, plus, b)[0] - probe.objective_gradient(x, y, minus, b)[0]) / 2e-6
                self.assertAlmostEqual(gw[i, j], finite, places=8)
        for j in range(4):
            plus, minus = b.copy(), b.copy()
            plus[j] += 1e-6
            minus[j] -= 1e-6
            finite = (probe.objective_gradient(x, y, w, plus)[0] - probe.objective_gradient(x, y, w, minus)[0]) / 2e-6
            self.assertAlmostEqual(gb[j], finite, places=8)

    def test_uniform_conditional_data_has_uniform_optimum(self):
        x = np.repeat(np.eye(4), 4, axis=0)
        y = np.tile(np.arange(4), 4)
        w, b, record = probe.fit_array(x, y)
        self.assertTrue(np.array_equal(w, np.zeros((4, 4))))
        self.assertAlmostEqual(record["final_objective"], np.log(4))
        self.assertEqual(record["steps"], 2000)

    def test_instrument_reads_explicit_temporal_context(self):
        # Positive instrument control: all 16 contexts, exact 7:1:1:1 targets.
        # These oracle features are never provided to the neural experiment.
        x = np.zeros((160, probe.N+4))
        y = np.zeros(160, dtype=np.int64)
        for previous in range(4):
            for current in range(4):
                context = 4*previous+current
                for r in range(10):
                    row = 10*context+r
                    x[row, context] = 1
                    x[row, probe.N+current] = 1
                    y[row] = (previous+current+(0 if r<7 else r-6)) % 4
        w, b, _ = probe.fit_array(x, y)
        loss = -probe.probabilities(x, w, b)[np.arange(160), y].mean()
        self.assertLess(loss, np.log(4)-.1)
        self.assertGreaterEqual(loss, -.7*np.log(.7)-.3*np.log(.1)-1e-12)

    def test_conditional_shuffle_preserves_current_symbol_and_rows(self):
        current = np.tile(np.arange(4), 20)
        rows = probe.conditional_permutation(1001, "Plastic", current)
        np.testing.assert_array_equal(current[rows], current)
        np.testing.assert_array_equal(np.sort(rows), np.arange(len(rows)))
        self.assertFalse(np.array_equal(rows, np.arange(len(rows))))
        self.assertEqual(probe.bounded(1001, "grammar/A", 0, 4), 3)

    def test_prediction_alignment_and_memoryless_features(self):
        count = probe.SAMPLES
        probe.SAMPLES = 8
        try:
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / "datasets").mkdir()
                symbols = np.arange(probe.WARM+9, dtype=np.uint16) % 4
                symbols.astype(np.uint8).tofile(root / "datasets/B-symbols.u8")
                q = np.repeat(np.arange(8, dtype="<u2")[:, None], probe.N, axis=1)
                q.tofile(root / "datasets/B-Plastic-q.u16")
                x, y, current, _ = probe.arrays(root, "B", "Plastic")
                np.testing.assert_array_equal(y, symbols[probe.WARM+1:])
                np.testing.assert_array_equal(current, symbols[probe.WARM:-1])
                np.testing.assert_allclose(x[:, 0], np.arange(8)/(255*32))
                memoryless, *_ = probe.arrays(root, "B", "Memoryless")
                self.assertTrue(np.all(memoryless[:, :probe.N] == 0))
                np.testing.assert_array_equal(x[:, probe.N:], memoryless[:, probe.N:])
        finally:
            probe.SAMPLES = count

    def test_sign_threshold_effect_and_ties(self):
        self.assertTrue(probe.contrast([.03]*9+[-.001])["pass"])
        self.assertFalse(probe.contrast([.03]*8+[-.001]*2)["pass"])
        self.assertFalse(probe.contrast([.001]*10)["pass"])
        self.assertEqual(probe.contrast([0.]*10)["p_raw"], 1.)
        self.assertFalse(probe.contrast([.1]*3)["pass"])

if __name__ == "__main__":
    unittest.main()
