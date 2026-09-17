"""NE-1a orchestration primitives, with an explicit fit/test barrier."""
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import sys
from datetime import datetime, timezone
import measurement as m
import calibrate
np = m.np
ART = m.BASE / "artifacts"
BIN = m.BASE / "target/release/ne1"


def read(path): return json.loads(Path(path).read_text())
def cohort(seed): return "dev" if seed == 3001 else "eval"
def seedroot(seed): return ART / f"{cohort(seed)}-{seed}"


def command(argv):
    argv = list(map(str, argv))
    record = dict(argv=argv, utc=datetime.now(timezone.utc).isoformat())
    with (ART / "attempts.jsonl").open("a") as log:
        log.write(json.dumps(dict(event="start", **record)) + "\n")
    result = subprocess.run(argv, cwd=m.REPO)
    with (ART / "attempts.jsonl").open("a") as log:
        log.write(json.dumps(dict(event="exit", returncode=result.returncode, **record)) + "\n")
    if result.returncode: raise RuntimeError("step failed; preserve attempt before an explicit rerun")


def verify_code(sealpath):
    seal = read(sealpath)
    m.verify_spec()
    if m.sha(BIN) != seal["binary_sha256"]: raise ValueError("binary mismatch")
    for path, digest in seal["files"].items():
        if m.sha(m.REPO / path) != digest: raise ValueError(f"changed code: {path}")
    return seal


def prepare(seed):
    root = seedroot(seed)
    root.mkdir(parents=True, exist_ok=True)
    source = m.BASE.parent / "neural-ecology-ne0c/artifacts" / f"{cohort(seed)}-{seed}"
    archive = read(source / "logs/archive.json")
    if seed == 3001:
        inherited = read(m.BASE.parent / 'neural-ecology-ne0c/implementation-seal.json')
        relative = str((source / 'logs/archive.json').relative_to(m.REPO))
        assert m.sha(source / 'logs/archive.json') == inherited['development_inventory'][relative]
    inventory = {(r['seed'], r['requested_tick']): r for r in read(m.BASE/'checkpoint-inventory.json')['records']}
    frozen = None
    for tick, saved in zip(m.TIMES, m.ARCHIVED):
        name = f"checkpoints/A-{saved:06}.json"
        path = source / (name + ".gz")
        assert m.sha(path) == archive[name]["gzip_sha256"]
        data = gzip.decompress(path.read_bytes())
        assert hashlib.sha256(data).hexdigest() == archive[name]["original_sha256"]
        if seed != 3001:
            expected = inventory[seed, tick]
            assert m.sha(path) == expected['gzip_sha256']
            assert hashlib.sha256(data).hexdigest() == expected['original_sha256']
        checkpoint = json.loads(data)
        assert checkpoint["seed"] == seed and tick // 4096 == saved // 4096
        weights = [net["weights"] for net in checkpoint["networks"]]
        if frozen is None:
            frozen = weights[0]
            assert all(w == frozen for w in weights)
        assert weights[0] == frozen
        value = dict(seed=seed, topology=checkpoint["topology"], weights=weights)
        destination = root / f"weights-{tick}.json"
        if destination.exists(): assert read(destination) == value
        else: m.write(destination, value)
    for split in ("fit", "validation"):
        path = root / f"{split}-symbols.u8"
        symbols = m.grammar(seed, split)
        assert m.groups_ok(m.labels(symbols, "current"))
        if path.exists(): assert path.read_bytes() == symbols.tobytes()
        else: symbols.tofile(path)
    return root


def verify_trajectory(path, seed, split, sealpath):
    summary = read(path / "summary.json")
    assert summary["seed"] == seed and summary["split"] == split and summary["ticks"] == 10256
    assert summary["reset"] and summary["backend_agreement"] and summary["metal_repeat"] and summary["weights_unchanged"]
    assert summary["binary_sha256"] == read(sealpath)["binary_sha256"]
    assert m.sha(path / "symbols.u8") == summary["stream_sha256"]
    assert (path / "symbols.u8").read_bytes() == m.grammar(seed, split).tobytes()
    for name, digest in summary["feature_sha256"].items():
        assert m.sha(path / name) == digest
        assert (path / name).stat().st_size == 10000 * 4096 * 2
    return summary


def collect(seed, split, sealpath):
    root = seedroot(seed)
    for tick in m.TIMES:
        path = root / f"time-{tick}" / split
        if not (path / "summary.json").exists():
            command([BIN, "measure", root / f"weights-{tick}.json", split, path, sealpath])
        verify_trajectory(path, seed, split, sealpath)
    first = read(root / "time-0" / split / "summary.json")
    first_hash = first["feature_sha256"]["Frozen-full.u16"]
    assert all(h == first_hash for h in first["feature_sha256"].values())
    for tick in m.TIMES:
        assert read(root / f"time-{tick}" / split / "summary.json")["feature_sha256"]["Frozen-full.u16"] == first_hash


def raw_features(path, view):
    a = np.memmap(path, mode="r", dtype="<u2", shape=(10000, 4096))
    assert np.all(a[:, :1024] <= 255) and np.all(a[:, 1024:3072] <= 1) and np.all(a[:, 3072:] <= 255)
    return np.array(a[:, 3072:] if view == "q" else a, dtype=np.float64)


def model_score(model, path, symbols):
    record = read(model / "model.json")
    if record["fit"]["selected_index"] is None:
        return dict(status="invalid_fit", loss=None, accuracy=None, logp_sha256=None, **{"lambda": None})
    family, target, view = record["family"], record["target"], record["view"]
    current = m.labels(symbols, "current")
    raw = raw_features(path, view)
    if family == "anchor": x = raw / (255.0 * 32.0)
    else:
        assert m.sha(model / "normalizer.npz") == record["normalizer_sha256"]
        with np.load(model / "normalizer.npz", allow_pickle=False) as data:
            x = m.transform(raw, {k: data[k] for k in data.files})
    x = m.inputs(x, current, family, target)
    fit = record["fit"]
    chosen = fit["candidates"][fit["selected_index"]]
    coefficients = model / chosen["coefficient_file"]
    assert m.sha(coefficients) == chosen["coefficient_sha256"]
    with np.load(coefficients, allow_pickle=False) as data:
        lp = m.log_probabilities(x, current, data["w"], data["b"], family)
    result = m.score(lp, m.labels(symbols, target))
    return dict(status="valid", **result, **{"lambda": chosen["lambda"]})


def fit_neural(seed):
    root = seedroot(seed)
    symbols = {s: np.fromfile(root / f"{s}-symbols.u8", dtype=np.uint8) for s in ("fit", "validation")}
    current = {s: m.labels(v, "current") for s, v in symbols.items()}
    catalog = []
    for tick in m.TIMES:
        for arm in m.ARMS:
            paths = {s: root / f"time-{tick}" / s / f"{arm}-full.u16" for s in symbols}
            hashes = {s: m.sha(p) for s, p in paths.items()}
            for view in ("q", "full"):
                raw = {s: raw_features(p, view) for s, p in paths.items()}
                norm = m.normalizer(raw["fit"])
                normalized = {s: m.transform(x, norm) for s, x in raw.items()}
                for v, family, target in m.READOUTS:
                    if v != view: continue
                    key = dict(seed=seed, feature_hashes=hashes, view=view, family=family, target=target,
                               stream_hashes={s: hashlib.sha256(x.tobytes()).hexdigest() for s,x in symbols.items()})
                    keyhash = hashlib.sha256(json.dumps(key, sort_keys=True).encode()).hexdigest()
                    model = root / "models" / keyhash
                    if (model / "model.json").exists():
                        assert read(model / "model.json")["key"] == key
                    else:
                        if model.exists(): raise ValueError(f"incomplete model attempt retained: {model}")
                        x = {s: m.inputs(a / (255.0*32.0) if family == "anchor" else normalized[s], current[s], family, target)
                             for s,a in raw.items()}
                        y = {s: m.labels(a, target) for s,a in symbols.items()}
                        if family == "anchor":
                            model.mkdir(parents=True)
                            try:
                                w,b,diagnostics = m.anchor.fit_array(x["fit"], y["fit"])
                                np.savez(model / "anchor.npz", w=w[None], b=b[None])
                                lp=m.anchor.probabilities(x["validation"],w,b)
                                candidate=dict(valid=True, coefficient_file="anchor.npz", coefficient_sha256=m.sha(model/"anchor.npz"),
                                               validation_loss=float(-lp[np.arange(10000),y["validation"]].mean()), heads=[diagnostics], **{"lambda":.01})
                                fit=dict(status="valid",family=family,selected_index=0,candidates=[candidate])
                            except ValueError as error:
                                candidate=dict(valid=False,validation_loss=None,heads=[dict(message=str(error))],**{"lambda":.01})
                                fit=dict(status="invalid_fit",family=family,selected_index=None,candidates=[candidate])
                            m.write(model/"fit.json",fit)
                        else:
                            fit=m.fit_grid(x["fit"],y["fit"],current["fit"],x["validation"],y["validation"],current["validation"],family,model)
                            np.savez(model / "normalizer.npz", **norm)
                        m.write(model / "model.json", dict(key=key,view=view,family=family,target=target,fit=fit,
                                normalizer_sha256=m.sha(model/"normalizer.npz") if family!="anchor" else None))
                    catalog.append(dict(seed=seed,tick=tick,arm=arm,view=view,family=family,target=target,
                                        model=str(model.relative_to(m.REPO))))
                del raw, normalized
    assert len(catalog)==225
    m.write(root / "neural-fits.json", catalog)
    return catalog


def fit_calibration(seed):
    root=seedroot(seed)/"calibration"
    if (root/"fits.json").exists():return read(root/"fits.json")
    symbols={s:m.grammar(seed,s) for s in ("fit","validation")}
    records=[]
    for width in (1024,4096):
        for family,kind,target in calibrate.CONTROLS:
            name=f"{width}-{family}-{kind}-{target}";out=root/name
            raw={s:calibrate.control_data(v,width,kind,target) for s,v in symbols.items()}
            norm=m.normalizer(raw['fit'])
            x={s:m.inputs(m.transform(a,norm),m.labels(symbols[s],'current'),family,target) for s,a in raw.items()}
            fit=m.fit_grid(x['fit'],m.labels(symbols['fit'],target),m.labels(symbols['fit'],'current'),
                          x['validation'],m.labels(symbols['validation'],target),m.labels(symbols['validation'],'current'),family,out)
            np.savez(out/'normalizer.npz',**norm)
            records.append(dict(name=name,width=width,family=family,kind=kind,target=target,fit=fit,
                                normalizer_sha256=m.sha(out/'normalizer.npz')))
    m.write(root/'fits.json',records)
    return records


def score_calibration(seed):
    if seed==3001:
        return read(ART/'calibration-dev-3001-v1/result.json')
    root=seedroot(seed)/'calibration';symbols=m.grammar(seed,'test');current=m.labels(symbols,'current')
    oracle=float(-np.log(np.where(m.labels(symbols,'next')==(m.labels(symbols,'lag1')+current)%4,.7,.1)).mean())
    rows=[]
    for record in read(root/'fits.json'):
        row={k:record[k] for k in ('name','width','family','kind','target')};fit=record['fit']
        row.update(status='invalid_fit',passed=False,loss=None,accuracy=None)
        if fit['selected_index'] is not None and m.groups_ok(current):
            out=root/record['name'];candidate=fit['candidates'][fit['selected_index']]
            assert m.sha(out/'normalizer.npz')==record['normalizer_sha256']
            assert m.sha(out/candidate['coefficient_file'])==candidate['coefficient_sha256']
            with np.load(out/'normalizer.npz',allow_pickle=False) as data:norm={k:data[k] for k in data.files}
            x=m.inputs(m.transform(calibrate.control_data(symbols,record['width'],record['kind'],record['target']),norm),current,record['family'],record['target'])
            with np.load(out/candidate['coefficient_file'],allow_pickle=False) as data:
                lp=m.log_probabilities(x,current,data['w'],data['b'],record['family'])
            row.update(m.score(lp,m.labels(symbols,record['target'])))
            passed=(row['accuracy']>=.999 and row['loss']<=.010) if record['kind']=='copy' else abs(row['loss']-np.log(4))<=.010 if record['kind']=='additive' else row['loss']-oracle<=.010 and row['accuracy']>=.65
            row.update(status='valid' if passed else 'calibration_failed',passed=bool(passed),**{'lambda':candidate['lambda']})
        rows.append(row)
    result=dict(seed=seed,rows=rows,pass_all=all(r['passed'] for r in rows),oracle_loss=oracle)
    m.write(root/'result.json',result);return result


def score_seed(seed):
    root=seedroot(seed);catalog=read(root/'neural-fits.json')
    calibration=score_calibration(seed)
    symbols=np.fromfile(root/'time-0/test/symbols.u8',dtype=np.uint8)
    if not m.groups_ok(m.labels(symbols,'current')):raise ValueError('test current-symbol count below 128; no redraw')
    zero={(r['view'],r['family'],r['target']):r['model'] for r in catalog if r['tick']==0 and r['arm']=='Frozen'}
    rows=[];cache={}
    for entry in catalog:
        tick,arm=entry['tick'],entry['arm'];path=root/f'time-{tick}/test'/f'{arm}-full.u16';featurehash=m.sha(path)
        for mode in ('refitted','transported'):
            model=m.REPO/(entry['model'] if mode=='refitted' else zero[entry['view'],entry['family'],entry['target']])
            key=(str(model),featurehash)
            if key not in cache:cache[key]=model_score(model,path,symbols)
            result=cache[key]
            record=read(model/'model.json')
            width=1024 if entry['view']=='q' else 4096
            checks=[c for c in calibration['rows'] if c['width']==width and c['family']==entry['family'] and c['target']==entry['target']]
            status='anchor_exempt' if entry['family']=='anchor' else 'pass' if checks and all(c['passed'] for c in checks) else 'fail'
            rows.append(dict(**{k:v for k,v in entry.items() if k!='model'},mode=mode,cohort=cohort(seed),archived_tick=m.ARCHIVED[m.TIMES.index(tick)],
                        sample_count=10000,**result,calibration_status=status,
                        model=str(model.relative_to(m.REPO)),model_manifest_sha256=m.sha(model/'model.json'),
                        normalization_sha256=record['normalizer_sha256'],feature_sha256=featurehash,
                        stream_sha256=hashlib.sha256(symbols.tobytes()).hexdigest()))
    assert len(rows)==450
    lookup={(r['tick'],r['arm'],r['view'],r['family'],r['target'],r['mode']):r for r in rows}
    for row in rows:
        if row['tick']==0 or row['arm']=='Frozen':
            baseline=lookup[0,'Frozen',row['view'],row['family'],row['target'],'refitted']
            assert row['logp_sha256']==baseline['logp_sha256']
    contrasts=[]
    for tick in m.TIMES:
        for view,family,target in m.READOUTS:
            for mode in ('refitted','transported'):
                plastic=lookup[tick,'Plastic',view,family,target,mode]
                for arm in ('Frozen','Yoked'):
                    control=lookup[tick,arm,view,family,target,mode]
                    difference=None if control['loss'] is None or plastic['loss'] is None else control['loss']-plastic['loss']
                    contrasts.append(dict(seed=seed,tick=tick,view=view,family=family,target=target,mode=mode,control=arm,difference=difference))
    fit_symbols=np.fromfile(root/'fit-symbols.u8',dtype=np.uint8);baselines=[]
    for target in m.TARGETS:
        y=m.labels(fit_symbols,target);test_y=m.labels(symbols,target)
        c=np.zeros(10000,dtype=int) if target=='current' else m.labels(fit_symbols,'current')
        tc=np.zeros(10000,dtype=int) if target=='current' else m.labels(symbols,'current')
        probabilities=np.array([(np.bincount(y[c==head],minlength=4)+1)/(int((c==head).sum())+4) for head in range(1 if target=='current' else 4)])
        baselines.append(dict(target=target,uniform_loss=float(np.log(4)),**m.score(np.log(probabilities[tc]),test_y)))
    oracle=float(-np.log(np.where(m.labels(symbols,'next')==(m.labels(symbols,'lag1')+m.labels(symbols,'current'))%4,.7,.1)).mean())
    m.write(root/'scores.json',dict(seed=seed,cohort=cohort(seed),rows=rows,contrasts=contrasts,baselines=baselines,oracle_loss=oracle,calibration=calibration))
    return rows
