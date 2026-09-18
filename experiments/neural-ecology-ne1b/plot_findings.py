"""Descriptive post-audit figure. Does not fit, select, or test hypotheses."""
import json
import os
from pathlib import Path

os.environ.setdefault('MPLCONFIGDIR', '/private/tmp/ne1b-matplotlib')
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.lines import Line2D

BASE = Path(__file__).resolve().parent


def main():
    audit = json.loads((BASE / 'evaluation-independent-audit.json').read_text())
    table = json.loads((BASE / 'evaluation-tables.json').read_text())
    assert audit['passed'] and len(table['seeds']) == 10
    rows = table['dose_effects']
    assert len(rows) == 72 and all(r['complete'] for r in rows)
    colors = {'Plastic': '#1769aa', 'Yoked': '#bd5900'}
    styles = {'top': ('-', 'o'), 'count': (':', 's'), 'movement': ('--', '^')}
    fig, axes = plt.subplots(2, 2, figsize=(11, 8.5))
    for i, mode in enumerate(('refitted', 'transported')):
        for j, (family, target, title) in enumerate((
            ('G', 'lag2', 'Two-step history · q / global'),
            ('C', 'next', 'Next symbol · q / conditioned'),
        )):
            ax = axes[i, j]
            for arm, color in colors.items():
                for selector, (line, marker) in styles.items():
                    selected = sorted((r for r in rows if
                        (r['arm'], r['family'], r['target'], r['mode'], r['selector']) ==
                        (arm, family, target, mode, selector)), key=lambda r: r['percent'])
                    assert len(selected) == 3
                    ax.plot([r['percent'] for r in selected], [r['mean'] for r in selected],
                            color=color, linestyle=line, marker=marker, markersize=5,
                            linewidth=1.8)
            ax.axhline(0, color='#444444', linewidth=.8)
            ax.set_title(f'{title}\n{mode.capitalize()} reader', fontsize=11)
            ax.set_xticks([1, 5, 10])
            ax.set_xlabel('Selected edges (% of all 16,384 edges)')
            ax.set_ylabel('Reverted − unchanged log loss (nats)')
            ax.grid(alpha=.15)
            ax.spines[['top', 'right']].set_visible(False)
    handles = [Line2D([0], [0], color=c, lw=2, label=a) for a, c in colors.items()]
    handles += [Line2D([0], [0], color='#444444', linestyle=s, marker=m, label=l)
                for l, (s, m) in zip(('Top movement', 'Count matched', 'Movement matched'), styles.values())]
    fig.suptitle('NE-1b: reversion helps the old reader but hurts refitted accessibility', fontsize=14)
    fig.legend(handles=handles, loc='upper center', bbox_to_anchor=(.5, .925), ncol=5, frameon=False)
    fig.text(.5, .027,
             'Means over 10 reused seeds; five random controls averaged within each seed. Descriptive, exploratory.\n'
             'Positive = reversion worsens the reader. Negative = reversion improves it. Panels use different y scales.\n'
             'Mean curves only; seed values and sample SDs are in evaluation-tables.json. High control overlap limits edge-identity claims.',
             ha='center', va='bottom', fontsize=9)
    fig.tight_layout(rect=(0, .10, 1, .885))
    fig.savefig(BASE / 'dose-effects.png', dpi=170)
    fig.savefig(BASE / 'dose-effects.svg')
    svg = BASE / 'dose-effects.svg'
    svg.write_text('\n'.join(line.rstrip() for line in svg.read_text().splitlines()) + '\n')
    plt.close(fig)


if __name__ == '__main__':
    main()
