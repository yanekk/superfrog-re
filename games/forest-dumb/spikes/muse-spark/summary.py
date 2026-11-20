#!/usr/bin/env python3
"""Sums run.py's calls.jsonl per model and routine: runs passed, tries, tokens, cost, time.

    python3 -I summary.py [build/spike/muse-spark/calls.jsonl]
"""
import collections, json, os, sys

path = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
    os.path.dirname(os.path.abspath(__file__)), '..', '..', 'build', 'spike', 'muse-spark', 'calls.jsonl')
calls = [json.loads(l) for l in open(path)]
runs = collections.defaultdict(list)
for c in calls:
    runs[(c['model'], c['routine'], c['run'])].append(c)

rows = collections.defaultdict(lambda: collections.Counter())
for (m, r, k), cs in runs.items():
    t = rows[(m, r)]
    t['runs'] += 1
    t['passed'] += any(c['outcome'] == 'pass' for c in cs)
    t['first_try'] += cs[0]['outcome'] == 'pass' if cs else 0
    for c in cs:
        t['calls'] += c['outcome'] not in ('budget',)
        t['in'] += c.get('in') or 0
        t['out'] += c.get('out') or 0
        t['cost'] += c.get('cost') or 0
        t['secs'] += c.get('secs') or 0
        t[c['outcome']] += 1

print('| Model | Routine | Runs passed | First try | Calls | Tokens in | Tokens out | Cost $ | Secs/call | Outcomes |')
print('|---|---|---|---|---|---|---|---|---|---|')
tot = collections.defaultdict(collections.Counter)
for (m, r), t in sorted(rows.items(), key=lambda x: (x[0][1], x[0][0])):
    oc = ', '.join(f'{k} {t[k]}' for k in ('pass', 'fail', 'compile', 'no-code', 'error', 'crash', 'budget') if t[k])
    print(f"| {m} | {r} | {t['passed']}/{t['runs']} | {t['first_try']}/{t['runs']} | {t['calls']} | {t['in']} | {t['out']} "
          f"| {t['cost']:.4f} | {t['secs'] / max(t['calls'], 1):.0f} | {oc} |")
    tot[m].update(t)
print()
print('| Model | Runs passed | Calls | Tokens in | Tokens out | Cost $ | Cost per passed run $ |')
print('|---|---|---|---|---|---|---|')
for m, t in sorted(tot.items()):
    print(f"| {m} | {t['passed']}/{t['runs']} | {t['calls']} | {t['in']} | {t['out']} | {t['cost']:.4f} "
          f"| {t['cost'] / max(t['passed'], 1):.4f} |")
