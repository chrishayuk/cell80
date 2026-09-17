"""Stages are sealed development, sealed evaluation fitting, then held-out test."""
import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
import platform
import statistics
import subprocess
import sys
import measurement as m
import pipeline as p


def committed(path):
    data=subprocess.check_output(['git','show',f'HEAD:{path.relative_to(m.REPO)}'],cwd=m.REPO)
    if data!=path.read_bytes():raise ValueError('seal must be committed before measurement')


def seal(stage):
    m.verify_spec()
    path=m.BASE/f'{stage}-seal.json'
    if path.exists():raise ValueError('refusing to replace seal')
    calibration=p.read(p.ART/'calibration-dev-3001-v1/result.json')
    if not calibration['pass_all']:raise ValueError('development calibration failed')
    original_path=m.BASE.parent/'neural-ecology-ne0c/implementation-seal.json'
    assert m.sha(original_path)==p.read(m.BASE/'measurement-seal.json')['inherited']['NE0c_implementation_seal_sha256']
    original=p.read(original_path)
    files=dict(original['files'])
    files[str(original_path.relative_to(m.REPO))]=m.sha(original_path)
    for f in [*m.BASE.glob('*.py'),*m.BASE.glob('src/*.rs'),*m.BASE.glob('tests/*.py'),
              m.BASE/'Cargo.toml',m.BASE/'Cargo.lock',m.BASE/'dependencies.lock',m.BASE/'requirements.txt',
              m.BASE/'measurement-spec.md',m.BASE/'measurement-seal.json',m.BASE/'verification.json',
              p.ART/'fixtures.json',*sorted((p.ART/'calibration-dev-3001-v1').rglob('*'))]:
        if f.is_file():files[str(f.relative_to(m.REPO))]=m.sha(f)
    evidence=None
    if stage=='evaluation':
        development=p.read(m.BASE/'development-results.json')
        if not development['complete'] or not development['integrity_pass']:raise ValueError('development matrix not validated')
        files[str((m.BASE/'development-seal.json').relative_to(m.REPO))]=m.sha(m.BASE/'development-seal.json')
        files[str((m.BASE/'development-results.json').relative_to(m.REPO))]=m.sha(m.BASE/'development-results.json')
        evidence=dict(development_results_sha256=m.sha(m.BASE/'development-results.json'))
        if list(p.ART.glob('eval-*')):raise ValueError('evaluation artifacts precede seal')
    elif (p.ART/'dev-3001').exists():raise ValueError('development data precede code seal')
    fixture=p.read(p.ART/'fixtures.json')
    assert fixture['fixture_hash']==original['fixture']['fixture_hash']
    m.write(path,dict(schema='NE1a-code-seal-v1',stage=stage,utc=datetime.now(timezone.utc).isoformat(),
        files=files,binary_sha256=m.sha(p.BIN),measurement_spec_sha256=m.sha(m.BASE/'measurement-spec.md'),
        source_commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=m.REPO,text=True).strip(),
        environment=dict(python=sys.version,platform=platform.platform(),numpy=m.np.__version__,
            scipy=m.importlib.metadata.version('scipy'),threadpoolctl=m.importlib.metadata.version('threadpoolctl'),
            threadpools=m.anchor.threadpool_info()),development_evidence=evidence))
    print(f'Wrote {path}; commit before running {stage}.',flush=True)


def fit_manifest(seeds,sealpath):
    name='development' if seeds==[3001] else 'evaluation'
    path=p.ART/f'{name}_fit_manifest.json'
    if path.exists():
        manifest=p.read(path)
        for file,digest in manifest['files'].items():assert m.sha(m.REPO/file)==digest
        return manifest
    files={}
    for seed in seeds:
        root=p.seedroot(seed)
        assert len(p.read(root/'neural-fits.json'))==225
        for tick in m.TIMES:assert not (root/f'time-{tick}/test').exists()
        for f in sorted(root.rglob('*')):
            if f.is_file():files[str(f.relative_to(m.REPO))]=m.sha(f)
    m.write(path,dict(all_fits_complete=True,seeds=seeds,code_seal_sha256=m.sha(sealpath),files=files,
            development_calibration_precedes_neural_fitting=name=='development'))
    return p.read(path)


def audit(seeds,sealpath):
    p.verify_code(sealpath)
    manifest=fit_manifest(seeds,sealpath)
    for file,digest in manifest['files'].items():assert m.sha(m.REPO/file)==digest
    outputs=[];reproduced=0;logical_candidates=0
    for seed in seeds:
        root=p.seedroot(seed);result=p.read(root/'scores.json')
        assert len(result['rows'])==450 and len(result['contrasts'])==300 and len(result['baselines'])==4
        cache={}
        for tick in m.TIMES:
            for split in ('fit','validation','test'):p.verify_trajectory(root/f'time-{tick}'/split,seed,split,sealpath)
        for fit in p.read(root/'neural-fits.json'):
            record=p.read(m.REPO/fit['model']/'model.json')
            expected=1 if fit['family']=='anchor' else 5
            assert len(record['fit']['candidates'])==expected
            logical_candidates+=expected
            if fit['family']!='anchor':assert record['fit']['selected_index']==m.selected_index(record['fit']['candidates'])
            for candidate in record['fit']['candidates']:
                if candidate['valid'] and fit['family']!='anchor':
                    assert all(h['solver_success'] and h['gradient_inf']<=1e-6 and h['final_objective']<=h['initial_objective']+1e-10 for h in candidate['heads'])
        symbols=m.np.fromfile(root/'time-0/test/symbols.u8',dtype=m.np.uint8)
        for row in result['rows']:
            model=m.REPO/row['model'];feature=root/f"time-{row['tick']}/test"/f"{row['arm']}-full.u16"
            key=(str(model),row['feature_sha256'])
            if key not in cache:cache[key]=p.model_score(model,feature,symbols)
            again=cache[key]
            for field in ('status','loss','accuracy','logp_sha256','lambda'):assert again[field]==row[field]
            reproduced+=1
        outputs.append(result)
    assert logical_candidates==1065*len(seeds)
    groups={}
    for result in outputs:
        for row in result['contrasts']:
            key=(row['tick'],row['view'],row['family'],row['target'],row['mode'],row['control'])
            groups.setdefault(key,[]).append(row['difference'])
    contrasts=[]
    for key,values in groups.items():
        valid=[v for v in values if v is not None]
        complete=len(valid)==len(seeds)
        contrasts.append(dict(zip(('tick','view','family','target','mode','control'),key),
            values=values,valid_count=len(valid),complete=complete,
            mean=statistics.mean(valid) if complete else None,median=statistics.median(valid) if complete else None,
            sample_sd=statistics.stdev(valid) if complete and len(valid)>1 else None,
            positive_count=sum(v>0 for v in valid) if complete else None))
    report=dict(complete=True,integrity_pass=True,cohort='dev' if seeds==[3001] else 'eval',seeds=seeds,
        code_seal_sha256=m.sha(sealpath),fit_manifest_sha256=m.sha(p.ART/f"{'development' if seeds==[3001] else 'evaluation'}_fit_manifest.json"),
        score_rows=reproduced,logical_candidate_rows=logical_candidates,
        valid_score_rows=sum(r['status']=='valid' for d in outputs for r in d['rows']),
        calibration_pass=all(d['calibration']['pass_all'] for d in outputs),
        data=outputs,paired_contrasts=contrasts,
        scope='Exploratory fixed-weight measurements; no interventions or new learning claim')
    destination=m.BASE/('development-results.json' if seeds==[3001] else 'evaluation-results.json')
    m.write(destination,report)
    print(json.dumps({k:v for k,v in report.items() if k not in ('data','paired_contrasts')}),flush=True)
    return report


def execute(stage):
    seeds=[3001] if stage=='development' else list(range(4001,4011))
    sealpath=m.BASE/f'{stage}-seal.json'
    committed(sealpath);p.verify_code(sealpath)
    for seed in seeds:
        p.prepare(seed)
        for split in ('fit','validation'):p.collect(seed,split,sealpath)
        p.fit_neural(seed)
        if seed!=3001:p.fit_calibration(seed)
    fit_manifest(seeds,sealpath)
    for seed in seeds:
        p.collect(seed,'test',sealpath)
        p.score_seed(seed)
    audit(seeds,sealpath)


if __name__=='__main__':
    parser=argparse.ArgumentParser()
    parser.add_argument('mode',choices=['seal-development','development','seal-evaluation','evaluation','audit-development','audit-evaluation'])
    mode=parser.parse_args().mode
    if mode.startswith('seal-'):seal(mode.removeprefix('seal-'))
    elif mode.startswith('audit-'):
        stage=mode.removeprefix('audit-');audit([3001] if stage=='development' else list(range(4001,4011)),m.BASE/f'{stage}-seal.json')
    else:execute(mode)
