"""Complete-matrix descriptive tables; no inference, selection or interventions."""
import argparse
import csv
import json
from pathlib import Path
import statistics
import numpy as np

BASE = Path(__file__).resolve().parent
REPO = BASE.parent.parent


def read(path):
    return json.loads(path.read_text())


def summarize(stage):
    report = read(BASE / f'{stage}-results.json')
    independent = read(BASE / f'{stage}-independent-audit.json')
    assert report['complete'] and report['integrity_pass'] and independent['passed']
    n = len(report['seeds'])
    assert report['score_rows'] == 450 * n
    groups = {}
    context = []
    for seed_result in report['data']:
        seed = seed_result['seed']
        root = BASE / 'artifacts' / f"{'dev' if seed == 3001 else 'eval'}-{seed}"
        initial = np.array(read(root / 'weights-0.json')['weights'], dtype=np.int64)
        for row in seed_result['rows']:
            key = tuple(row[k] for k in ('tick', 'arm', 'view', 'family', 'target', 'mode'))
            groups.setdefault(key, []).append(row)
        for tick in (0, 12500, 25000, 37500, 50000):
            weights = np.array(read(root / f'weights-{tick}.json')['weights'], dtype=np.int64)
            summaries = {s: read(root / f'time-{tick}/{s}/summary.json')
                         for s in ('fit', 'validation', 'test')}
            for index, arm in enumerate(('Frozen', 'Plastic', 'Yoked')):
                row = next(r for r in seed_result['rows'] if r['tick'] == tick and r['arm'] == arm
                           and r['view'] == 'full' and r['family'] == 'G'
                           and r['target'] == 'current' and r['mode'] == 'refitted')
                with np.load(REPO / row['model'] / 'normalizer.npz', allow_pickle=False) as norm:
                    blocks = {}
                    for j, name in enumerate(('v', 'r', 's', 'q')):
                        take = slice(j * 1024, (j + 1) * 1024)
                        blocks[name] = dict(mean_of_node_means=float(norm['mu'][take].mean()),
                                            mean_of_node_standard_deviations=float(norm['sigma'][take].mean()),
                                            constant_nodes=int(norm['constant'][take].sum()))
                delta = weights[index] - initial[index]
                context.append(dict(seed=seed, tick=tick, arm=arm,
                    changed_weights=int(np.count_nonzero(delta)), weight_count=len(delta),
                    changed_fraction=float(np.mean(delta != 0)),
                    mean_absolute_weight_change=float(np.abs(delta).mean()),
                    signed_mean_weight_change=float(delta.mean()),
                    endpoint_fraction=float(np.mean((weights[index] == 0) | (weights[index] == 32))),
                    fit_feature_moments=blocks, normalization_sha256=row['normalization_sha256'],
                    activity={s: summary['activity'][index] for s, summary in summaries.items()}))
    rows = []
    for key, values in sorted(groups.items()):
        assert len(values) == n and len({r['seed'] for r in values}) == n
        valid = [r for r in values if r['status'] == 'valid']
        complete = len(valid) == n
        rows.append(dict(zip(('tick', 'arm', 'view', 'family', 'target', 'mode'), key),
            seed_count=n, valid_count=len(valid), complete=complete,
            calibration_failures=sum(r['calibration_status'] == 'fail' for r in values),
            mean_loss=statistics.mean(r['loss'] for r in valid) if complete else None,
            median_loss=statistics.median(r['loss'] for r in valid) if complete else None,
            sample_sd_loss=statistics.stdev(r['loss'] for r in valid) if complete and n > 1 else None,
            mean_accuracy=statistics.mean(r['accuracy'] for r in valid) if complete else None))
    assert len(rows) == 450 and len(context) == 15 * n
    with (BASE / f'{stage}-tables.csv').open('w', newline='') as out:
        writer = csv.DictWriter(out, fieldnames=list(rows[0]), lineterminator='\n')
        writer.writeheader()
        writer.writerows(rows)
    (BASE / f'{stage}-context.json').write_text(json.dumps(dict(
        scope='Descriptive context, not evidence of task alignment; fit moments only',
        rows=context), indent=2) + '\n')
    print(json.dumps(dict(stage=stage, aggregate_rows=len(rows), context_rows=len(context),
                          incomplete_groups=sum(not r['complete'] for r in rows))))


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('stage', choices=['development', 'evaluation'])
    summarize(parser.parse_args().stage)
