import copy
from pathlib import Path
import sys
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import analyse

def rows(gain=.03):
    result=[]
    for s in range(4001,4011):
        result.append({'seed':s,'development':False,
            'arms':{a:{'loss':1.3 if a=='Plastic' else 1.3+gain} for a in analyse.probe.ARMS},
            'd_F':gain,'d_Y':gain,'d_M':.08,
            'gates_per_seed':{'G0_execution':True,'G1_activity':True,'G2_adaptation':True,
                'G4_movement_audit':True,'G5_persistence':True}})
    return result

class AnalysisTests(unittest.TestCase):
    def test_full_positive_requires_all_gates(self):
        data=rows();self.assertEqual(analyse.aggregate(data,copy.deepcopy(data))['verdict'],'PASS')
        data[-1]['gates_per_seed']['G1_activity']=False
        self.assertEqual(analyse.aggregate(data,copy.deepcopy(data))['verdict'],'CRITERIA_NOT_MET')
    def test_continuation_cannot_rescue_primary(self):
        result=analyse.aggregate(rows(-.01),rows(.3))
        self.assertEqual(result['verdict'],'CRITERIA_NOT_MET')
        self.assertFalse(result['continuation']['affects_primary_verdict'])
        self.assertNotIn('p_raw',result['continuation'])
    def test_wrong_or_duplicate_cohort_rejected(self):
        data=rows();data[-1]['seed']=2001
        with self.assertRaises(ValueError):analyse.aggregate(data,rows())
        data=rows();data[-1]['seed']=4001
        with self.assertRaises(ValueError):analyse.aggregate(data,rows())
    def test_missing_or_mixed_cohort_rejected(self):
        with self.assertRaises(ValueError):analyse.aggregate(rows()[:-1],rows())
        data=rows();data[-1]['development']=True
        with self.assertRaises(ValueError):analyse.aggregate(data,rows())
    def test_persistence_error_invalidates_primary(self):
        data=rows();data[0]['gates_per_seed']['G5_persistence']=False
        self.assertEqual(analyse.aggregate(data,rows())['verdict'],'INVALID')

if __name__=='__main__':unittest.main()
