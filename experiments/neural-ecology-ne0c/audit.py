import hashlib,json,math,statistics,struct,gzip,io,os
for key in ("OMP_NUM_THREADS","OPENBLAS_NUM_THREADS","MKL_NUM_THREADS","VECLIB_MAXIMUM_THREADS","NUMEXPR_NUM_THREADS"):os.environ[key]="1"
import numpy as np
from threadpoolctl import threadpool_limits
_threads=threadpool_limits(limits=1)
from collections import Counter
from datetime import datetime,timezone
from pathlib import Path

b=Path('experiments/neural-ecology-ne0c');root=b/'artifacts'
def read(p):return json.loads(p.read_text())
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
d=read(root/'eval-report.json');seal=read(b/'implementation-seal.json')
assert d['complete'] and not d['development'] and [r['seed'] for r in d['seeds']]==list(range(4001,4011))
for p,h in seal['files'].items():assert sha(Path(p))==h,p
assert sha(b/'target/release/ne0c')==seal['binary_sha256']
assert sha(b/'preregistration.md')==seal['protocol_sha256']
zero=hashlib.sha256(bytes(1024*6)).hexdigest()
def bytes_from(path):
 return path.read_bytes() if path.exists() else gzip.open(str(path)+'.gz','rb').read()
def check_prediction(path,expected,manifest):
 symbols=np.frombuffer(bytes_from(path/'datasets/C-symbols.u8'),dtype=np.uint8)
 assert len(symbols)==10257 and np.all(symbols<4)
 current=symbols[256:10256].astype(np.int64);target=symbols[257:10257].astype(np.int64)
 for arm,row in expected['arms'].items():
  features=np.zeros((10000,1028),dtype=np.float64)
  if arm!='Memoryless':
   traces=np.frombuffer(bytes_from(path/f'datasets/C-{arm}-q.u16'),dtype='<u2')
   assert traces.shape==(10240000,) and np.all(traces<=255)
   features[:,:1024]=traces.reshape(10000,1024)/(255.0*32.0)
  features[np.arange(10000),1024+current]=1
  with np.load(io.BytesIO(bytes_from(path/'checkpoints'/manifest['arms'][arm]['coefficient_file'])),allow_pickle=False) as model:
   scores=features@model['w']+model['b']
  scores-=scores.max(axis=1,keepdims=True)
  logp=scores-np.log(np.exp(scores).sum(axis=1,keepdims=True))
  assert math.isclose(float(-logp[np.arange(10000),target].mean()),row['loss'],abs_tol=1e-14)
  assert float((logp.argmax(axis=1)==target).mean())==row['accuracy']
  assert hashlib.sha256(logp.astype('<f8').tobytes()).hexdigest()==row['logp_sha256']
 oracle=(symbols[255:10255].astype(np.int64)+current)%4
 assert math.isclose(float(-np.log(np.where(target==oracle,.7,.1)).mean()),expected['oracle_loss'],abs_tol=1e-14)
 assert float((target==oracle).mean())==expected['oracle_accuracy']
details=[];evidence={}
for r in d['seeds']:
 p=root/f"eval-{r['seed']}"
 phases={x:read(p/f'logs/{x}-summary.json') for x in 'ABC'}
 secondary={x:read(p/f'logs/continuation-{x}-summary.json') for x in 'BC'}
 full=read(p/'checkpoints/final-A.json')
 for net in full['networks']:
  assert len(net['weights'])==16384 and all(0<=w<=32 for w in net['weights'])
  assert len(net['nodes'])==1024 and all(0<=n['v']<=255 and 0<=n['q']<=255 and n['r'] in [0,1] and n['s'] in [0,1] for n in net['nodes'])
 a_hashes=[hashlib.sha256(b''.join(struct.pack('<HBBH',n['v'],n['r'],n['s'],n['q']) for n in net['nodes'])).hexdigest() for net in full['networks']]
 assert any(h!=zero for h in a_hashes)
 for phase,n in [('A',50000),('B',10256),('C',10256)]:
  a=phases[phase];assert a['seed']==r['seed'] and a['ticks']==n and not a['development']
  assert a['cadence']==4096 and a['update_opportunities']==(12 if phase=='A' else 0)
  assert a['backend_agreement'] and a['metal_replay'] and a['reset_identity'] and a['transient_reset']
  assert a['binary_sha256']==seal['binary_sha256'] and a['initial_node_hashes']==[zero]*3
  for arm,v in a['arms'].items():
   assert v['persistent_hash']==phases['A']['arms'][arm]['persistent_hash']
   assert v['activity']['ticks']==10000
   if phase!='A':assert all(m['requested']==m['realised']==m['clipped']==0 for m in v['movement'])
 for phase,c in secondary.items():
  assert c['ticks']==10256 and c['seed']==r['seed'] and not c['development']
  assert not c['transient_reset'] and not c['reset_identity'] and c['initial_node_hashes']==a_hashes
  assert c['backend_agreement'] and c['metal_replay'] and c['update_opportunities']==0
  assert c['binary_sha256']==seal['binary_sha256'] and c['stream_hash']==phases[phase]['stream_hash']
  for arm,v in c['arms'].items():
   assert v['persistent_hash']==phases['A']['arms'][arm]['persistent_hash']
   assert all(m['requested']==m['realised']==m['clipped']==0 for m in v['movement'])
 identity=read(p/'logs/replay-probe-identity.json');persist=read(p/'logs/persistence.json')
 assert identity['pass'] and len(identity['checks'])==8 and all(q['pass'] for q in identity['checks'])
 assert persist['pass'] and persist['probe_identity']
 for sign,(a,y) in enumerate(zip(phases['A']['arms']['Plastic']['movement'],phases['A']['arms']['Yoked']['movement'])):
  assert a['requested_hist']==y['requested_hist']
  assert all(v['requested']==v['realised']+v['clipped'] for v in [a,y])
  positive=min(a['requested'],a['realised'],y['requested'],y['realised'])>0
  gap=abs(a['realised']-y['realised'])/max(1,a['realised'],y['realised'])
  clip=abs(a['clipped']/max(1,a['requested'])-y['clipped']/max(1,y['requested']))
  observed=r['movement_audit'][sign]
  assert observed['pass']==(positive and gap<=.05 and clip<=.05)
  assert math.isclose(observed['realised_relative_gap'],gap,abs_tol=1e-15)
  assert math.isclose(observed['clipping_fraction_gap'],clip,abs_tol=1e-15)
 for variant,phase in [('persistent','B'),('persistent','C'),('restored','B'),('restored','C'),('disabled','A')]:
  q=read(p/f'logs/replay-{variant}-{phase}.json')
  assert q['fresh_process'] and q['pass'] and q['compared_ticks']==(50000 if phase=='A' else 10256)
 archive=read(p/'logs/archive.json')
 for name,v in archive.items():
  q=p/(name+'.gz');assert q.stat().st_size==v['gzip_bytes'] and sha(q)==v['gzip_sha256']
 probes=read(p/'checkpoints/probes.json');contprobes=read(p/'logs/continuation-probes.json')
 for arm,q in probes['arms'].items():
  assert q['steps']==2000 and q['learning_rate']==.5 and q['l2']==.01
  assert q['final_objective']<=q['initial_objective']+1e-10 and math.isfinite(q['final_gradient_norm'])
  assert sha(p/'checkpoints'/q['coefficient_file'])==q['coefficient_sha256']
  assert sha(p/'datasets/B-symbols.u8')==q['B_symbols_sha256']
  if arm!='Memoryless':assert sha(p/f'datasets/B-{arm}-q.u16')==q['B_features_sha256']
 for arm,q in contprobes['arms'].items():
  assert q['steps']==2000 and q['learning_rate']==.5 and q['l2']==.01
  assert q['final_objective']<=q['initial_objective']+1e-10 and math.isfinite(q['final_gradient_norm'])
  assert archive['diagnostics/continuation/checkpoints/'+q['coefficient_file']]['original_sha256']==q['coefficient_sha256']
  assert q['B_symbols_sha256']==probes['arms'][arm]['B_symbols_sha256']
  if arm!='Memoryless':assert archive[f'diagnostics/continuation/datasets/B-{arm}-q.u16']['original_sha256']==q['B_features_sha256']
 check_prediction(p,r,probes)
 check_prediction(p/'diagnostics/continuation',read(p/'logs/continuation-result.json'),contprobes)
 for q in (p/'logs').glob('*.json'):evidence[str(q.relative_to(b))]=sha(q)
 evidence[str((p/'checkpoints/probes.json').relative_to(b))]=sha(p/'checkpoints/probes.json')
 details.append({'seed':r['seed'],'phases':phases,'continuation_phases':secondary,
  'probes':probes,'continuation_probes':contprobes,'persistence':persist,'probe_identity':identity,
  'final_A_node_hashes':a_hashes,'archive_files':len(archive)})

independent={}
for key,arm in [('d_F','Frozen'),('d_Y','Yoked'),('d_M',None)]:
 vals=[(r['arms'][arm]['loss'] if arm else math.log(4))-r['arms']['Plastic']['loss'] for r in d['seeds']]
 k=sum(v>0 for v in vals);p=sum(math.comb(10,j) for j in range(k,11))/1024
 mean=statistics.mean(vals)
 got={'values':vals,'mean':mean,'median':statistics.median(vals),'sample_sd':statistics.stdev(vals),
      'positive_seeds':k,'p_raw':p,'p_bonferroni':min(1,3*p),'pass':3*p<=.05 and mean>=.020}
 expected=d['contrasts'][key]
 for q in ['mean','median','sample_sd','p_raw','p_bonferroni']:assert math.isclose(got[q],expected[q],abs_tol=1e-15,rel_tol=1e-12)
 assert vals==expected['values'] and k==expected['positive_seeds'] and got['pass']==expected['pass']
 independent[key]=got
gates={'G0':True,'G5':True,
 'G1':all(.01<=a['activity']['spike_fraction']<=.35 and a['activity']['active_unit_fraction']>=.5 for q in details for p in 'ABC' for a in q['phases'][p]['arms'].values()) and all(a['endpoint_fraction']<=.25 for q in details for a in q['phases']['A']['arms'].values()),
 'G2':all(q['phases']['A']['arms']['Plastic']['changed_fraction']>=.01 and q['phases']['A']['arms']['Plastic']['mean_absolute_change']>=.1 for q in details),
 'G3':independent['d_F']['pass'],'G4':independent['d_Y']['pass'] and all(m['pass'] for r in d['seeds'] for m in r['movement_audit']),
 'G6':independent['d_M']['pass']}
assert gates==d['gates']
events=[json.loads(x) for x in (root/'attempts.jsonl').read_text().splitlines()]
events=[e for e in events if any('/eval-' in a for a in e['argv'])]
starts=[e for e in events if e['event']=='start'];exits=[e for e in events if e['event']=='exit']
interruption=read(b/'interruption-record.json');recovery=read(b/'recovery-progress.json')
successful=[e for e in exits if e['returncode']==0];failed=[e for e in exits if e['returncode']!=0]
assert len(successful)==160 and len(failed)==3
assert all(e['returncode']==1 and e['argv'][1]=='continuation' and int(e['argv'][2]) in (4004,4005,4006) and e['argv'][3]=='B' for e in failed)
assert all(v==1 for v in Counter(tuple(e['argv']) for e in successful).values())
exit_keys={(e['utc'],tuple(e['argv'])) for e in exits}
unmatched=[e for e in starts if (e['utc'],tuple(e['argv'])) not in exit_keys]
assert unmatched==interruption['unmatched_starts'] and len(unmatched)==3
assert len(starts)==166 and len(exits)==163
assert Counter(tuple(e['argv']) for e in starts)==Counter(tuple(e['argv']) for e in exits+unmatched)
assert sha(root/'recovery-driver.py')==recovery['script_sha256']
assert recovery['completed_secondary']==[4004,4005,4006] and recovery['completed_remaining_seeds']==list(range(4007,4011))
assert len(recovery['partial_replay_checks'])==3 and all(x['pass'] for x in recovery['partial_replay_checks'])
for seed,files in interruption['preserved_partial_continuation'].items():
 seedroot=root/f'eval-{seed}'
 interrupted=bytes_from(seedroot/'diagnostics/interrupted-continuation-attempt-1/logs/B-ticks.jsonl')
 prefix=interrupted[:interrupted.rfind(b'\n')+1]
 assert prefix and bytes_from(seedroot/'diagnostics/continuation/logs/B-ticks.jsonl').startswith(prefix)
 recorded=next(x for x in recovery['partial_replay_checks'] if x['seed']==int(seed))
 assert recorded['complete_tick_lines']==prefix.count(b'\n')
 archive=read(root/f'eval-{seed}/logs/archive.json')
 for name,v in files.items():
  saved=archive['diagnostics/interrupted-continuation-attempt-1/'+name]
  assert saved['original_sha256']==v['sha256'] and saved['original_bytes']==v['bytes']

d['provenance']={'protocol_sha256':seal['protocol_sha256'],'seal_sha256':sha(b/'implementation-seal.json'),
 'binary_sha256':seal['binary_sha256'],'source_commit':seal['source_commit'],'environment':seal['environment'],
 'raw_report_sha256':sha(root/'eval-report.json'),'attempts_sha256':sha(root/'attempts.jsonl'),
 'artifact_summary_sha256':evidence}
d['independent_audit']={'pass':True,'utc':datetime.now(timezone.utc).isoformat(),'files_verified':len(seal['files']),
 'primary_ticks_per_recurrent_arm':705120,'continuation_ticks_per_recurrent_arm':205120,
 'independently_rescored_primary_and_secondary_predictions':800000,'fresh_process_replays':50,'frozen_probe_identity_checks':80,'successful_evaluation_steps':160,
 'retried_incomplete_continuations':3,'unmatched_starts_at_interruption':3,'interruption':interruption,'recovery':recovery,'independent_contrasts':independent,'independent_gates':gates,
 'continuation_initial_nodes_match_final_A':True,'primary_initial_nodes_zero':True,
 'archive_scope':'Compressed bytes independently hashed; decompression hashes verified by the inherited archiver.'}
d['diagnostics']=details
(b/'evaluation-results.json').write_text(json.dumps(d,indent=2)+'\n')
print(json.dumps({k:d[k] for k in ['verdict','gates','mean_test_loss','contrasts']},indent=2))
print('Continuation means',d['continuation']['mean_test_loss'])
print('Audit passed.')
