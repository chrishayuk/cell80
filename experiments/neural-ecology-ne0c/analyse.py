"""NE-0c cohort aggregation; inherited probe and registered tests are unchanged."""
import json
from pathlib import Path
import statistics
import sys
BASE=Path(__file__).resolve().parent
sys.path.insert(0,str(BASE.parent/'neural-ecology'))
import probe

def aggregate(results, secondary, development=False):
    results=sorted(results,key=lambda r:r['seed'])
    ids=[r['seed'] for r in results]
    expected=[3001] if development else list(range(4001,4011))
    if ids!=expected or sorted(r['seed'] for r in secondary)!=expected:
        raise ValueError('wrong, incomplete or duplicate cohort')
    if any(r['development']!=development for r in results+secondary):
        raise ValueError('mixed cohorts')
    comparisons={k:probe.contrast([r[k] for r in results]) for k in ['d_F','d_Y','d_M']}
    gates={
        'G0':all(r['gates_per_seed']['G0_execution'] for r in results),
        'G1':all(r['gates_per_seed']['G1_activity'] for r in results),
        'G2':all(r['gates_per_seed']['G2_adaptation'] for r in results),
        'G3':comparisons['d_F']['pass'],
        'G4':comparisons['d_Y']['pass'] and all(r['gates_per_seed']['G4_movement_audit'] for r in results),
        'G5':all(r['gates_per_seed']['G5_persistence'] is True for r in results),
        'G6':comparisons['d_M']['pass'],
    }
    lookup={r['seed']:r for r in results}
    diagnostic=[]
    for r in sorted(secondary,key=lambda r:r['seed']):
        diagnostic.append({'seed':r['seed'],'arms':r['arms'],
            'd_F':r['d_F'],'d_Y':r['d_Y'],'d_M':r['d_M'],
            'continuation_minus_primary':{a:r['arms'][a]['loss']-lookup[r['seed']]['arms'][a]['loss'] for a in probe.ARMS}})
    verdict='DEVELOPMENT_ONLY' if development else 'INVALID' if not gates['G0'] or not gates['G5'] else 'PASS' if all(gates.values()) else 'CRITERIA_NOT_MET'
    return {'verdict':verdict,'development':development,'complete':True,'n':len(results),
        'cadence':4096,'gates':gates,'contrasts':comparisons,'seeds':results,
        'mean_test_loss':{a:statistics.mean(r['arms'][a]['loss'] for r in results) for a in probe.ARMS},
        'continuation':{'secondary_only':True,'affects_primary_verdict':False,'seeds':diagnostic,
            'mean_test_loss':{a:statistics.mean(r['arms'][a]['loss'] for r in secondary) for a in probe.ARMS},
            'mean_d_F':statistics.mean(r['d_F'] for r in secondary),
            'mean_d_Y':statistics.mean(r['d_Y'] for r in secondary)}}

def report(root, development):
    root=Path(root);seeds=[3001] if development else list(range(4001,4011));mode='dev' if development else 'eval'
    paths=[root/f'{mode}-{s}' for s in seeds]
    for p in paths:
        for phase in 'BC':
            c=json.loads((p/f'logs/continuation-{phase}-summary.json').read_text())
            a=json.loads((p/f'logs/{phase}-summary.json').read_text())
            if c['transient_reset'] or not c['backend_agreement'] or not c['metal_replay'] or c['stream_hash']!=a['stream_hash']:
                raise ValueError('continuation execution/stream check failed')
    result=aggregate([json.loads((p/'logs/result.json').read_text()) for p in paths],
        [json.loads((p/'logs/continuation-result.json').read_text()) for p in paths],development)
    probe.write_json(root/f'{mode}-report.json',result)
    print(json.dumps({k:v for k,v in result.items() if k not in ['seeds','continuation']}),flush=True)
    return result
