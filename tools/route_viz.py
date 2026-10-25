#!/usr/bin/env python3
"""Route viewer: replay routes on the engine and draw them over their level maps, one web page.

    python3 tools/route_viz.py --planner             # every route the smart planner has made
    python3 tools/route_viz.py path/to/W2-3.txt ...  # any route files (level from the name or
                                                     # the header, or --level W2-3)
        [--guide waypoints.txt]                      # a waypoint path drawn over them
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


def read_guide(path):
    """An sf-plan waypoint file: `col,row` per line, `# t <secs>[ est]` after it (est: the frog was
    not visible, its place estimated from the camera). Returns [col, row, secs or None, est]."""
    pts = []
    for line in open(path):
        body, _, note = line.partition('#')
        m = re.match(r'\s*(\d+)\s*,\s*(\d+)\s*$', body)
        if not m:
            continue
        t = re.search(r't\s+([\d.]+)', note)
        pts.append([int(m.group(1)), int(m.group(2)), float(t.group(1)) if t else None, 'est' in note.split()])
    return pts


def progress(data):
    """Where the frog stops making progress: the frame it last stood on a cell it had not been on,
    and the frame it came closest to an exit cell (cells, Chebyshev). Both are appended to the
    events as `stall` marks."""
    end = data['end']
    # Past the route's last line the replay only idles: judge the frames the route drives.
    track = data['track'][:data.get('inputs_end', len(data['track'])) + 1]
    if len(track) < 2:
        return []
    cell = lambda p: ((p[0] + 0x17) // 16, (p[1] - 0x10) // 16)
    exits = [(o[0], o[1]) for o in data['objects'] if o[4] == 'level exit']
    seen, last_new = set(), 0
    best, best_k = None, 0
    for k, p in enumerate(track):
        c = cell(p)
        if c not in seen:
            seen.add(c)
            last_new = k
        if exits:
            d = min(max(abs(c[0] - x), abs(c[1] - y)) for x, y in exits)
            if best is None or d < best:
                best, best_k = d, k
    n = len(track) - 1
    out = []
    if end is None:
        p = track[last_new]
        data['events'].append([last_new, p[0], p[1], 'stall', f'last new ground: no cell after this one is new (inputs end {n - last_new} frames later)'])
        out.append(f'last new ground at frame {last_new}; the route\'s inputs end at frame {n}')
        if best is not None:
            p = track[best_k]
            data['events'].append([best_k, p[0], p[1], 'stall', f'closest to the exit: {best} cells; never closer after this'])
            out.append(f'closest to the exit {best} cells, at frame {best_k}')
        data['events'].sort(key=lambda e: e[0])
    return out


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
    if entry.get('partial'):
        info.append('the line is a partial route from a re-run, not the scored run')
    if status == 'failed':
        pass
    elif entry['route'] and data['end'] is None:
        status = 'failed'
        faults.append('the replay never reaches the exit' + (' (a partial route)' if 'partial route' in ' '.join(data['notes']) else ''))
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
    info.extend(progress(data))
    if status == 'clean' and faults:
        status = 'faults'
    return status, faults, info


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('routes', nargs='*', help='route files (# timing: app)')
    ap.add_argument('--planner', action='store_true', help="every route the smart planner has made, with verdicts")
    ap.add_argument('--level', help='level id for the route files, when their names do not say')
    ap.add_argument('--out', default=os.path.join(ROOT, 'build', 'route-viz'), help='output folder')
    ap.add_argument('--guide', help='an sf-plan waypoint file (col,row per line) drawn over every route given')
    ap.add_argument('--partials', default=os.path.join(ROOT, 'build', 'route-viz-runs'),
                    help='folder of re-run partial routes, <level>.txt, for levels the planner failed')
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
            partial = False
            if not os.path.exists(route):
                # sf-bench keeps no route for a failed level; a re-run's best partial route,
                # `sf-plan <lvl> --app ... --out build/route-viz-runs/<lvl>.txt`, stands in.
                route = os.path.join(a.partials, f'{lvl}.txt')
                partial = os.path.exists(route)
                if not partial:
                    route = None
            if not route and lvl not in bench:
                continue
            entries.append({'id': lvl, 'label': lvl, 'level': lvl, 'group': lvl.split('-')[0],
                            'route': route, 'partial': partial, 'bench': bench.get(lvl)})
    for p in a.routes:
        lvl = level_of(p, a.level)
        rid = re.sub(r'[^A-Za-z0-9._-]', '_', os.path.splitext(os.path.basename(p))[0])
        while any(e['id'] == rid for e in entries):
            rid += '_'
        entries.append({'id': rid, 'label': rid, 'level': lvl, 'group': lvl,
                        'route': os.path.abspath(p), 'bench': None})
    guide = read_guide(a.guide) if a.guide else None
    if guide:
        print(f'guide: {len(guide)} waypoints from {a.guide}', file=sys.stderr)

    os.makedirs(os.path.join(a.out, 'data'), exist_ok=True)
    build_binary()

    def run(e):
        js = os.path.join(a.out, 'data', e['id'] + '.json')
        msg = replay(e['level'], e['route'] or '-', js)
        data = json.load(open(js))
        os.remove(js)
        status, faults, info = verdict(e, data, e['bench'])
        if guide and e in entries[len(entries) - len(a.routes):]:
            data['guide'] = guide
            data['guide_file'] = os.path.basename(a.guide)
        with open(os.path.join(a.out, 'data', e['id'] + '.js'), 'w') as f:
            f.write('window.ROUTE_DATA=window.ROUTE_DATA||{};ROUTE_DATA[%s]=' % json.dumps(e['id']))
            f.write(json.dumps(data, separators=(',', ':')))
            f.write(';\n')
        print(f'  {e["id"]:10} {status:7} {msg}', file=sys.stderr)
        return {'id': e['id'], 'label': e['label'], 'group': e['group'], 'status': status,
                'faults': faults, 'info': info, 'data': f'data/{e["id"]}.js',
                'route_file': e['route'] and (os.path.relpath(e['route'], ROOT) if e['route'].startswith(ROOT) else e['route']),
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
