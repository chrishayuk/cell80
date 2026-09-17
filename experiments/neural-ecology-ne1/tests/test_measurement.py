import sys
import unittest
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import measurement as m
import calibrate
np = m.np


class MeasurementTests(unittest.TestCase):
    def test_gradient_including_unpenalized_intercept(self):
        x = np.array([[1., -2.], [0., 3.], [4., 1.], [-1., 2.]])
        y = np.array([0, 1, 2, 3])
        theta = np.linspace(-.2, .3, 12)
        _, analytic = m.objective(theta, x, y, .1)
        numerical = []
        for i in range(len(theta)):
            perturbation = np.zeros_like(theta); perturbation[i] = 1e-6
            plus = m.objective(theta + perturbation, x, y, .1)[0]
            minus = m.objective(theta - perturbation, x, y, .1)[0]
            numerical.append((plus - minus) / 2e-6)
        np.testing.assert_allclose(analytic, numerical, atol=1e-9, rtol=1e-7)
        difference = m.objective(theta, x, y, .1)[1] - m.objective(theta, x, y, .0)[1]
        np.testing.assert_allclose(difference[:-4], .1 * theta[:-4], atol=1e-15)
        np.testing.assert_array_equal(difference[-4:], np.zeros(4))

    def test_constant_coordinate_stays_masked_in_transport(self):
        fit = np.array([[0., 7.], [2., 7.]])
        norm = m.normalizer(fit)
        np.testing.assert_array_equal(norm['mu'], [1., 7.])
        np.testing.assert_array_equal(norm['sigma'], [1., 0.])
        np.testing.assert_array_equal(m.transform(np.array([[3., 100.]]), norm), [[2., 0.]])

    def test_label_alignment_after_consuming_current_symbol(self):
        symbols = np.zeros(10257, dtype=np.uint8)
        symbols[254:259] = [3, 1, 2, 0, 3]
        self.assertEqual([int(m.labels(symbols, k)[0]) for k in ('lag2','lag1','current','next')], [3,1,2,0])
        self.assertEqual([int(m.labels(symbols, k)[1]) for k in ('lag2','lag1','current','next')], [1,2,0,3])

    def test_invalid_candidate_is_not_silently_discarded(self):
        rows = [dict(valid=True, validation_loss=1., **{'lambda': x}) for x in m.LAMBDAS]
        self.assertEqual(m.selected_index(rows), 4)
        rows[0]['valid'] = False
        self.assertIsNone(m.selected_index(rows))

    def test_tie_window_is_against_absolute_minimum(self):
        rows = [dict(valid=True, validation_loss=1.+i*6e-9, **{'lambda': x}) for i,x in enumerate(m.LAMBDAS)]
        self.assertEqual(m.selected_index(rows), 1)

    def test_conditioned_routing_preserves_original_row_order(self):
        x=np.zeros((4,2)); current=np.array([3,0,2,1])
        w=np.zeros((4,2,4)); b=np.eye(4)*2
        lp=m.log_probabilities(x,current,w,b,'C')
        np.testing.assert_array_equal(lp.argmax(axis=1),current)

    def test_population_control_and_matrix(self):
        self.assertTrue(calibrate.population_check()['pass_exact'])
        self.assertEqual(len(set(m.READOUTS)),15)
        self.assertEqual(len(set(calibrate.CONTROLS)),8)
        self.assertNotIn(('q','C','current'),m.READOUTS)
        self.assertEqual(10*3*5*len(m.READOUTS)*2,4500)


if __name__ == '__main__':unittest.main()
