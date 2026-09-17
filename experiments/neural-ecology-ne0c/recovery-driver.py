import sys,json,shutil,gzip,hashlib
from pathlib import Path
from datetime import datetime,timezone
BASE=Path('/Users/christopherhay/chris-source/cell80/experiments/neural-ecology-ne0c')
sys.path.insert(0,str(BASE));import run
record=BASE/'recovery-progress.json'
progress=json.loads(record.read_text()) if record.exists() else {'script_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'started_utc':datetime.now(timezone.utc).isoformat(),'completed_secondary':[],'partial_replay_checks':[],'completed_remaining_seeds':[]}
def save():record.write_text(json.dumps(progress,indent=2)+'\n')
run.verify();save()
for seed in (4004,4005,4006):
 root=run.ART/f'eval-{seed}'
 if seed in progress['completed_secondary']:continue
 if shutil.disk_usage(BASE).free<700_000_000:raise RuntimeError('Recovery paused before seed: need 700 MB free')
 cont=root/'diagnostics/continuation';(cont/'logs').mkdir(parents=True,exist_ok=True)
 shutil.copyfile(root/'logs/A-summary.json',cont/'logs/A-summary.json')
 for phase in 'BC':
  summary=cont/f'logs/{phase}-summary.json'
  if not summary.exists():run.command([run.BIN,'continuation',seed,phase,root,'eval',run.SEAL])
  c=run.read(summary);assert not c['transient_reset'] and c['stream_hash']==run.read(root/f'logs/{phase}-summary.json')['stream_hash']
  shutil.copyfile(summary,root/f'logs/continuation-{phase}-summary.json')
  if phase=='B':
   old=root/'diagnostics/interrupted-continuation-attempt-1/logs/B-ticks.jsonl.gz'
   raw=gzip.open(old,'rb').read();lines=raw.splitlines(keepends=True);complete=[x for x in lines if x.endswith(b'\n')]
   with (cont/'logs/B-ticks.jsonl').open('rb') as f:
    assert all(f.readline()==line for line in complete),'interrupted prefix differs'
   progress['partial_replay_checks'].append({'seed':seed,'complete_tick_lines':len(complete),'pass':True,'preserved_log_sha256':hashlib.sha256(raw).hexdigest()});save()
   if not (cont/'checkpoints/probes.json').exists():run.command([sys.executable,run.OLD/'probe.py','fit',cont])
 run.command([sys.executable,run.OLD/'probe.py','score',cont])
 shutil.copyfile(cont/'logs/result.json',root/'logs/continuation-result.json')
 shutil.copyfile(cont/'checkpoints/probes.json',root/'logs/continuation-probes.json')
 run.command([sys.executable,run.OLD/'archive.py',root]);progress['completed_secondary'].append(seed);save()
for seed in range(4007,4011):
 if seed in progress['completed_remaining_seeds']:continue
 if shutil.disk_usage(BASE).free<1_100_000_000:
  progress['paused_before_seed']=seed;save();raise RuntimeError('Recovery paused before seed: need 1.1 GB free')
 run.one_seed(seed,False);progress['completed_remaining_seeds'].append(seed);save()
run.verify();run.analyse.report(run.ART,False)
progress['completed_utc']=datetime.now(timezone.utc).isoformat();progress.pop('paused_before_seed',None);save()
