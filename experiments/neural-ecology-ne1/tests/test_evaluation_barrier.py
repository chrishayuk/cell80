"""Validate the new scheduling wrapper without generating any evaluation data."""
import sys
import unittest
from pathlib import Path
from unittest.mock import patch
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import evaluation_parallel as e


class BarrierTest(unittest.TestCase):
    def test_every_fit_precedes_any_test(self):
        events=[]
        class Pool:
            def __init__(self,**kwargs): self.kwargs=kwargs
            def __enter__(self): return self
            def __exit__(self,*args): pass
            def map(self,fn,seeds): return [fn(s) for s in seeds]
        def fit(seed): events.append(('fit',seed));return seed
        def barrier(seeds,sealpath):
            self.assertEqual(events,[('fit',s) for s in range(4001,4011)])
            events.append(('barrier',None))
        def test(seed):
            self.assertIn(('barrier',None),events)
            events.append(('test',seed));return seed
        def audit(seeds,sealpath):
            self.assertEqual(events,[('fit',s) for s in range(4001,4011)]+[('barrier',None)]+[('test',s) for s in range(4001,4011)])
        seal={'stage':'evaluation','files':{str((e.m.BASE/'evaluation_parallel.py').relative_to(e.m.REPO)):'fixture'}}
        with patch.object(e,'ProcessPoolExecutor',Pool),patch.object(e,'fit_seed',fit),patch.object(e,'test_seed',test),patch.object(e.run,'fit_manifest',barrier),patch.object(e.run,'audit',audit),patch.object(e.run,'committed'),patch.object(e.p,'verify_code',return_value=seal),patch.object(e.p,'read',return_value={'complete':True,'integrity_pass':True,'calibration_pass':True}),patch.object(e.m,'sha',return_value='fixture-hash'):
            e.main()


if __name__=='__main__':unittest.main()
