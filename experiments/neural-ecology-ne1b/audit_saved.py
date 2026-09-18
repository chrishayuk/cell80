"""Independent saved-array arithmetic and reversion audit; never imports run.py."""
import os
for name in ('OMP_NUM_THREADS','OPENBLAS_NUM_THREADS','MKL_NUM_THREADS','VECLIB_MAXIMUM_THREADS','NUMEXPR_NUM_THREADS'):
    os.environ[name]='1'
import argparse
from collections import Counter
from concurrent.futures import ProcessPoolExecutor
import hashlib
import importlib.util
import json
import multiprocessing
from pathlib import Path
import statistics
import struct
import numpy as np
from threadpoolctl import threadpool_limits
LIMIT=threadpool_limits(limits=1)
BASE=Path(__file__).resolve().parent
REPO=BASE.parent.parent
OLD=BASE.parent/'neural-ecology-ne1'
spec=importlib.util.spec_from_file_location('ne1_independent',OLD/'audit_saved.py')
ind=importlib.util.module_from_spec(spec); spec.loader.exec_module(ind)


def read(path): return json.loads(Path(path).read_text())
def sha(path): return ind.digest(Path(path))
def array_sha(a): return hashlib.sha256(a.tobytes()).hexdigest()
def rel(path): return str(Path(path).relative_to(REPO))
def root(seed): return BASE/'artifacts'/f"{'dev' if seed==3001 else 'eval'}-{seed}"
def raw(seed,split,system): return np.fromfile(root(seed)/split/f'{system}-q.u16',dtype='<u2').reshape(10000,1024).astype(np.float64)
def symbols(seed,split): return np.fromfile(root(seed)/split/'symbols.u8',dtype=np.uint8)


def audit_subsets(seed):
    inputs=read(root(seed)/'systems.json'); original=read(OLD/'artifacts'/f"{'dev' if seed==3001 else 'eval'}-{seed}"/'weights-50000.json')
    initial=np.array(original['weights'][0]); sources=original['topology']['sources']
    assert inputs['topology']==original['topology']
    inventory=read(BASE/'selection-inventory.json')
    assert len(inputs['systems'])==71
    for system in inputs['systems']:
        weights=np.array(system['weights']); ids=system['edges']; arm=system['arm']
        if arm=='Frozen':
            assert np.array_equal(weights,initial) and ids==[]; continue
        end=np.array(original['weights'][1 if arm=='Plastic' else 2]); delta=end-initial
        assert ids==sorted(set(ids)) and all(delta[e]!=0 for e in ids)
        expected=end.copy(); expected[ids]=initial[ids]; assert np.array_equal(weights,expected)
        selected=next(r for s in inventory['systems'] if s['seed']==seed and s['arm']==arm for r in s['branches'] if r['branch']==system['branch'])
        assert hashlib.sha256(struct.pack(f'<{len(ids)}I',*ids)).hexdigest()==selected['subset_sha256_le_u32']
        assert hashlib.sha256(weights.astype('<u2').tobytes()).hexdigest()==selected['weights_sha256_le_u16']
        assert int(abs(delta[ids]).sum())==selected['removed_l1']
        if system['branch']=='all': assert np.array_equal(weights,initial)
        if 'percent' in system:
            k=(16384*system['percent']+99)//100; assert len(ids)==k
            top=next(s['edges'] for s in inputs['systems'] if s['arm']==arm and s['branch']==f"top-{system['percent']}")
            if system['branch'].startswith('top-'):
                other=[e for e in range(16384) if e not in set(ids) and delta[e]!=0]
                assert min(abs(delta[ids]))>=max(abs(delta[other]))
            if 'family' in system:
                def groups(edges):
                    return Counter((1 if sources[e]<768 else -1,int(delta[e]) if system['family']=='movement' else int(np.sign(delta[e]))) for e in edges)
                assert groups(ids)==groups(top)
    return 70


def check_model(seed,model):
    directory=REPO/model; record=read(directory/'model.json'); system=record['system']
    arrays={s:raw(seed,s,system) for s in ('fit','validation')}
    sy={s:symbols(seed,s) for s in arrays}; cur={s:ind.labels(v,'current') for s,v in sy.items()}
    normpath=REPO/record['normalizer']; assert sha(normpath)==record['normalizer_sha256']
    with np.load(normpath,allow_pickle=False) as n: norm={k:n[k] for k in n.files}
    mean=arrays['fit'].mean(axis=0); std=np.sqrt(((arrays['fit']-mean)**2).mean(axis=0))
    assert np.array_equal(mean,norm['mu']) and np.array_equal(std,norm['sigma'])
    assert np.array_equal(std<=1e-12,norm['constant'])
    x={s:ind.normalize(v,norm) for s,v in arrays.items()}
    fit=record['fit']; assert len(fit['candidates'])==5
    assert [c['lambda'] for c in fit['candidates']]==[.0001,.001,.01,.1,1.]
    assert fit['selected_index']==ind.select(fit['candidates'])
    checked=0
    for candidate in fit['candidates']:
        assert sha(directory/candidate['coefficient_file'])==candidate['coefficient_sha256']
        with np.load(directory/candidate['coefficient_file'],allow_pickle=False) as z:
            w,b=z['w'],z['b']
        lp=ind.predict(x['validation'],cur['validation'],record['family'],record['target'],w,b)
        score=ind.metrics(lp,ind.labels(sy['validation'],record['target']))
        if candidate['validation_loss'] is not None: assert abs(score['loss']-candidate['validation_loss'])<=1e-12
        heads_valid=[]
        y=ind.labels(sy['fit'],record['target'])
        for h in range(len(w)):
            mask=cur['fit']==h if record['family']=='C' else slice(None)
            xx=x['fit'][mask]; yy=y[mask]
            logits=xx@w[h]+b[h]; logits-=logits.max(axis=1,keepdims=True)
            logp=logits-np.log(np.exp(logits).sum(axis=1,keepdims=True))
            loss=float(-logp[np.arange(len(yy)),yy].mean()+candidate['lambda']/2*np.sum(w[h]*w[h]))
            residual=np.exp(logp); residual[np.arange(len(yy)),yy]-=1; residual/=len(yy)
            gradient=np.concatenate(((xx.T@residual+candidate['lambda']*w[h]).ravel(),residual.sum(axis=0)))
            gn=float(np.abs(gradient).max()); saved=candidate['heads'][h]
            valid=bool(saved['solver_success'] and np.isfinite(loss) and np.all(np.isfinite(gradient)) and
                       np.all(np.isfinite(w[h])) and np.all(np.isfinite(b[h])) and gn<=1e-6 and loss<=saved['initial_objective']+1e-10)
            assert valid==saved['valid']
            if valid:
                assert abs(loss-saved['final_objective'])<=1e-12 and abs(gn-saved['gradient_inf'])<=1e-12
            heads_valid.append(valid)
        assert bool(all(heads_valid) and np.isfinite(score['loss']))==candidate['valid']
        checked+=1
    return checked


def independent_contrasts(saved):
    # Recompute each declared contrast directly from the score table; do not call
    # the runner's contrast generator or use saved difference values recursively.
    table={(r['arm'],r['branch'],r['family'],r['target'],r['mode']):r['loss'] for r in saved['rows']}
    assert len(table)==284
    def difference(x,y): return None if x is None or y is None else x-y
    for row in saved['contrasts']:
        family,target,mode,arm,branch=(row[k] for k in ('family','target','mode','arm','branch'))
        def loss(a,b,mode=mode): return table[a,b,family,target,mode]
        def effect(a,b,mode=mode): return difference(loss(a,b,mode),loss(a,'unchanged',mode))
        def top_random(a):
            control,pct=branch.split('-'); top=effect(a,f'top-{pct}')
            random=[effect(a,f'{control}-{pct}-{r}') for r in range(5)]
            return difference(top,statistics.mean(random) if all(v is not None for v in random) else None)
        kind=row['kind']
        if kind=='baseline': value=difference(loss('Frozen','unchanged'),loss(arm,'unchanged'))
        elif kind=='reversion': value=effect(arm,branch)
        elif kind=='specificity': value=difference(effect('Plastic',branch),effect('Yoked',branch))
        elif kind=='top_minus_random': value=top_random(arm)
        elif kind=='specificity_top_minus_random': value=difference(top_random('Plastic'),top_random('Yoked'))
        elif kind=='reader_dependence': value=difference(effect(arm,branch,'transported'),effect(arm,branch,'refitted'))
        else: raise AssertionError(kind)
        assert value==row['difference']
    counts=Counter(r['kind'] for r in saved['contrasts'])
    assert counts==dict(baseline=8,reversion=272,specificity=136,top_minus_random=48,specificity_top_minus_random=24,reader_dependence=136)
    assert len({tuple(r[k] for k in ('kind','arm','branch','family','target','mode')) for r in saved['contrasts']})==624
    return 624


def audit_seed(seed):
    subsets=audit_subsets(seed); saved=read(root(seed)/'scores.json'); fits=read(root(seed)/'fits.json')
    models=sorted({c['model'] for c in fits if c['model'] is not None})
    candidates=sum(check_model(seed,model) for model in models)
    sy=symbols(seed,'test'); current=ind.labels(sy,'current'); cache={}
    for row in saved['rows']:
        if row['status']!='valid':
            assert row['loss'] is None and row['logp_sha256'] is None; continue
        model=REPO/row['model']; record=read(model/'model.json')
        feature=root(seed)/'test'/f"{row['system']}-q.u16"
        assert sha(feature)==row['feature_sha256']
        assert sha(model/'model.json')==row['model_manifest_sha256']
        key=(row['model'],row['feature_sha256'])
        if key not in cache:
            normpath=REPO/record['normalizer'] if 'normalizer' in record else model/'normalizer.npz'
            assert sha(normpath)==row['normalization_sha256']
            with np.load(normpath,allow_pickle=False) as n: x=ind.normalize(raw(seed,'test',row['system']),n)
            fit=record['fit']; assert fit['selected_index']==ind.select(fit['candidates'])
            selected=fit['candidates'][fit['selected_index']]
            assert selected['lambda']==row['lambda']
            assert sha(model/selected['coefficient_file'])==selected['coefficient_sha256']
            with np.load(model/selected['coefficient_file'],allow_pickle=False) as z:
                lp=ind.predict(x,current,row['family'],row['target'],z['w'],z['b'])
            cache[key]=ind.metrics(lp,ind.labels(sy,row['target']))
        ind.check_metrics(cache[key],row)
    # Independently reconstruct linked synthetic controls from saved coefficients.
    caldir=BASE/'artifacts/calibration-dev' if seed==3001 else OLD/'artifacts'/f'eval-{seed}'/'calibration'
    calibration=read(caldir/'fits.json'); checked=0
    for row in saved['calibration']['rows']:
        record=next(c for c in calibration if c['name']==row['name']); out=caldir/record['name']; fit=record['fit']
        assert fit['selected_index']==ind.select(fit['candidates'])
        assert fit['selected_index'] is not None
        chosen=fit['candidates'][fit['selected_index']]
        a=np.zeros((10000,1024)); category=ind.labels(sy,'lag2' if row['family']=='G' else 'lag1')
        a[np.arange(10000),category]=1
        with np.load(out/'normalizer.npz') as n: x=ind.normalize(a,n)
        with np.load(out/chosen['coefficient_file']) as z: lp=ind.predict(x,current,row['family'],row['target'],z['w'],z['b'])
        metrics=ind.metrics(lp,ind.labels(sy,row['target'])); ind.check_metrics(metrics,row)
        oracle=float(-np.log(np.where(ind.labels(sy,'next')==(ind.labels(sy,'lag1')+current)%4,.7,.1)).mean())
        passed=metrics['accuracy']>=.999 and metrics['loss']<=.010 if row['family']=='G' else metrics['accuracy']>=.65 and metrics['loss']-oracle<=.010
        assert passed==row['passed']; checked+=1
    # Full reversion and zero-change prediction identities.
    table={(r['arm'],r['branch'],r['family'],r['target'],r['mode']):r for r in saved['rows']}
    oldrows=read(OLD/'artifacts'/f"{'dev' if seed==3001 else 'eval'}-{seed}"/'scores.json')['rows']
    identities=0
    for row in saved['rows']:
        if row['status']!='valid' or not (row['arm']=='Frozen' or row['branch'] in ('all','unchanged')): continue
        arm='Frozen' if row['arm']=='Frozen' or row['branch']=='all' else row['arm']
        old=next(x for x in oldrows if x['tick']==(0 if arm=='Frozen' else 50000) and x['arm']==arm and x['view']=='q' and
                 (x['family'],x['target'],x['mode'])==(row['family'],row['target'],row['mode']))
        ind.check_metrics(row,old); identities+=1
    return dict(seed=seed,passed=True,subsets=subsets,rows=len(saved['rows']),valid_rows=sum(r['status']=='valid' for r in saved['rows']),
                unique_predictions=len(cache),models=len(models),candidate_checks=candidates,
                contrasts=independent_contrasts(saved),calibration=checked,identity_predictions=identities)


def main():
    parser=argparse.ArgumentParser();parser.add_argument('stage',choices=['development','evaluation']); args=parser.parse_args()
    stage=args.stage; seeds=[3001] if stage=='development' else list(range(4001,4011))
    seal=read(BASE/f'{stage}-seal.json'); manifest=read(BASE/f'artifacts/{stage}_fit_manifest.json')
    for name,expected in {**seal['files'],**manifest['files']}.items(): assert sha(REPO/name)==expected,name
    saved=read(BASE/f'{stage}-results.json')
    with ProcessPoolExecutor(max_workers=4,mp_context=multiprocessing.get_context('spawn')) as pool:
        reports=list(pool.map(audit_seed,seeds))
    for summary in saved['paired_contrasts']:
        keys=('kind','arm','branch','family','target','mode')
        values=[next(r['difference'] for r in data['contrasts'] if all(r[k]==summary[k] for k in keys)) for data in saved['data']]
        assert summary['values']==values
        good=[v for v in values if v is not None]; complete=len(good)==len(seeds)
        assert summary['complete']==complete and summary['valid_count']==len(good)
        if complete:
            assert summary['mean']==statistics.mean(good) and summary['median']==statistics.median(good)
            assert summary['positive_count']==sum(v>0 for v in good)
            assert summary['sample_sd']==(statistics.stdev(good) if len(good)>1 else None)
    result=dict(stage=stage,passed=True,seeds=reports,auditor_sha256=sha(Path(__file__)),
                scope='Independent saved-array normalization, candidate objectives/gradients, selection, prediction, reversion, calibration and contrast reconstruction; same numerical environment')
    path=BASE/f'{stage}-independent-audit.json'
    if path.exists(): assert read(path)==result
    else: path.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result),flush=True)


if __name__=='__main__':main()
