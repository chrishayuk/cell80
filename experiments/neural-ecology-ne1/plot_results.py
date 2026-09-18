"""Reporting only, after the complete matrix and both audits; no model fitting."""
import json
import os
from pathlib import Path
import statistics

BASE = Path(__file__).resolve().parent
os.environ['MPLCONFIGDIR'] = str(BASE / 'artifacts/matplotlib-cache')
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np


def main():
    report = json.loads((BASE / 'evaluation-results.json').read_text())
    audit = json.loads((BASE / 'evaluation-independent-audit.json').read_text())
    assert report['complete'] and report['integrity_pass'] and audit['passed']
    rows = [row for seed in report['data'] for row in seed['rows']]
    assert len(rows) == 4500 and len(report['seeds']) == 10
    readers = [('q', 'anchor', 'q · original'), ('q', 'G', 'q · global'),
               ('full', 'G', 'full · global'), ('q', 'C', 'q · conditioned'),
               ('full', 'C', 'full · conditioned')]
    times = [0, 12500, 25000, 37500, 50000]
    colors = {'Frozen': '#2166ac', 'Plastic': '#d66024', 'Yoked': '#278451'}
    oracle = statistics.mean(seed['oracle_loss'] for seed in report['data'])
    fig, axes = plt.subplots(2, 5, figsize=(17, 6.8), sharex=True, sharey=True)
    extremes = [oracle, np.log(4)]
    incomplete = 0
    for row_index, mode in enumerate(('refitted', 'transported')):
        for col, (view, family, title) in enumerate(readers):
            ax = axes[row_index, col]
            for arm, color in colors.items():
                means, deviations = [], []
                for tick in times:
                    group = [r for r in rows if (r['tick'], r['arm'], r['view'], r['family'], r['target'], r['mode'])
                             == (tick, arm, view, family, 'next', mode)]
                    assert len(group) == 10 and len({r['seed'] for r in group}) == 10
                    if any(r['status'] != 'valid' for r in group):
                        means.append(np.nan)
                        deviations.append(np.nan)
                        incomplete += 1
                    else:
                        means.append(statistics.mean(r['loss'] for r in group))
                        deviations.append(statistics.stdev(r['loss'] for r in group))
                mean = np.array(means)
                sd = np.array(deviations)
                extremes.extend((mean - sd)[np.isfinite(mean)].tolist())
                extremes.extend((mean + sd)[np.isfinite(mean)].tolist())
                ax.plot(np.array(times) / 1000, mean, marker='o', markersize=3.5,
                        color=color, label=arm, linewidth=1.6)
                ax.fill_between(np.array(times) / 1000, mean - sd, mean + sd, color=color, alpha=.10)
            ax.axhline(oracle, color='#505050', linestyle='--', linewidth=.9, label='Sampled oracle (mean)')
            ax.axhline(np.log(4), color='#969696', linestyle=':', linewidth=1, label='Uniform')
            ax.grid(axis='y', alpha=.16)
            ax.spines[['top', 'right']].set_visible(False)
            ax.set_xticks([0, 12.5, 25, 37.5, 50], ['0', '12.5', '25', '37.5', '50'])
            ax.tick_params(labelsize=8)
            if row_index == 0:
                ax.set_title(title, fontsize=11)
            if row_index == 1:
                ax.set_xlabel('Adaptation ticks (thousands)', fontsize=9)
        label = 'Refitted' if mode == 'refitted' else 'Zero-time reader transported'
        axes[row_index, 0].set_ylabel(f'{label}\nTest log loss (nats)', fontsize=10)
    axes[0, 0].set_ylim(min(extremes) - .018, max(extremes) + .018)
    handles, labels = axes[0, 0].get_legend_handles_labels()
    fig.legend(handles, labels, loc='lower center', ncol=5, frameon=False, bbox_to_anchor=(.5, .035))
    fig.suptitle('NE-1a · all next-symbol readers and checkpoints', fontsize=15, y=.98)
    fig.text(.5, .015, f'10 reused seeds; exploratory. Bands show ±1 sample SD, not confidence intervals. '
             f'Incomplete groups: {incomplete}. Calibration passes: {report["calibration_pass"]}.',
             ha='center', fontsize=9)
    fig.tight_layout(rect=(0, .10, 1, .94))
    for suffix in ('png', 'svg'):
        fig.savefig(BASE / f'next-symbol-loss.{suffix}', dpi=160, facecolor='white')
    svg = BASE / 'next-symbol-loss.svg'
    svg.write_text('\n'.join(line.rstrip() for line in svg.read_text().splitlines()) + '\n')
    plt.close(fig)


if __name__ == '__main__':
    main()
