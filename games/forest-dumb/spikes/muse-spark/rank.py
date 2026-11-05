#!/usr/bin/env python3
"""Ranks the models in a run.py calls.jsonl: runs passed, passed first try, cost, time a call.

    python3 -I rank.py <calls.jsonl> <routine,routine,...>

Per routine, one character a run: the try it passed on, or x.
"""
import collections, json, sys

recs = [json.loads(l) for l in open(sys.argv[1])]
routines = sys.argv[2].split(',')
runs = collections.defaultdict(list)
for c in recs:
    runs[(c['model'], c['routine'], c['run'])].append(c)
nruns = max(c['run'] for c in recs)
rows = []
for m in sorted({c['model'] for c in recs}):
    cells, cost, secs, oc, first, passed = [], 0.0, [], collections.Counter(), 0, 0
    for r in routines:
        cell = ''
        for k in range(1, nruns + 1):
            cs = runs.get((m, r, k), [])
            p = [c['try'] for c in cs if c['outcome'] == 'pass']
            cell += str(p[0]) if p else 'x'
            passed += bool(p)
            first += bool(p and p[0] == 1)
            for c in cs:
                cost += c.get('cost') or 0
                oc[c['outcome']] += 1
                if c.get('secs') and c['outcome'] != 'error':
                    secs.append(c['secs'])
        cells.append(cell)
    rows.append((-passed, -first, cost, m, cells, oc, sum(secs) / max(len(secs), 1)))
print(f"| Model | Passed | First try | {' | '.join(routines)} | Cost $ | Secs/call | Outcomes |")
print('|---|---|---|' + '---|' * len(routines) + '---|---|---|')
for p, f, cost, m, cells, oc, s in sorted(rows):
    print(f"| {m} | {-p}/{len(routines) * nruns} | {-f} | {' | '.join(cells)} | {cost:.4f} | {s:.0f} | "
          f"{', '.join(f'{k} {v}' for k, v in oc.most_common())} |")
