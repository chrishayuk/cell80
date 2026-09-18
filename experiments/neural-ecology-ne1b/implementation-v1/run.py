"""Frozen NE-1b orchestration, with global pre-test fit barrier."""
import argparse
from concurrent.futures import ProcessPoolExecutor
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import multiprocessing
from pathlib import Path
import platform
import statistics
import subprocess
import sys

BASE = Path(__file__).resolve().parent
REPO = BASE.parent.parent
OLD = BASE.parent / 'neural-ecology-ne1'
sys.path.insert(0, str(BASE))
import select_interventions as selector
sys.path.insert(0, str(OLD))
import measurement as m
import calibrate as cal
np = m.np
ART = BASE / 'artifacts'
BIN = BASE / 'target/release/ne1b'
READOUTS = [('G', 'lag2'), ('C', 'next')]


def read(path): return json.loads(Path(path).read_text())
def sha(path): return m.sha(Path(path))
def relative(path): return str(Path(path).relative_to(REPO))
def cohort(seed): return 'dev' if seed == 3001 else 'eval'
def root(seed): return ART / f'{cohort(seed)}-{seed}'
def oldroot(seed): return OLD / f'artifacts/{cohort(seed)}-{seed}'
def stageof(seed): return 'development' if seed == 3001 else 'evaluation'
def sha_bytes(data): return hashlib.sha256(data).hexdigest()


def write_new(path, value):
    path = Path(path)
    if path.exists():
        assert read(path) == value, f'refusing to replace {path}'
    else:
        m.write(path, value)


def committed(path):
    assert subprocess.check_output(['git', 'show', f'HEAD:{relative(path)}'], cwd=REPO) == path.read_bytes()


def verify_spec():
    m.verify_spec()
    for name, expected in read(BASE / 'specification-seal.json')['files'].items():
        assert sha(REPO / name) == expected, name


def verify_code(stage):
    verify_spec()
    path = BASE / f'{stage}-seal.json'
    seal = read(path)
    assert seal['stage'] == stage and sha(BIN) == seal['binary_sha256']
    assert seal['environment']['python'] == sys.version
    for name, expected in seal['files'].items():
        assert sha(REPO / name) == expected, name
    return seal


def command(argv, log):
    log.parent.mkdir(parents=True, exist_ok=True)
    argv = list(map(str, argv))
    with log.open('a') as f:
        f.write(json.dumps(dict(event='start', utc=datetime.now(timezone.utc).isoformat(), argv=argv))+'\n')
    result = subprocess.run(argv, cwd=REPO)
    with log.open('a') as f:
        f.write(json.dumps(dict(event='exit', utc=datetime.now(timezone.utc).isoformat(), returncode=result.returncode, argv=argv))+'\n')
    if result.returncode:
        raise RuntimeError(f'command failed ({result.returncode}); attempt preserved')


def calibration(seed, refit=False):
    symbols = {s:m.grammar(seed,s) for s in ('fit','validation','test')}
    current = {s:m.labels(x,'current') for s,x in symbols.items()}
    assert all(m.groups_ok(c) for c in current.values())
    oracle = float(-np.log(np.where(m.labels(symbols['test'],'next') ==
                    (m.labels(symbols['test'],'lag1')+current['test'])%4,.7,.1)).mean())
    directory = ART/'calibration-dev' if refit else oldroot(seed)/'calibration'
    if refit:
        if directory.exists(): raise ValueError('calibration attempt already exists')
        directory.mkdir(parents=True)
        m.write(directory/'code.json',dict(files={relative(p):sha(p) for p in
            [BASE/'run.py',OLD/'measurement.py',OLD/'calibrate.py',m.OLD/'probe.py']},python=sys.version))
        fits=[]
        for family,kind,target in [('G','copy','lag2'),('C','previous','next')]:
            name=f'1024-{family}-{kind}-{target}'; out=directory/name
            raw={s:cal.control_data(symbols[s],1024,kind,target) for s in ('fit','validation')}
            norm=m.normalizer(raw['fit'])
            fit=m.fit_grid(m.transform(raw['fit'],norm),m.labels(symbols['fit'],target),current['fit'],
                           m.transform(raw['validation'],norm),m.labels(symbols['validation'],target),current['validation'],family,out)
            np.savez(out/'normalizer.npz',**norm)
            fits.append(dict(name=name,family=family,kind=kind,target=target,fit=fit,
                             normalizer_sha256=sha(out/'normalizer.npz')))
        m.write(directory/'fits.json',fits)
    else:
        fits=[f for f in read(directory/'fits.json') if f['width']==1024 and
              (f['family'],f['kind'],f['target']) in [('G','copy','lag2'),('C','previous','next')]]
    assert len(fits)==2
    rows=[]
    for record in fits:
        out=directory/record['name']; fit=record['fit']
        row=dict(name=record['name'],family=record['family'],target=record['target'],passed=False,
                 loss=None,accuracy=None,logp_sha256=None)
        if fit['selected_index'] is not None:
            chosen=fit['candidates'][fit['selected_index']]
            assert sha(out/'normalizer.npz')==record['normalizer_sha256']
            assert sha(out/chosen['coefficient_file'])==chosen['coefficient_sha256']
            with np.load(out/'normalizer.npz',allow_pickle=False) as n:
                x=m.transform(cal.control_data(symbols['test'],1024,record['kind'],record['target']),n)
            with np.load(out/chosen['coefficient_file'],allow_pickle=False) as z:
                lp=m.log_probabilities(x,current['test'],z['w'],z['b'],record['family'])
            row.update(m.score(lp,m.labels(symbols['test'],record['target'])))
            row['passed']=bool(row['accuracy']>=.999 and row['loss']<=.010) if record['kind']=='copy' else bool(row['accuracy']>=.65 and row['loss']-oracle<=.010)
            if not refit:
                old=next(r for r in read(directory/'result.json')['rows'] if r['name']==record['name'])
                assert all(row[k]==old[k] for k in ('loss','accuracy','logp_sha256','passed'))
        rows.append(row)
    result=dict(seed=seed,rows=rows,pass_all=all(r['passed'] for r in rows),oracle_loss=oracle)
    write_new((directory if refit else root(seed))/'calibration.json',result)
    return result


def make_seal(stage):
    verify_spec()
    destination=BASE/f'{stage}-seal.json'
    if destination.exists(): raise ValueError('seal already exists')
    assert read(ART/'calibration-dev/calibration.json')['pass_all']
    fixtures=read(ART/'fixtures.json')
    assert fixtures['generalized_tick_identity']
    assert fixtures['inherited']['fixture_hash']==read(OLD/'artifacts/fixtures.json')['fixture_hash']
    inherited=read(OLD/'evaluation-seal.json')
    files=dict(inherited['files'])
    files.update(read(BASE/'selection-inventory.json')['source_files'])
    paths=[*BASE.glob('*.py'),*BASE.glob('src/*.rs'),*BASE.glob('tests/*.py'),
           BASE/'Cargo.toml',BASE/'Cargo.lock',BASE/'dependencies.lock',BASE/'intervention-spec.md',
           BASE/'intervention-spec.sha256',BASE/'specification-seal.json',BASE/'selection-inventory.json',
           ART/'fixtures.json',*sorted((ART/'calibration-dev').rglob('*'))]
    if stage=='evaluation':
        d=read(BASE/'development-results.json'); a=read(BASE/'development-independent-audit.json')
        assert d['complete'] and d['integrity_pass'] and d['calibration_pass'] and d['valid_rows']==284 and a['passed']
        assert not list(ART.glob('eval-*'))
        paths += [BASE/'development-seal.json',BASE/'development-results.json',BASE/'development-independent-audit.json']
    else:
        assert not (ART/'dev-3001').exists()
    for path in paths:
        if path.is_file(): files[relative(path)]=sha(path)
    # Capture all local compiled dependency sources, including pre-existing edits.
    for name in ('cell80-core','rustz80','rustmsl'):
        for path in (REPO/name).rglob('*'):
            if 'target' not in path.parts and path.is_file() and (path.suffix=='.rs' or path.name=='Cargo.toml'):
                files[relative(path)]=sha(path)
    for name, expected in inherited['files'].items(): assert sha(REPO/name)==expected,name
    m.write(destination,dict(schema='NE1b-code-seal-v1',stage=stage,files=files,binary_sha256=sha(BIN),
        specification_seal_sha256=sha(BASE/'specification-seal.json'),
        utc=datetime.now(timezone.utc).isoformat(),
        source_commit=subprocess.check_output(['git','rev-parse','HEAD'],text=True,cwd=REPO).strip(),
        environment=dict(python=sys.version,platform=platform.platform(),numpy=np.__version__,
            scipy=m.importlib.metadata.version('scipy'),threadpoolctl=m.importlib.metadata.version('threadpoolctl'),
            threadpools=m.anchor.threadpool_info())))
    print(f'Created {destination}; commit before neural execution.',flush=True)


def prepare(seed):
    r=root(seed); r.mkdir(parents=True,exist_ok=True)
    start=read(oldroot(seed)/'weights-0.json'); end=read(oldroot(seed)/'weights-50000.json')
    initial=start['weights'][0]; sources=end['topology']['sources']
    systems=[dict(id='Frozen',arm='Frozen',branch='unchanged',weights=initial,edges=[])]
    inventory=read(BASE/'selection-inventory.json')
    for a,arm in enumerate(('Plastic','Yoked'),1):
        expected=next(s for s in inventory['systems'] if s['seed']==seed and s['arm']==arm)
        branches=selector.select(seed,sources,initial,end['weights'][a])
        assert selector.diagnose(branches,sources,initial,end['weights'][a])==expected['branches']
        for branch in branches:
            weights=list(end['weights'][a])
            for e in branch['edges']: weights[e]=initial[e]
            systems.append(dict(id=f"{arm}-{branch['branch']}",arm=arm,weights=weights,**branch))
    assert len(systems)==71
    write_new(r/'systems.json',dict(seed=seed,topology=end['topology'],systems=systems))
    return systems


def inherited_replays(seed,split):
    result={}
    for tick,arms in ((0,('Frozen',)),(50000,('Plastic','Yoked'))):
        directory=oldroot(seed)/f'time-{tick}'/split
        hashes=[hashlib.sha256() for _ in range(3)]
        count=0
        with (directory/'ticks.jsonl').open() as f:
            for line in f:
                row=json.loads(line); assert row['tick']==count; count+=1
                for h,x in zip(hashes,row['node_hashes']): h.update(x.encode())
        assert count==10256
        summary=read(directory/'summary.json')
        for arm in arms:
            result[arm]=dict(full=summary['feature_sha256'][f'{arm}-full.u16'],
                             replay=hashes[('Frozen','Plastic','Yoked').index(arm)].hexdigest())
    return result


def check_trajectory(seed,split):
    r=root(seed); out=r/split; s=read(out/'summary.json'); systems=read(r/'systems.json')['systems']
    assert s['seed']==seed and s['split']==split and s['rows']==10000 and s['ticks']==10256
    assert all(s[k] for k in ('reset','backend_agreement','metal_repeat','weights_unchanged','plasticity_disabled'))
    assert s['requested_delta_nonzero']==0 and s['input_sha256']==sha(r/'systems.json')
    assert s['binary_sha256']==read(BASE/f'{stageof(seed)}-seal.json')['binary_sha256']
    assert sha(out/'symbols.u8')==s['stream_sha256']==sha_bytes(m.grammar(seed,split).tobytes())
    assert sha(out/'ticks.jsonl')==s['ticks_sha256']
    inherited=inherited_replays(seed,split)
    assert len(s['systems'])==71
    for system,saved in zip(systems,s['systems']):
        assert saved['id']==system['id']
        assert saved['weight_sha256']==selector.array_hash(system['weights'],'H')
        path=out/f"{system['id']}-q.u16"
        assert path.stat().st_size==20480000 and sha(path)==saved['q_sha256']
        reference='Frozen' if system['arm']=='Frozen' or system['branch']=='all' else system['arm'] if system['branch']=='unchanged' else None
        if reference:
            assert saved['full_sha256']==inherited[reference]['full'],system['id']
            assert saved['replay_sha256']==inherited[reference]['replay'],system['id']
    return s


def collect_job(job):
    seed,split=job; stage=stageof(seed); verify_code(stage)
    out=root(seed)/split
    failure=root(seed)/f'{split}-failure.json'
    if failure.exists(): return False
    try:
        if not (out/'summary.json').exists():
            if out.exists(): raise ValueError('incomplete attempt; explicit logged recovery required')
            command([BIN,'measure',root(seed)/'systems.json',split,out,BASE/f'{stage}-seal.json'],root(seed)/'attempts.jsonl')
        check_trajectory(seed,split)
        return True
    except Exception as error:
        write_new(failure,dict(seed=seed,split=split,status='invalid_trajectory',error=str(error)))
        print(json.dumps(dict(trajectory_failed=job,error=str(error))),flush=True)
        return False


def raw(seed,split,system):
    x=np.fromfile(root(seed)/split/f'{system}-q.u16',dtype='<u2').reshape(10000,1024)
    assert np.all(x<=255)
    return x.astype(np.float64)


def fit_job(job):
    seed,system,key=job; r=root(seed); model=r/'models'/key
    if (model/'complete.json').exists(): return relative(model)
    if model.exists(): raise ValueError('incomplete model attempt retained')
    symbols={s:np.fromfile(r/s/'symbols.u8',dtype=np.uint8) for s in ('fit','validation')}
    current={s:m.labels(v,'current') for s,v in symbols.items()}
    assert all(m.groups_ok(c) for c in current.values())
    data={s:raw(seed,s,system) for s in symbols}; norm=m.normalizer(data['fit'])
    x={s:m.transform(v,norm) for s,v in data.items()}
    model.mkdir(parents=True); np.savez(model/'normalizer.npz',**norm)
    moments=dict(mean=data['fit'].mean(axis=0).tolist(),sigma=norm['sigma'].tolist(),
                 constant_count=int(norm['constant'].sum()))
    m.write(model/'feature-context.json',moments)
    for family,target in READOUTS:
        out=model/f'{family}-{target}'
        fit=m.fit_grid(x['fit'],m.labels(symbols['fit'],target),current['fit'],
                       x['validation'],m.labels(symbols['validation'],target),current['validation'],family,out)
        m.write(out/'model.json',dict(seed=seed,system=system,family=family,target=target,view='q',fit=fit,
                                   normalizer=relative(model/'normalizer.npz'),normalizer_sha256=sha(model/'normalizer.npz')))
    m.write(model/'complete.json',dict(seed=seed,system=system,key=key))
    return relative(model)


def original_model(seed,arm,family,target):
    tick=0 if arm=='Frozen' else 50000
    cell=next(r for r in read(oldroot(seed)/'neural-fits.json') if
              r['tick']==tick and r['arm']==arm and r['view']=='q' and r['family']==family and r['target']==target)
    return REPO/cell['model']


def check_fit_identity(seed,catalog):
    checked=0
    for cell in catalog:
        arm,branch=cell['arm'],cell['branch']
        if not (arm=='Frozen' or branch in ('unchanged','all')): continue
        reference='Frozen' if arm=='Frozen' or branch=='all' else arm
        old=original_model(seed,reference,cell['family'],cell['target']); new=REPO/cell['model']
        a,b=read(old/'model.json'),read(new/'model.json')
        with np.load(old/'normalizer.npz') as x,np.load(REPO/b['normalizer']) as y:
            assert all(np.array_equal(x[k],y[k]) for k in x.files)
        assert a['fit']['selected_index']==b['fit']['selected_index']
        for ca,cb in zip(a['fit']['candidates'],b['fit']['candidates']):
            assert ca['valid']==cb['valid'] and ca['validation_loss']==cb['validation_loss']
            with np.load(old/ca['coefficient_file']) as x,np.load(new/cb['coefficient_file']) as y:
                assert all(np.array_equal(x[k],y[k]) for k in x.files)
        checked+=1
    assert checked==10
    return checked


def fit_all(seeds):
    jobs=[]; catalogs={}
    for seed in seeds:
        systems=read(root(seed)/'systems.json')['systems']; catalog=[]
        valid=all((root(seed)/s/'summary.json').exists() and not (root(seed)/f'{s}-failure.json').exists() for s in ('fit','validation'))
        summaries={s:{x['id']:x for x in read(root(seed)/s/'summary.json')['systems']} for s in ('fit','validation')} if valid else {}
        keys=set()
        for system in systems:
            key=sha_bytes(json.dumps([seed,*[summaries[s][system['id']]['q_sha256'] for s in ('fit','validation')]]).encode()) if valid else None
            if key is not None and key not in keys:
                jobs.append((seed,system['id'],key)); keys.add(key)
            for family,target in READOUTS:
                catalog.append(dict(seed=seed,system=system['id'],arm=system['arm'],branch=system['branch'],
                    family=family,target=target,model=relative(root(seed)/'models'/key/f'{family}-{target}') if valid else None,
                    status='fit_pending' if valid else 'invalid_trajectory'))
        catalogs[seed]=catalog
    with ProcessPoolExecutor(max_workers=8,mp_context=multiprocessing.get_context('spawn')) as pool:
        for _ in pool.map(fit_job,jobs): pass
    for seed,catalog in catalogs.items():
        for cell in catalog:
            if cell['model'] is not None: cell['status']=read(REPO/cell['model']/'model.json')['fit']['status']
        if all(c['model'] is not None for c in catalog): check_fit_identity(seed,catalog)
        write_new(root(seed)/'fits.json',catalog)


def fit_manifest(stage,seeds):
    path=ART/f'{stage}_fit_manifest.json'
    if path.exists():
        value=read(path)
        for name,expected in value['files'].items(): assert sha(REPO/name)==expected,name
        return value
    files={}
    for seed in seeds:
        assert len(read(root(seed)/'fits.json'))==142
        assert not (root(seed)/'test').exists()
        for p in sorted(root(seed).rglob('*')):
            if p.is_file(): files[relative(p)]=sha(p)
    value=dict(all_fits_complete=True,seeds=seeds,code_seal_sha256=sha(BASE/f'{stage}-seal.json'),files=files)
    write_new(path,value)
    return value


def score_model(path,data,symbols):
    record=read(path/'model.json'); fit=record['fit']
    if fit['selected_index'] is None:
        return dict(status='invalid_fit',loss=None,accuracy=None,logp_sha256=None,**{'lambda':None})
    normpath=REPO/record['normalizer'] if 'normalizer' in record else path/'normalizer.npz'
    assert sha(normpath)==record['normalizer_sha256']
    with np.load(normpath,allow_pickle=False) as n: x=m.transform(data,n)
    chosen=fit['candidates'][fit['selected_index']]
    assert sha(path/chosen['coefficient_file'])==chosen['coefficient_sha256']
    with np.load(path/chosen['coefficient_file'],allow_pickle=False) as z:
        lp=m.log_probabilities(x,m.labels(symbols,'current'),z['w'],z['b'],record['family'])
    return dict(status='valid',**m.score(lp,m.labels(symbols,record['target'])),**{'lambda':chosen['lambda']})


def contrasts(rows):
    table={(r['arm'],r['branch'],r['family'],r['target'],r['mode']):r['loss'] for r in rows}
    result=[]
    branches=[r['branch'] for r in rows if r['arm']=='Plastic' and r['family']=='G' and r['mode']=='refitted' and r['branch']!='unchanged']
    assert len(branches)==34
    def difference(a,b): return None if a is None or b is None else a-b
    for family,target in READOUTS:
        effects={}; top_random={}
        for mode in ('refitted','transported'):
            for arm in ('Plastic','Yoked'):
                unchanged=table[arm,'unchanged',family,target,mode]
                result.append(dict(kind='baseline',arm=arm,branch='all',family=family,target=target,mode=mode,
                                   difference=difference(table['Frozen','unchanged',family,target,mode],unchanged)))
                for branch in branches:
                    d=difference(table[arm,branch,family,target,mode],unchanged)
                    effects[arm,branch,mode]=d
                    result.append(dict(kind='reversion',arm=arm,branch=branch,family=family,target=target,mode=mode,difference=d))
                for control in ('count','movement'):
                    for pct in (1,5,10):
                        values=[effects[arm,f'{control}-{pct}-{rep}',mode] for rep in range(5)]
                        avg=statistics.mean(values) if all(v is not None for v in values) else None
                        d=difference(effects[arm,f'top-{pct}',mode],avg)
                        key=f'{control}-{pct}'; top_random[arm,key,mode]=d
                        result.append(dict(kind='top_minus_random',arm=arm,branch=key,family=family,target=target,mode=mode,difference=d))
            for branch in branches:
                result.append(dict(kind='specificity',arm='Plastic-Yoked',branch=branch,family=family,target=target,mode=mode,
                                   difference=difference(effects['Plastic',branch,mode],effects['Yoked',branch,mode])))
            for control in ('count','movement'):
                for pct in (1,5,10):
                    key=f'{control}-{pct}'
                    result.append(dict(kind='specificity_top_minus_random',arm='Plastic-Yoked',branch=key,family=family,target=target,mode=mode,
                                       difference=difference(top_random['Plastic',key,mode],top_random['Yoked',key,mode])))
        for arm in ('Plastic','Yoked'):
            for branch in branches:
                result.append(dict(kind='reader_dependence',arm=arm,branch=branch,family=family,target=target,mode='transported-refitted',
                                   difference=difference(effects[arm,branch,'transported'],effects[arm,branch,'refitted'])))
    assert len(result)==624
    return result


def score_seed(seed):
    r=root(seed); catalog=read(r/'fits.json'); symbols=m.grammar(seed,'test')
    valid=(r/'test/summary.json').exists() and not (r/'test-failure.json').exists()
    summaries={x['id']:x for x in read(r/'test/summary.json')['systems']} if valid else {}
    calibration_result=read(ART/'calibration-dev/calibration.json') if seed==3001 else calibration(seed)
    cache={}; rows=[]
    for cell in catalog:
        system=cell['system']
        for mode in ('refitted','transported'):
            model=REPO/cell['model'] if mode=='refitted' and cell['model'] else original_model(seed,'Frozen',cell['family'],cell['target']) if mode=='transported' else None
            item=dict(status='invalid_trajectory',loss=None,accuracy=None,logp_sha256=None,**{'lambda':None})
            if valid and model is not None:
                key=(str(model),summaries[system]['q_sha256'])
                if key not in cache: cache[key]=score_model(model,raw(seed,'test',system),symbols)
                item=cache[key]
            rows.append(dict(**{k:v for k,v in cell.items() if k not in ('model','status')},mode=mode,**item,
                model=relative(model) if model is not None else None,
                normalization_sha256=read(model/'model.json')['normalizer_sha256'] if model else None,
                model_manifest_sha256=sha(model/'model.json') if model else None,
                feature_sha256=summaries[system]['q_sha256'] if valid else None,
                stream_sha256=sha_bytes(symbols.tobytes()),sample_count=10000,
                calibration_status='pass' if calibration_result['pass_all'] else 'fail'))
    assert len(rows)==284
    # New predictions for the five identity systems must reproduce NE-1a bytes.
    oldrows=read(oldroot(seed)/'scores.json')['rows']
    for row in rows:
        if row['status']!='valid' or not (row['arm']=='Frozen' or row['branch'] in ('unchanged','all')): continue
        arm='Frozen' if row['arm']=='Frozen' or row['branch']=='all' else row['arm']
        old=next(x for x in oldrows if x['tick']==(0 if arm=='Frozen' else 50000) and x['arm']==arm and x['view']=='q' and
                 (x['family'],x['target'],x['mode'])==(row['family'],row['target'],row['mode']))
        assert all(row[k]==old[k] for k in ('loss','accuracy','logp_sha256','lambda'))
    inherited=read(oldroot(seed)/'scores.json')
    write_new(r/'scores.json',dict(seed=seed,rows=rows,contrasts=contrasts(rows),calibration=calibration_result,
        oracle_loss=inherited['oracle_loss'],baselines=[b for b in inherited['baselines'] if b['target'] in ('lag2','next')]))
    return seed


def summarize(stage,seeds):
    seal=verify_code(stage); manifest=fit_manifest(stage,seeds)
    data=[read(root(s)/'scores.json') for s in seeds]
    groups={}
    for d in data:
        for row in d['contrasts']:
            key=tuple(row[k] for k in ('kind','arm','branch','family','target','mode'))
            groups.setdefault(key,[]).append(row['difference'])
    summaries=[]
    for key,values in groups.items():
        valid=[x for x in values if x is not None]; complete=len(valid)==len(seeds)
        summaries.append(dict(zip(('kind','arm','branch','family','target','mode'),key),values=values,
            complete=complete,valid_count=len(valid),mean=statistics.mean(valid) if complete else None,
            median=statistics.median(valid) if complete else None,sample_sd=statistics.stdev(valid) if complete and len(valid)>1 else None,
            positive_count=sum(v>0 for v in valid) if complete else None))
    failures=[relative(p) for s in seeds for p in root(s).glob('*-failure.json')]
    for seed in seeds:
        for split in ('fit','validation','test'):
            if not (root(seed)/f'{split}-failure.json').exists(): check_trajectory(seed,split)
    value=dict(stage=stage,seeds=seeds,complete=all(len(d['rows'])==284 for d in data),
        integrity_pass=not failures,failures=failures,valid_rows=sum(r['status']=='valid' for d in data for r in d['rows']),
        score_rows=len(seeds)*284,logical_candidate_rows=len(seeds)*710,
        calibration_pass=all(d['calibration']['pass_all'] for d in data),
        code_seal_sha256=sha(BASE/f'{stage}-seal.json'),fit_manifest_sha256=sha(ART/f'{stage}_fit_manifest.json'),
        sealed_files_verified=len(seal['files']),data=data,paired_contrasts=summaries)
    write_new(BASE/f'{stage}-results.json',value)
    print(json.dumps({k:v for k,v in value.items() if k not in ('data','paired_contrasts')}),flush=True)


def execute(stage):
    committed(BASE/f'{stage}-seal.json'); verify_code(stage)
    seeds=[3001] if stage=='development' else list(range(4001,4011))
    for seed in seeds: prepare(seed)
    context=multiprocessing.get_context('spawn')
    print(json.dumps(dict(stage=stage,phase='fit-trajectories',seeds=seeds)),flush=True)
    with ProcessPoolExecutor(max_workers=3,mp_context=context) as pool:
        list(pool.map(collect_job,[(s,split) for s in seeds for split in ('fit','validation')]))
    fit_all(seeds)
    fit_manifest(stage,seeds)
    print(json.dumps(dict(stage=stage,phase='test',fit_manifest=sha(ART/f'{stage}_fit_manifest.json'))),flush=True)
    with ProcessPoolExecutor(max_workers=3,mp_context=context) as pool:
        list(pool.map(collect_job,[(s,'test') for s in seeds]))
    with ProcessPoolExecutor(max_workers=4,mp_context=context) as pool:
        list(pool.map(score_seed,seeds))
    summarize(stage,seeds)


def main():
    parser=argparse.ArgumentParser(); parser.add_argument('command',choices=['calibrate','seal','execute','summarize'])
    parser.add_argument('stage',nargs='?',choices=['development','evaluation'],default='development')
    args=parser.parse_args()
    if args.command=='calibrate': verify_spec(); calibration(3001,refit=True)
    elif args.command=='seal': make_seal(args.stage)
    elif args.command=='execute': execute(args.stage)
    else: summarize(args.stage,[3001] if args.stage=='development' else list(range(4001,4011)))


if __name__=='__main__': main()
