"""Post-audit complete-matrix tables; no inference, fitting or selection."""
import argparse
import csv
import json
from pathlib import Path
import statistics

BASE=Path(__file__).resolve().parent


def read(path): return json.loads(path.read_text())


def summarize(values):
    good=[x for x in values if x is not None]; complete=len(good)==len(values)
    return dict(values=values,complete=complete,valid_count=len(good),
                mean=statistics.mean(good) if complete else None,
                sample_sd=statistics.stdev(good) if complete and len(good)>1 else None,
                positive_count=sum(v>0 for v in good) if complete else None)


def main():
    parser=argparse.ArgumentParser(); parser.add_argument('stage',choices=['development','evaluation']); args=parser.parse_args()
    stage=args.stage
    result=read(BASE/f'{stage}-results.json'); audit=read(BASE/f'{stage}-independent-audit.json')
    assert result['complete'] and result['integrity_pass'] and audit['passed']
    assert result['calibration_pass']
    rows=[row for data in result['data'] for row in data['rows']]
    with (BASE/f'{stage}-scores.csv').open('w',newline='') as f:
        writer=csv.DictWriter(f,fieldnames=list(rows[0]),lineterminator='\n');writer.writeheader();writer.writerows(rows)
    groups={}
    for data in result['data']:
        for row in data['rows']:
            key=tuple(row[k] for k in ('arm','branch','family','target','mode'))
            groups.setdefault(key,[]).append(row['loss'])
    table=[dict(zip(('arm','branch','family','target','mode'),key),**summarize(values)) for key,values in groups.items()]
    dose=[]
    for arm in ('Plastic','Yoked'):
        for family,target in [('G','lag2'),('C','next')]:
            for mode in ('refitted','transported'):
                for fraction in (1,5,10):
                    for selector in ('top','count','movement'):
                        values=[]
                        for data in result['data']:
                            selected=[r['difference'] for r in data['contrasts'] if r['kind']=='reversion' and
                                      (r['arm'],r['family'],r['target'],r['mode'])==(arm,family,target,mode) and
                                      (r['branch']==f'top-{fraction}' if selector=='top' else r['branch'].startswith(f'{selector}-{fraction}-'))]
                            assert len(selected)==(1 if selector=='top' else 5)
                            values.append(statistics.mean(selected) if all(x is not None for x in selected) else None)
                        dose.append(dict(arm=arm,family=family,target=target,mode=mode,percent=fraction,selector=selector,
                                         **summarize(values)))
    output=dict(stage=stage,seeds=result['seeds'],scope='Exploratory; control replicates averaged within seed before summaries',
                score_groups=table,dose_effects=dose,paired_contrasts=result['paired_contrasts'])
    (BASE/f'{stage}-tables.json').write_text(json.dumps(output,indent=2)+'\n')
    assert len(table)==284 and len(dose)==72
    print(json.dumps(dict(stage=stage,score_rows=len(rows),score_groups=len(table),dose_groups=len(dose))))


if __name__=='__main__':main()
