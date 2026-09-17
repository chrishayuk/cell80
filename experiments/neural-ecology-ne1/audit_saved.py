"""Read-only independent numerical audit; never imports the experiment runner.

Reconstructs inputs, normalization, selection and predictions from saved arrays.
This is independent arithmetic code in the same sealed NumPy environment, not a
cross-library or cross-hardware reproducibility claim. No fitting takes place.
"""
import os
for key in ('OMP_NUM_THREADS', 'OPENBLAS_NUM_THREADS', 'MKL_NUM_THREADS',
            'VECLIB_MAXIMUM_THREADS', 'NUMEXPR_NUM_THREADS'):
    os.environ[key] = '1'
import argparse
import hashlib
import json
from pathlib import Path
import numpy as np
from threadpoolctl import threadpool_limits

LIMIT = threadpool_limits(limits=1)
BASE = Path(__file__).resolve().parent
REPO = BASE.parent.parent
OFFSETS = dict(current=0, lag1=-1, lag2=-2, next=1)


def read(path):
    return json.loads(path.read_text())


def digest(path):
    h = hashlib.sha256()
    with path.open('rb') as stream:
        for part in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(part)
    return h.hexdigest()


def labels(symbols, target):
    return symbols[np.arange(256, 10256) + OFFSETS[target]].astype(np.int64)


def features(path, view):
    raw = np.memmap(path, dtype='<u2', mode='r', shape=(10000, 4096))
    return np.array(raw[:, 3072:] if view == 'q' else raw, dtype=np.float64)


def normalize(raw, norm):
    denominator = norm['sigma'].copy()
    denominator[norm['constant']] = 1
    x = (raw - norm['mu']) / denominator
    x[:, norm['constant']] = 0
    return x


def predict(x, current, family, target, weights, intercepts):
    if family != 'C' and target == 'next':
        x = np.concatenate([x, np.eye(4)[current]], axis=1)
    result = np.empty((len(x), 4))
    for head in range(4 if family == 'C' else 1):
        rows = current == head if family == 'C' else slice(None)
        logits = x[rows] @ weights[head] + intercepts[head]
        logits -= logits.max(axis=1, keepdims=True)
        result[rows] = logits - np.log(np.exp(logits).sum(axis=1, keepdims=True))
    return result


def metrics(logp, target):
    return dict(loss=float(-logp[np.arange(len(target)), target].mean()),
                accuracy=float(np.mean(np.argmax(logp, axis=1) == target)),
                logp_sha256=hashlib.sha256(logp.astype('<f8').tobytes()).hexdigest())


def check_metrics(actual, saved):
    assert abs(actual['loss'] - saved['loss']) <= 1e-12
    assert actual['accuracy'] == saved['accuracy']
    assert actual['logp_sha256'] == saved['logp_sha256']


def select(candidates):
    if any(not c['valid'] for c in candidates):
        return None
    best = min(c['validation_loss'] for c in candidates)
    eligible = [i for i, c in enumerate(candidates) if c['validation_loss'] <= best + 1e-8]
    return max(eligible, key=lambda i: candidates[i]['lambda'])


def audit_seed(seed):
    root = BASE / 'artifacts' / f"{'dev' if seed == 3001 else 'eval'}-{seed}"
    saved = read(root / 'scores.json')
    symbols = np.fromfile(root / 'time-0/test/symbols.u8', dtype=np.uint8)
    current = labels(symbols, 'current')
    lookup = {(r['tick'], r['arm'], r['view'], r['family'], r['target'], r['mode']): r
              for r in saved['rows']}
    assert len(lookup) == len(saved['rows']) == 450
    catalog = read(root / 'neural-fits.json')
    assert len(catalog) == 225
    entry_by_model = {entry['model']: entry for entry in catalog}
    records, norms, coefficients, normalization_checks = {}, {}, {}, set()
    for modelpath, entry in entry_by_model.items():
        model = REPO / modelpath
        record = read(model / 'model.json')
        records[modelpath] = record
        fit = record['fit']
        assert fit['selected_index'] == select(fit['candidates'])
        assert len(fit['candidates']) == (1 if entry['family'] == 'anchor' else 5)
        if entry['family'] != 'anchor':
            assert digest(model / 'normalizer.npz') == record['normalizer_sha256']
            with np.load(model / 'normalizer.npz', allow_pickle=False) as z:
                norm = {key: z[key] for key in z.files}
            norms[modelpath] = norm
            key = (record['key']['feature_hashes']['fit'], entry['view'], record['normalizer_sha256'])
            if key not in normalization_checks:
                raw = features(root / f"time-{entry['tick']}/fit/{entry['arm']}-full.u16", entry['view'])
                mean = raw.mean(axis=0)
                scale = np.sqrt(((raw - mean) ** 2).mean(axis=0))
                np.testing.assert_array_equal(mean, norm['mu'])
                np.testing.assert_array_equal(scale, norm['sigma'])
                np.testing.assert_array_equal(scale <= 1e-12, norm['constant'])
                normalization_checks.add(key)
        if fit['selected_index'] is not None:
            chosen = fit['candidates'][fit['selected_index']]
            assert digest(model / chosen['coefficient_file']) == chosen['coefficient_sha256']
            with np.load(model / chosen['coefficient_file'], allow_pickle=False) as z:
                coefficients[modelpath] = (z['w'], z['b'])
    cache, invalid = {}, 0
    for row in saved['rows']:
        modelpath = row['model']
        zero = lookup[0, 'Frozen', row['view'], row['family'], row['target'], 'refitted']
        if row['mode'] == 'transported':
            assert modelpath == zero['model']
        if row['tick'] == 0 or row['arm'] == 'Frozen':
            assert row['logp_sha256'] == zero['logp_sha256']
        record = records[modelpath]
        assert row['model_manifest_sha256'] == digest(REPO / modelpath / 'model.json')
        if record['fit']['selected_index'] is None:
            assert row['status'] == 'invalid_fit'
            assert all(row[key] is None for key in ('loss', 'accuracy', 'logp_sha256', 'lambda'))
            invalid += 1
            continue
        key = (modelpath, row['feature_sha256'])
        if key not in cache:
            path = root / f"time-{row['tick']}/test/{row['arm']}-full.u16"
            assert digest(path) == row['feature_sha256']
            raw = features(path, row['view'])
            x = raw / 8160 if row['family'] == 'anchor' else normalize(raw, norms[modelpath])
            lp = predict(x, current, row['family'], row['target'], *coefficients[modelpath])
            cache[key] = metrics(lp, labels(symbols, row['target']))
        check_metrics(cache[key], row)
    for row in saved['contrasts']:
        key = (row['tick'], row['view'], row['family'], row['target'], row['mode'])
        a = lookup[key[0], row['control'], *key[1:]]['loss']
        b = lookup[key[0], 'Plastic', *key[1:]]['loss']
        assert row['difference'] == (None if a is None or b is None else a - b)
    fit_symbols = np.fromfile(root / 'fit-symbols.u8', dtype=np.uint8)
    for baseline in saved['baselines']:
        target = baseline['target']
        y = labels(fit_symbols, target)
        group = np.zeros(10000, dtype=int) if target == 'current' else labels(fit_symbols, 'current')
        test_group = np.zeros(10000, dtype=int) if target == 'current' else current
        tables = []
        for c in range(1 if target == 'current' else 4):
            counts = np.bincount(y[group == c], minlength=4) + 1
            tables.append(counts / counts.sum())
        check_metrics(metrics(np.log(np.array(tables)[test_group]), labels(symbols, target)), baseline)
    oracle = -np.log(np.where(labels(symbols, 'next') ==
                             (labels(symbols, 'lag1') + current) % 4, .7, .1)).mean()
    assert float(oracle) == saved['oracle_loss']
    return dict(seed=seed, passed=True, rows=450, invalid_rows=invalid,
                unique_predictions=len(cache), normalization_checks=len(normalization_checks),
                model_selection_checks=len(records), contrasts=300, baselines=4)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('stage', choices=['development', 'evaluation'])
    args = parser.parse_args()
    seeds = [3001] if args.stage == 'development' else list(range(4001, 4011))
    report = dict(stage=args.stage, auditor_sha256=digest(Path(__file__)), seeds=[])
    for seed in seeds:
        result = audit_seed(seed)
        report['seeds'].append(result)
        print(json.dumps(result), flush=True)
    report['passed'] = True
    report['scope'] = 'Independent saved-neural-prediction arithmetic, normalization, selection, identities, contrasts and baselines; same NumPy environment.'
    (BASE / f'{args.stage}-independent-audit.json').write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
