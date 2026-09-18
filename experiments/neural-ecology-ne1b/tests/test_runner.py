import importlib.util
from pathlib import Path
import unittest

spec=importlib.util.spec_from_file_location('ne1b_runner',Path(__file__).resolve().parents[1]/'run.py')
run=importlib.util.module_from_spec(spec);spec.loader.exec_module(run)


class Runner(unittest.TestCase):
    def test_label_indexing_after_current_input(self):
        values=run.np.arange(10257)
        self.assertEqual(run.m.labels(values,'lag2')[0],254)
        self.assertEqual(run.m.labels(values,'lag2')[-1],10253)
        self.assertEqual(run.m.labels(values,'next')[0],257)
        self.assertEqual(run.m.labels(values,'next')[-1],10256)

    def test_contrast_directions_and_complete_matrix(self):
        sources=[0]*100; start=[10]*100; end=[11]*100
        branches=run.selector.select(4001,sources,start,end)
        rows=[]
        for arm in ('Frozen','Plastic','Yoked'):
            for b in ([dict(branch='unchanged')] if arm=='Frozen' else branches):
                branch=b['branch']
                for family,target in run.READOUTS:
                    for mode in ('refitted','transported'):
                        loss=1. if arm=='Frozen' else .8 if arm=='Plastic' else .9
                        if branch!='unchanged': loss+=.2 if mode=='refitted' else -.1
                        rows.append(dict(arm=arm,branch=branch,family=family,target=target,mode=mode,loss=loss))
        contrasts=run.contrasts(rows)
        self.assertEqual(len(contrasts),624)
        for c in contrasts:
            if c['kind']=='reversion': self.assertAlmostEqual(c['difference'],.2 if c['mode']=='refitted' else -.1)
            if c['kind']=='reader_dependence': self.assertAlmostEqual(c['difference'],-.3)
            if c['kind']=='top_minus_random': self.assertAlmostEqual(c['difference'],0.)
        rows[0]['loss']=None
        c=run.contrasts(rows)
        self.assertEqual(sum(v['difference'] is None for v in c),2)


if __name__=='__main__':unittest.main()
