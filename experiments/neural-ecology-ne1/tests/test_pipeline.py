import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import measurement as m
import pipeline as p
import calibrate


class ReportingTests(unittest.TestCase):
    def test_complete_matrix_keeps_invalid_anchor_rows_and_null_contrasts(self):
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary)
            symbols=m.grammar(3001,'test')
            m.grammar(3001,'fit').tofile(root/'fit-symbols.u8')
            catalog=[]
            for tick in m.TIMES:
                directory=root/f'time-{tick}/test';directory.mkdir(parents=True)
                symbols.tofile(directory/'symbols.u8')
                for arm in m.ARMS:
                    (directory/f'{arm}-full.u16').write_bytes(b'synthetic-fixture')
                    for view,family,target in m.READOUTS:
                        model=root/'models'/f'{view}-{family}-{target}'
                        model.mkdir(parents=True,exist_ok=True)
                        m.write(model/'model.json',dict(normalizer_sha256=None))
                        catalog.append(dict(seed=3001,tick=tick,arm=arm,view=view,family=family,target=target,model=str(model)))
            m.write(root/'neural-fits.json',catalog)
            controls=[dict(width=w,family=f,target=t,passed=True) for w in (1024,4096) for f,kind,t in calibrate.CONTROLS]
            def fake_score(model,path,symbols):
                if 'anchor' in model.name:return dict(status='invalid_fit',loss=None,accuracy=None,logp_sha256=None,**{'lambda':None})
                return dict(status='valid',loss=1.,accuracy=.25,logp_sha256='synthetic-fixture',**{'lambda':.1})
            with patch.object(m,'REPO',root),patch.object(p,'seedroot',return_value=root),patch.object(p,'model_score',side_effect=fake_score),patch.object(p,'score_calibration',return_value=dict(rows=controls,pass_all=True)):
                rows=p.score_seed(3001)
            self.assertEqual(len(rows),450)
            self.assertEqual(sum(r['status']=='invalid_fit' for r in rows),30)
            result=p.read(root/'scores.json')
            self.assertEqual(len(result['contrasts']),300)
            self.assertTrue(all(r['difference'] is None for r in result['contrasts'] if r['family']=='anchor'))
            self.assertTrue(all(r['difference']==0 for r in result['contrasts'] if r['family']!='anchor'))
            self.assertEqual(len(result['baselines']),4)


if __name__=='__main__':unittest.main()
