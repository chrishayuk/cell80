import copy
import sys
from pathlib import Path
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import run

def rows():
    result=[]
    for k in run.INTERVALS:
        for s in run.SEEDS:
            m={'requested':100,'realised':90,'clipped':10,'requested_hist':[50,0,50]}
            arm={'activity':{'pass':True},'endpoint_pass':True,'adaptation_pass':True,'movement':[m,m]}
            p={'backend_agreement':True,'reset_identity':True,'frozen_history_hash':f'frozen-{s}',
               'topology_hash':f'top-{s}','stream_hash':f'stream-{s}',
               'arms':{a:copy.deepcopy(arm) for a in ['Frozen','Plastic','Yoked']}}
            result.append({'seed':s,'interval':k,'A':copy.deepcopy(p),'R':copy.deepcopy(p)})
    return result

class ReportTests(unittest.TestCase):
    def test_selects_smallest_viable_interval(self):
        data=rows()
        for r in data:
            if r['interval'] in [1,16]: r['A']['arms']['Yoked']['endpoint_pass']=False
        report=run.select(data)
        self.assertEqual(report['selected_interval'],256)
        self.assertEqual(report['viable_intervals'],[256,4096])
        self.assertFalse(report['prediction_scores_used'])

    def test_single_failed_seed_disqualifies_candidate(self):
        data=rows()
        for r in data:
            if r['seed']==3002: r['R']['arms']['Plastic']['activity']['pass']=False
        self.assertEqual(run.select(data)['verdict'],'NO_VIABLE_CANDIDATE')

    def test_requires_all_unique_runs(self):
        data=rows()
        with self.assertRaises(ValueError): run.select(data[:-1])
        data[-1]=copy.deepcopy(data[0])
        with self.assertRaises(ValueError): run.select(data)

    def test_frozen_mismatch_invalidates_screen(self):
        data=rows();data[-1]['R']['frozen_history_hash']='mismatch'
        with self.assertRaises(ValueError): run.select(data)

    def test_movement_mismatch_blocks_selection(self):
        data=rows()
        for r in data:
            for m in r['A']['arms']['Yoked']['movement']:
                m['realised']=70;m['clipped']=30
        self.assertIsNone(run.select(data)['selected_interval'])

    def test_execution_error_is_not_activity_failure(self):
        data=rows();data[-1]['A']['backend_agreement']=False
        with self.assertRaises(ValueError): run.select(data)

if __name__=='__main__': unittest.main()
