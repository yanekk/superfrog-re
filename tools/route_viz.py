#!/usr/bin/env python3
"""Route viewer: replay routes on the engine and draw them over their level maps, one web page.

    python3 tools/route_viz.py --planner             # every route the smart planner has made
    python3 tools/route_viz.py path/to/W2-3.txt ...  # any route files (level from the name or
                                                     # the header, or --level W2-3)
    open build/route-viz/index.html

Each route is played on the engine from the app's start (`# timing: app`, its `# seed:`) by
engine/crates/sf-plan/examples/route_viz.rs, which dumps the level's tiles, every object marker,
the frog's track and the events along it; this script writes them as build/route-viz/data/*.js
beside one index.html with a route switcher. Script tags, not fetch, load the data, so the page
also works opened straight from disk.

--planner takes the committed set-C routes (plans/smart-planner/bench/routes/app) and the
planner's latest guided set-C result per level from bench/results.jsonl, and gives each route a
verdict: failed (no route reaches the exit), faults (see FAULTS below) or clean. A level the
planner failed is listed with its map and objects and no route.

The output holds the disks' graphics: build/ is git-ignored, keep it there.
"""
import argparse, json, os, re, subprocess, sys
from concurrent.futures import ThreadPoolExecutor

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ENGINE = os.path.join(ROOT, 'engine')
HERE = os.path.join(ROOT, 'tools', 'route-viz')
BENCH = os.path.join(ROOT, 'plans', 'smart-planner', 'bench')
BIN = os.path.join(ENGINE, 'target', 'release', 'examples', 'route_viz')
LEVELS = [f'W{w}-{l}' for w in range(1, 7) for l in range(1, 5)]
LEVEL_RE = re.compile(r'\b(W[1-6]-[1-4](?:alt)?|WW-1|WB-1)\b', re.I)

# What makes a finished route "struggling", from the plan's own measures
# (plans/smart-planner/DESIGN §4, RESULTS.md): M6 counts routes that lose no heart; C4 wants no
# route over +25% of the committed reference; a plan that used half its 30-minute budget or more
# is flagged as slow.
LONGER_THAN_REF = 0.25
SLOW_PLAN_S = 15 * 60


def level_of(path, given=None):
    if given:
        return given.upper()
    m = LEVEL_RE.search(os.path.basename(path))
    if not m:
        with open(path) as f:
            head = ''.join(l for l in f if l.startswith('#'))
        m = LEVEL_RE.search(head)
    if not m:
        sys.exit(f'{path}: no level id in its name or header; pass --level')
    return m.group(1).upper().replace('ALT', 'alt')


def build_binary():
    r = subprocess.run(['cargo', 'build', '--release', '-q', '-p', 'sf-plan', '--example', 'route_viz'], cwd=ENGINE)
    if r.returncode:
        sys.exit('cargo build failed')


def replay(level, route, out_json):
    r = subprocess.run([BIN, level, route, out_json], capture_output=True, text=True)
    if r.returncode:
        raise RuntimeError(f'{level} {route}: {r.stderr.strip()}')
    return r.stderr.strip()


def bench_results():
    """The planner's latest guided set-C result per level."""
    latest = {}
    path = os.path.join(BENCH, 'results.jsonl')
    if os.path.exists(path):
        for line in open(path):
            d = json.loads(line)
            if d.get('planner') == 'new' and d.get('set') == 'C' and d.get('guided'):
                latest[d['level']] = d
    return latest


def verdict(entry, data, bench):
    """Status, faults and info for a route from its replay and, if it has one, its bench result."""
    faults, info = [], []
    ev = data['events']
    hearts = sum(1 for e in ev if e[3] == 'hurt')
    lives = sum(1 for e in ev if e[3] == 'death')
    status = 'clean'
    if bench and not bench['completed']:
        status = 'failed'
        faults.append('planner failed: ' + bench.get('reason', ''))
    elif entry['route'] and data['end'] is None:
        status = 'failed'
        faults.append('the replay never reaches the exit')
    elif not entry['route']:
        status = 'failed'
        faults.append('no route')
    if lives:
        faults.append(f'lost {lives} {"life" if lives == 1 else "lives"}')
    if hearts:
        faults.append(f'lost {hearts} heart{"s" if hearts > 1 else ""}')
    if bench:
        info.append(f'planned in {bench["wall_s"] / 60:.1f} min')
        if bench['completed'] and bench['wall_s'] >= SLOW_PLAN_S:
            faults.append(f'slow plan: {bench["wall_s"] / 60:.1f} of 30 min')
        rf, ref = bench.get('route_frames'), bench.get('ref_frames')
        if rf and ref:
            d = rf / ref - 1
            info.append(f'{rf} frames vs reference {ref} ({d:+.0%})')
            if d > LONGER_THAN_REF:
                faults.append(f'{d:+.0%} longer than the reference route')
    if data.get('timing') != 'app':
        faults.append('timed for the original: replay may drift')
    if status == 'clean' and faults:
        status = 'faults'
    return status, faults, info


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('routes', nargs='*', help='route files (# timing: app)')
    ap.add_argument('--planner', action='store_true', help="every route the smart planner has made, with verdicts")
    ap.add_argument('--level', help='level id for the route files, when their names do not say')
    ap.add_argument('--out', default=os.path.join(ROOT, 'build', 'route-viz'), help='output folder')
    ap.add_argument('--jobs', type=int, default=4)
    ap.add_argument('--bare', action='store_true', help='index.html without its <!doctype>/<head> skeleton (for hosts that add one)')
    a = ap.parse_args()
    if not a.routes and not a.planner:
        ap.error('give route files or --planner')

    entries = []
    if a.planner:
        bench = bench_results()
        for lvl in LEVELS:
            route = os.path.join(BENCH, 'routes', 'app', f'{lvl}.txt')
            has = os.path.exists(route)
            if not has and lvl not in bench:
                continue
            entries.append({'id': lvl, 'label': lvl, 'level': lvl, 'group': lvl.split('-')[0],
                            'route': route if has else None, 'bench': bench.get(lvl)})
    for p in a.routes:
        lvl = level_of(p, a.level)
        rid = re.sub(r'[^A-Za-z0-9._-]', '_', os.path.splitext(os.path.basename(p))[0])
        while any(e['id'] == rid for e in entries):
            rid += '_'
        entries.append({'id': rid, 'label': rid, 'level': lvl, 'group': 'Routes',
                        'route': os.path.abspath(p), 'bench': None})

    os.makedirs(os.path.join(a.out, 'data'), exist_ok=True)
    build_binary()

    def run(e):
        js = os.path.join(a.out, 'data', e['id'] + '.json')
        msg = replay(e['level'], e['route'] or '-', js)
        data = json.load(open(js))
        os.remove(js)
        status, faults, info = verdict(e, data, e['bench'])
        with open(os.path.join(a.out, 'data', e['id'] + '.js'), 'w') as f:
            f.write('window.ROUTE_DATA=window.ROUTE_DATA||{};ROUTE_DATA[%s]=' % json.dumps(e['id']))
            f.write(json.dumps(data, separators=(',', ':')))
            f.write(';\n')
        print(f'  {e["id"]:10} {status:7} {msg}', file=sys.stderr)
        return {'id': e['id'], 'label': e['label'], 'group': e['group'], 'status': status,
                'faults': faults, 'info': info, 'data': f'data/{e["id"]}.js',
                'route_file': os.path.relpath(e['route'], ROOT) if e['route'] else None,
                'bench': e['bench'] and {k: e['bench'].get(k) for k in ('wall_s', 'ref_frames', 'route_frames')}}

    with ThreadPoolExecutor(max_workers=a.jobs) as ex:
        manifest = list(ex.map(run, entries))
    page = open(os.path.join(HERE, 'template.html')).read()
    page = page.replace('__MANIFEST__', json.dumps(manifest).replace('</', '<\\/'))
    if not a.bare:
        page = ('<!doctype html>\n<html lang="en"><head><meta charset="utf-8">'
                '<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">'
                '<style>body{margin:0}</style></head><body>\n' + page + '\n</body></html>\n')
    out = os.path.join(a.out, 'index.html')
    open(out, 'w').write(page)
    n = lambda s: sum(1 for m in manifest if m['status'] == s)
    print(f'{out}: {len(manifest)} routes, {n("clean")} clean, {n("faults")} with faults, {n("failed")} failed')


if __name__ == '__main__':
    main()
