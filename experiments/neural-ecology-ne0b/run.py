"""Fixed NE-0b development grid; no prediction measurement or evaluation mode."""
import argparse
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import platform
import subprocess
import sys
import time

BASE = Path(__file__).resolve().parent
REPO = BASE.parent.parent
ART = BASE / 'artifacts'
BINARY = BASE / 'target/release/ne0b'
SEEDS = [3001, 3002, 3003]
INTERVALS = [1, 16, 256, 4096]
SEAL = BASE / 'implementation-seal.json'

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def read(path):
    return json.loads(path.read_text())

def write(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2) + '\n')

def movement_audit(row):
    p, y = (row['A']['arms'][a]['movement'] for a in ['Plastic', 'Yoked'])
    result = []
    for sign, a, b in zip([1,-1], p, y):
        positive = all(m['requested'] > 0 and m['realised'] > 0 for m in (a,b))
        gap = abs(a['realised']-b['realised']) / max(1,a['realised'],b['realised'])
        clip = abs(a['clipped']/max(1,a['requested']) - b['clipped']/max(1,b['requested']))
        matched = a['requested_hist'] == b['requested_hist']
        conservation = all(m['requested'] == m['realised']+m['clipped'] for m in (a,b))
        result.append({'sign':sign, 'realised_relative_gap':gap,'clipping_fraction_gap':clip,
                       'pass':positive and matched and conservation and gap <= .05 and clip <= .05})
    return result

def assess(row):
    audits = movement_audit(row)
    checks = {
        'execution': all(row[p]['backend_agreement'] and row[p]['reset_identity'] for p in ['A','R']),
        'activity': all(a['activity']['pass'] for p in ['A','R'] for a in row[p]['arms'].values()),
        'endpoints': all(a['endpoint_pass'] for a in row['A']['arms'].values()),
        'adaptation': row['A']['arms']['Plastic']['adaptation_pass'],
        'movement_match': all(a['pass'] for a in audits),
    }
    return {'checks':checks,'movement_audit':audits,'viable':all(checks.values())}

def select(rows):
    identities = [(r['seed'],r['interval']) for r in rows]
    expected = [(s,k) for k in INTERVALS for s in SEEDS]
    if sorted(identities) != sorted(expected):
        raise ValueError('all 12 unique development runs are required')
    for seed in SEEDS:
        selected = [r for r in rows if r['seed'] == seed]
        for p in ['A','R']:
            for key in ['frozen_history_hash','topology_hash','stream_hash']:
                if len({r[p][key] for r in selected}) != 1:
                    raise ValueError(f'paired Frozen/configuration mismatch {seed}/{p}/{key}')
    enriched = [{**r, **assess(r)} for r in sorted(rows,key=lambda r:(r['interval'],r['seed']))]
    if not all(r['checks']['execution'] for r in enriched):
        raise ValueError('invalid execution')
    viable = [k for k in INTERVALS if all(r['viable'] for r in enriched if r['interval']==k)]
    return {'development_only':True,'complete':True,'n_development_seeds':3,
            'prediction_scores_used':False,'confirmatory_seeds_accessed':False,
            'verdict':'VIABLE_DEVELOPMENT_CANDIDATE' if viable else 'NO_VIABLE_CANDIDATE',
            'selected_interval':min(viable) if viable else None,
            'viable_intervals':viable,'frozen_cross_interval_replay':True,'runs':enriched}

def seal():
    if SEAL.exists() or list(ART.glob('seed-*')):
        raise ValueError('refusing to reseal or seal after development access')
    old_path = REPO/'experiments/neural-ecology/implementation-seal-v2.json'
    old = read(old_path)
    files = dict(old['files'])
    for p,h in files.items():
        if sha(REPO/p) != h: raise ValueError(f'inherited sealed file changed: {p}')
    old_binary=REPO/'experiments/neural-ecology/target/release/ne0'
    if sha(old_binary) != old['binary_sha256']: raise ValueError('old binary changed')
    for p in [old_path, BASE/'plan.md', BASE/'Cargo.lock', BASE/'Cargo.toml',
              BASE/'dependencies.lock',BASE/'src/main.rs',BASE/'run.py',BASE/'tests/test_report.py',
              BASE/'verification.json', ART/'fixtures.json', *sorted((ART/'build').iterdir())]:
        files[str(p.relative_to(REPO))] = sha(p)
    fixture=read(ART/'fixtures.json')
    if fixture['fixture_hash'] != old['fixture']['fixture_hash']:
        raise ValueError('inherited fixtures differ')
    write(SEAL,{'schema':'NE0b-development-seal-v1','utc':datetime.now(timezone.utc).isoformat(),
        'source_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=REPO,text=True).strip(),
        'binary_sha256':sha(BINARY),'inherited_binary_sha256':sha(old_binary),
        'files':files,'seeds':SEEDS,'intervals':INTERVALS,'workers':3,
        'fixture':fixture,'environment':{'python':sys.version,'platform':platform.platform(),
        'rustc':subprocess.check_output(['rustc','--version'],text=True).strip(),
        'cxx':subprocess.check_output(['c++','--version'],text=True).splitlines()[0]},
        'scope':'Development viability only; no predictive acquisition claim.'})
    print(f'Sealed {len(files)} files; commit {SEAL.name} before screen.')

def verify():
    s=read(SEAL)
    if sha(BINARY)!=s['binary_sha256']: raise ValueError('binary changed')
    for p,h in s['files'].items():
        if sha(REPO/p)!=h: raise ValueError(f'sealed file changed: {p}')
    committed=subprocess.check_output(['git','show',f'HEAD:{SEAL.relative_to(REPO)}'],cwd=REPO)
    if committed!=SEAL.read_bytes(): raise ValueError('uncommitted seal')

def screen():
    verify()
    if list(ART.glob('seed-*')): raise ValueError('screen already started; preserve attempts')
    ART.mkdir(exist_ok=True)
    def one(pair):
        seed,k=pair
        out=ART/f'seed-{seed}-k-{k}'
        argv=[str(BINARY),'run',str(seed),str(k),str(out),str(SEAL)]
        start={'seed':seed,'interval':k,'argv':argv,'utc':datetime.now(timezone.utc).isoformat()}
        with (ART/'attempts.jsonl').open('a') as f: f.write(json.dumps({'event':'start',**start})+'\n')
        clock=time.monotonic()
        completed=subprocess.run(argv,cwd=REPO)
        with (ART/'attempts.jsonl').open('a') as f:
            f.write(json.dumps({'event':'exit',**start,'returncode':completed.returncode,'seconds':time.monotonic()-clock})+'\n')
        return out if completed.returncode==0 else None
    pairs=[(s,k) for k in INTERVALS for s in SEEDS]
    with ThreadPoolExecutor(max_workers=3) as pool: paths=list(pool.map(one,pairs))
    verify()
    if any(p is None for p in paths):
        write(ART/'report.json',{'verdict':'INVALID_OR_INCOMPLETE','complete':False})
        raise RuntimeError('execution failed; retained all attempts')
    report=select([read(p/'result.json') for p in paths])
    report['seal_sha256']=sha(SEAL)
    report['binary_sha256']=sha(BINARY)
    report['plan_sha256']=sha(BASE/'plan.md')
    report['artifact_hashes']={str(p.relative_to(BASE)):sha(p) for root in paths for p in sorted(root.iterdir()) if p.is_file()}
    write(ART/'report.json',report)
    print(json.dumps({k:v for k,v in report.items() if k not in ['runs','artifact_hashes']},indent=2))

def main():
    p=argparse.ArgumentParser()
    p.add_argument('command',choices=['fixtures','seal','screen'])
    args=p.parse_args()
    if args.command=='fixtures': subprocess.run([BINARY,'fixtures'],cwd=REPO,check=True)
    elif args.command=='seal': seal()
    else: screen()

if __name__=='__main__': main()
