"""Sealed NE-0c pipeline: primary experiment first, secondary continuation last."""
import argparse
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime,timezone
import json
from pathlib import Path
import shutil
import subprocess
import sys
import time
import analyse
probe=analyse.probe

BASE=Path(__file__).resolve().parent
REPO=BASE.parent.parent
OLD=BASE.parent/'neural-ecology'
ART=BASE/'artifacts'
BIN=BASE/'target/release/ne0c'
SEAL=BASE/'implementation-seal.json'

def read(p):return json.loads(Path(p).read_text())
def output(*argv):return subprocess.check_output(argv,cwd=REPO,text=True).strip()

def command(argv):
    argv=list(map(str,argv));start=time.monotonic()
    entry={'argv':argv,'utc':datetime.now(timezone.utc).isoformat()}
    ART.mkdir(exist_ok=True)
    with (ART/'attempts.jsonl').open('a') as f:f.write(json.dumps({'event':'start',**entry})+'\n')
    result=subprocess.run(argv,cwd=REPO)
    with (ART/'attempts.jsonl').open('a') as f:
        f.write(json.dumps({'event':'exit',**entry,'returncode':result.returncode,'seconds':time.monotonic()-start})+'\n')
    if result.returncode:raise RuntimeError('step failed; attempt and outputs retained')

def verify():
    s=read(SEAL)
    if probe.sha(BIN)!=s['binary_sha256']:raise ValueError('binary changed')
    for p,h in s['files'].items():
        if probe.sha(REPO/p)!=h:raise ValueError(f'sealed file changed: {p}')
    if subprocess.check_output(['git','show',f'HEAD:{SEAL.relative_to(REPO)}'],cwd=REPO)!=SEAL.read_bytes():raise ValueError('seal is not committed')

def one_seed(seed,dev):
    mode='dev' if dev else 'eval';root=ART/f'{mode}-{seed}'
    for phase in 'ABC':
        summary=root/f'logs/{phase}-summary.json'
        if not summary.exists():
            command([BIN,'phase',seed,phase,root,mode]+([] if dev else [SEAL]))
        s=read(summary)
        if s['seed']!=seed or s['binary_sha256']!=probe.sha(BIN) or s['ticks']!=(50000 if phase=='A' else 10256):
            raise ValueError('existing phase identity/binary/count differs')
        if phase=='A' and dev:
            got=read(root/'checkpoints/final-A.json')
            old=read(BASE.parent/'neural-ecology-ne0b/artifacts/seed-3001-k-4096/A-050000.json')
            if got!=old:raise ValueError('development A differs from selected NE-0b candidate')
            probe.write_json(root/'logs/ne0b-identity.json',{'pass':True,'full_checkpoint_equal':True})
        if phase=='B' and not (root/'checkpoints/probes.json').exists():
            command([sys.executable,OLD/'probe.py','fit',root])
    for variant,phase in [('persistent','B'),('persistent','C'),('restored','B'),('restored','C'),('disabled','A')]:
        if not (root/f'logs/replay-{variant}-{phase}.json').exists():command([BIN,'replay',root,phase,variant])
    command([sys.executable,OLD/'validate_replays.py',root])
    command([sys.executable,OLD/'probe.py','score',root])
    continuation=root/'diagnostics/continuation'
    (continuation/'logs').mkdir(parents=True,exist_ok=True)
    shutil.copyfile(root/'logs/A-summary.json',continuation/'logs/A-summary.json')
    for phase in 'BC':
        summary=continuation/f'logs/{phase}-summary.json'
        if not summary.exists():command([BIN,'continuation',seed,phase,root,mode]+([] if dev else [SEAL]))
        c=read(summary)
        if c['transient_reset'] or c['stream_hash']!=read(root/f'logs/{phase}-summary.json')['stream_hash']:
            raise ValueError('incorrect continuation branch')
        shutil.copyfile(summary,root/f'logs/continuation-{phase}-summary.json')
        if phase=='B' and not (continuation/'checkpoints/probes.json').exists():
            command([sys.executable,OLD/'probe.py','fit',continuation])
    command([sys.executable,OLD/'probe.py','score',continuation])
    shutil.copyfile(continuation/'logs/result.json',root/'logs/continuation-result.json')
    shutil.copyfile(continuation/'checkpoints/probes.json',root/'logs/continuation-probes.json')
    command([sys.executable,OLD/'archive.py',root])
    return root

def seal():
    if SEAL.exists() or list(ART.glob('eval-*')):raise ValueError('refusing reseal or post-evaluation seal')
    dev=read(ART/'dev-report.json')
    if not dev['gates']['G0'] or not dev['gates']['G5']:raise ValueError('validation incomplete/invalid')
    if not read(ART/'dev-3001/logs/ne0b-identity.json')['pass']:raise ValueError('NE-0b mismatch')
    identity=read(ART/'dev-3001/logs/replay-probe-identity.json')
    if not identity['pass'] or len(identity['checks'])!=8:raise ValueError('probe identity missing')
    inherited=read(BASE.parent/'neural-ecology-ne0b/implementation-seal.json')
    files=dict(inherited['files'])
    for p,h in files.items():
        if probe.sha(REPO/p)!=h:raise ValueError(f'inherited sealed file changed: {p}')
    paths=[BASE/'preregistration.md',BASE/'Cargo.lock',BASE/'dependencies.lock',BASE/'Cargo.toml',
        BASE/'run.py',BASE/'analyse.py',BASE/'verification.json',BASE/'tests/test_analysis.py',
        BASE.parent/'neural-ecology-ne0b/implementation-seal.json',ART/'fixtures.json',
        *sorted((BASE/'src').glob('*.rs')),*sorted((ART/'build').iterdir())]
    for p in paths:files[str(p.relative_to(REPO))]=probe.sha(p)
    fixture=read(ART/'fixtures.json')
    if fixture['fixture_hash']!=inherited['fixture']['fixture_hash']:raise ValueError('fixture mismatch')
    probe.write_json(SEAL,{'schema':'NE0c-implementation-seal-v1','utc':datetime.now(timezone.utc).isoformat(),
        'source_commit':output('git','rev-parse','HEAD'),'files':files,'binary_sha256':probe.sha(BIN),
        'protocol_sha256':probe.sha(BASE/'preregistration.md'),'evaluation_seeds':list(range(4001,4011)),
        'cadence':4096,'fixture':fixture,'development_report':dev,
        'development_inventory':{str(p.relative_to(REPO)):probe.sha(p) for p in sorted((ART/'dev-3001/logs').glob('*.json'))},
        'environment':{'python':sys.version,'numpy':probe.np.__version__,'rustc':output('rustc','--version'),
            'macos':output('sw_vers','-productVersion'),'chip':output('sysctl','-n','machdep.cpu.brand_string'),
            'cxx':output('c++','--version')},
        'config':{'adapt':50000,'warm':256,'B':10000,'C':10000,'workers':3,'probe_steps':2000,'probe_lr':.5,'probe_l2':.01,'minimum_effect':.020}})
    print('Seal written; commit before evaluation.',flush=True)

def main():
    p=argparse.ArgumentParser();p.add_argument('mode',choices=['fixtures','dev','seal','eval']);args=p.parse_args()
    if args.mode=='fixtures':command([BIN,'fixtures']);return
    if args.mode=='seal':seal();return
    dev=args.mode=='dev'
    if not dev:verify()
    with ThreadPoolExecutor(max_workers=1 if dev else 3) as pool:
        list(pool.map(lambda s:one_seed(s,dev),[3001] if dev else range(4001,4011)))
    if not dev:verify()
    analyse.report(ART,dev)

if __name__=='__main__':main()
