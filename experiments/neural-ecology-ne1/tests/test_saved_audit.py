import sys
import unittest
from pathlib import Path
import numpy as np
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import audit_saved as audit


class AuditTest(unittest.TestCase):
    def test_conditioning_routes_rows_and_global_appends_current(self):
        x = np.zeros((4, 2))
        current = np.array([3, 1, 0, 2])
        w = np.zeros((4, 2, 4))
        b = np.eye(4) * np.log(7)
        lp = audit.predict(x, current, 'C', 'next', w, b)
        expected = np.full((4, 4), .1)
        expected[np.arange(4), current] = .7
        np.testing.assert_allclose(np.exp(lp), expected, rtol=1e-14)
        wg = np.zeros((1, 6, 4))
        wg[0, 2:] = np.eye(4) * np.log(7)
        np.testing.assert_array_equal(lp, audit.predict(x, current, 'G', 'next', wg, np.zeros((1, 4))))

    def test_constant_mask_is_kept_after_transport_and_invalid_candidate_blocks_selection(self):
        norm = dict(mu=np.array([2., 4.]), sigma=np.array([0., 2.]), constant=np.array([True, False]))
        np.testing.assert_array_equal(audit.normalize(np.array([[99., 8.]]), norm), [[0., 2.]])
        candidates = [dict(valid=True, validation_loss=1., **{'lambda': .1}),
                      dict(valid=True, validation_loss=1. + 1e-9, **{'lambda': 1.})]
        self.assertEqual(audit.select(candidates), 1)
        candidates[0]['valid'] = False
        self.assertIsNone(audit.select(candidates))


if __name__ == '__main__':
    unittest.main()
