#!/usr/bin/env python3
"""Run the translation service over every routine of re/translate.toml in one unattended run, and
write the table that says, per routine, whether a passing port waits for its porting task
(plans/forest-dumb-level-1 T40, DESIGN §4.3, §5.2, §5.3; docs/11-translation.md).

    python3 -I games/forest-dumb/tools/translate_batch.py --max-usd 5 [--parallel 8] [--resume] [--out build/translate]

Each routine is one `translate.py '<routine>' --gen <its name>` process; up to --parallel run at once.
Their model calls overlap; their judge builds and runs take turns, since translate.py holds
`<out>/judge.lock` around each (the spike's `judge_lock`). A routine with `split_at` starts once each
callee has its result: all passed, or it is `blocked`.

The cap is this run's spend. Each routine gets min($0.10, what is left less what running routines may
still spend); when that is less than one call's worst case, nothing more starts, and every routine not
started is `not-attempted`. translate.py itself starts no call past the cap it is given.

`--resume` keeps every row of <out>/batch.json that finished (pass with its port on disk, fail,
too-big, no-oracle, no-gen) and runs only the rest. Without it the batch starts afresh.

Writes <out>/batch.json (the rows, after each routine) and <out>/table.md (the table for
docs/11-translation.md). Both hold addresses, results and counts only; the ports stay in build/.
Prints a one-line summary on stdout. Exit 0 when every routine finished, 1 when any is
`not-attempted` or `error`, 2 on a usage error (no key, no inventory).

Results: pass, fail, too-big, no-oracle (as translate.py), no-gen (no generator registered, or none
planned: sound), blocked (a split callee did not pass), not-attempted (the cap), error (translate.py
stopped on a usage error: the reason is in the row's note). Standard library only, for `python3 -I`.
"""
import argparse, concurrent.futures as cf, json, os, subprocess, sys, threading, time, tomllib

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import translate  # noqa: E402

GAME = translate.GAME
REPO = os.path.dirname(os.path.dirname(GAME))
TOOL = os.path.join(HERE, 'translate.py')
PER_ROUTINE_USD = 0.10  # DESIGN §5.2
FINISHED = ('pass', 'fail', 'too-big', 'no-oracle', 'no-gen')
# Every value a row's result may hold, in the order the table's summary counts them.
RESULTS = FINISHED + ('blocked', 'not-attempted', 'error')


def entries(path=translate.INVENTORY):
    """The inventory's routines in address order: dicts with label, name, gen (None for sound),
    port_task, split_at (labels)."""
    with open(path, 'rb') as f:
        inv = tomllib.load(f)
    out = []
    for r in inv.get('routine', []):
        out.append({'label': translate.label(r['hunk'], r['offset']), 'name': r['name'],
                    'hunk': r['hunk'], 'offset': r['offset'],
                    # The generator is named as the routine (T35 to T39, T53, T54, T56); sound has none.
                    'gen': r['name'] if r.get('gen_task') else None,
                    'port_task': r.get('port_task'), 'subsystem': r.get('subsystem'),
                    'split_at': [translate.label(*translate.resolve(c)) for c in r.get('split_at', [])]})
    return sorted(out, key=lambda e: (e['hunk'], e['offset']))


def run_one(entry, max_usd, out):
    """translate.py on one routine as its own process -> its result line, plus `note` on stderr's
    last line for anything but a pass."""
    argv = [sys.executable, '-I', TOOL, entry['label'], '--gen', entry['gen'], '--max-usd', f'{max_usd:.6f}',
            '--out', out]
    p = subprocess.run(argv, cwd=REPO, capture_output=True, text=True)
    last = (p.stderr.strip().splitlines() or [''])[-1]
    if p.returncode == 2:
        if 'no generator' in p.stderr:
            return {'result': 'no-gen', 'note': last}
        return {'result': 'error', 'note': last[:300]}
    try:
        res = json.loads(p.stdout.strip().splitlines()[-1])
    except (IndexError, ValueError):
        return {'result': 'error', 'note': f'exit {p.returncode}, no result line: {last[:300]}'}
    if res['result'] != 'pass' and last:
        res['note'] = last[:300]
    return res


def row_of(entry, res, secs):
    port = res.get('port')
    if port:
        port = os.path.relpath(os.path.join(REPO, port), GAME)
    return {'routine': entry['label'], 'name': entry['name'], 'result': res['result'], 'port': port,
            'gen': entry['gen'], 'port_task': entry['port_task'], 'paths': res.get('paths', 0),
            'tries': res.get('tries', 0), 'tokens_in': res.get('tokens_in', 0),
            'tokens_out': res.get('tokens_out', 0), 'usd': round(res.get('usd', 0.0), 6),
            'secs': round(secs), 'note': res.get('note', '')}


def finished(row):
    if row.get('result') not in FINISHED:
        return False
    return row['result'] != 'pass' or bool(row.get('port')) and os.path.exists(os.path.join(GAME, row['port']))


def run_batch(todo, out, max_usd, parallel=8, resume=False, runner=run_one, min_call=0.0, clock=time.time):
    """Run every entry of `todo` -> the batch dict ({rows: {label: row}, spent, ...}), written to
    <out>/batch.json after each routine. `runner(entry, max_usd, out)` translates one routine;
    `min_call` is the least budget worth starting a routine with (one call's worst case)."""
    os.makedirs(out, exist_ok=True)
    state_path = os.path.join(out, 'batch.json')
    rows = {}
    if resume:
        try:
            with open(state_path, encoding='utf-8') as f:
                rows = {k: r for k, r in json.load(f).get('rows', {}).items() if finished(r)}
        except (OSError, ValueError):
            rows = {}
    started = clock()
    batch = {'started': time.strftime('%Y-%m-%dT%H:%M:%S'), 'max_usd': max_usd, 'parallel': parallel,
             'resumed': resume, 'kept': len(rows), 'spent': 0.0, 'secs': 0, 'rows': rows}
    lock = threading.Lock()

    def save():
        batch['secs'] = round(clock() - started)
        tmp = state_path + '.tmp'
        with open(tmp, 'w', encoding='utf-8') as f:
            json.dump(batch, f, indent=1)
        os.replace(tmp, state_path)

    by_label = {e['label']: e for e in todo}
    pending = [e for e in todo if e['label'] not in rows]
    for e in list(pending):
        if e['gen'] is None:
            rows[e['label']] = row_of(e, {'result': 'no-gen', 'note': f"{e['subsystem']}: no generator planned"}, 0)
            pending.remove(e)
    reserved, capped, running = {}, False, {}

    def blocker(e):
        """None when every split callee passed; the first that cannot pass; or '' while one is still to come."""
        for c in e['split_at']:
            r = rows.get(c)
            if r is None:
                if c in by_label:
                    return ''
                return f'{c}: not in the inventory'
            if r['result'] != 'pass':
                return c
        return None

    with cf.ThreadPoolExecutor(max_workers=max(1, parallel)) as pool:
        while pending or running:
            progressed = True
            while progressed:
                progressed = False
                for e in list(pending):
                    b = blocker(e)
                    if b == '':
                        continue
                    if b is not None:
                        callee = rows.get(b)
                        result = 'not-attempted' if callee and callee['result'] == 'not-attempted' else 'blocked'
                        note = f"split callee {b} is {callee['result'] if callee else 'missing'}"
                        rows[e['label']] = row_of(e, {'result': result, 'note': note}, 0)
                        pending.remove(e)
                        progressed = True
                        continue
                    if capped or len(running) >= parallel:
                        continue
                    with lock:
                        budget = min(PER_ROUTINE_USD, max_usd - batch['spent'] - sum(reserved.values()))
                    if budget <= 0 or budget < min_call:
                        # Nothing starts that the cap could not pay for; what runs finishes.
                        capped = True
                        continue
                    reserved[e['label']] = budget
                    running[pool.submit(runner, e, budget, out)] = (e, clock())
                    pending.remove(e)
                    progressed = True
            if capped and not running:
                for e in pending:
                    rows[e['label']] = row_of(e, {'result': 'not-attempted', 'note': f'the cap, ${max_usd}'}, 0)
                pending = []
                break
            if not running:
                # Only a split callee that can never finish (a cycle) leaves anything here.
                for e in pending:
                    rows[e['label']] = row_of(e, {'result': 'error', 'note': 'its split callees never finished'}, 0)
                pending = []
                break
            done, _ = cf.wait(list(running), return_when=cf.FIRST_COMPLETED)
            for fut in done:
                e, t0 = running.pop(fut)
                try:
                    res = fut.result()
                except Exception as ex:  # a broken runner must not end the unattended run
                    res = {'result': 'error', 'note': str(ex)[:300]}
                if res.get('stopped') == 'max-usd' and res.get('tries', 0) == 0:
                    res = dict(res, result='not-attempted', note=f'the cap, ${max_usd}')
                    capped = True
                with lock:
                    reserved.pop(e['label'], None)
                    batch['spent'] = round(batch['spent'] + res.get('usd', 0.0), 6)
                rows[e['label']] = row_of(e, res, clock() - t0)
                save()
    batch['finished'] = time.strftime('%Y-%m-%dT%H:%M:%S')
    batch['rows'] = {k: rows[k] for k in sorted(rows, key=lambda k: translate.resolve(k))}
    save()
    return batch


def money(x):
    return f'${x:.4f}'


def table(batch):
    """The markdown for docs/11-translation.md: the summary, every routine, then what goes by hand."""
    rows = list(batch['rows'].values())
    count = {r: sum(1 for x in rows if x['result'] == r) for r in RESULTS}
    total = sum(x['usd'] for x in rows)
    lines = [f"{len(rows)} routines: " + ', '.join(f'{n} {r}' for r, n in count.items() if n) +
             f". This run spent {money(batch['spent'])} of its ${batch['max_usd']:g} cap in {batch['secs']} s"
             + (f" (resumed, {batch['kept']} rows kept)" if batch.get('resumed') else '') +
             f"; the table's routines cost {money(total)} in all.", '',
             '| Routine | Result | Port task | Port | Generator | Paths | Tries | Cost |',
             '|---|---|---|---|---|---|---|---|']
    for x in rows:
        name = f"`{x['name']}`" if x['name'] != x['routine'] else ''
        lines.append(f"| `{x['routine']}` {name} | {x['result']} | {x['port_task'] or ''} | "
                     f"{'`' + x['port'] + '`' if x['port'] else ''} | {'`' + x['gen'] + '`' if x['gen'] else ''} | "
                     f"{x['paths'] or ''} | {x['tries'] or ''} | {money(x['usd']) if x['usd'] else ''} |")
    by_task = {}
    for x in rows:
        if x['result'] in ('fail', 'too-big', 'no-oracle', 'blocked', 'error') and x['port_task']:
            by_task.setdefault(x['port_task'], []).append(x)
    if by_task:
        lines += ['', 'Translated by hand, under the porting task that owns each:', '']
        for task in sorted(by_task, key=lambda t: int(t[1:])):
            lines.append(f"- {task}: " + '; '.join(f"`{x['name']}` ({x['result']}"
                                                    + (f": {x['note']}" if x['note'] and x['result'] != 'too-big' else '')
                                                    + ')' for x in by_task[task]))
    skipped = [x for x in rows if x['result'] == 'not-attempted']
    if skipped:
        lines += ['', 'Not attempted (the cap stopped the run): ' + ', '.join(f"`{x['name']}`" for x in skipped) + '.']
    return '\n'.join(lines) + '\n'


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--max-usd', type=float, required=True, help="this run's spend cap (DESIGN §5.2: 5)")
    ap.add_argument('--parallel', type=int, default=8)
    ap.add_argument('--resume', action='store_true', help='run only routines not finished in <out>/batch.json')
    ap.add_argument('--out', default=translate.OUT)
    a = ap.parse_args(argv)
    key, looked = translate.find_key()
    if not key:
        print(f'translate_batch: no OpenRouter key: looked in {looked}', file=sys.stderr)
        return 2
    if not os.path.exists(translate.INVENTORY):
        print(f'translate_batch: no inventory at {translate.INVENTORY}', file=sys.stderr)
        return 2
    out = os.path.abspath(a.out)
    _, per_out = translate.price(key)
    # One call's worst case: a full answer (translate.py adds the prompt, a few thousand tokens at most).
    min_call = translate.MAX_TOKENS * per_out * 1.2
    batch = run_batch(entries(), out, a.max_usd, a.parallel, a.resume, min_call=min_call)
    with open(os.path.join(out, 'table.md'), 'w', encoding='utf-8') as f:
        f.write(table(batch))
    rows = batch['rows'].values()
    print(json.dumps({'routines': len(rows), **{r: sum(1 for x in rows if x['result'] == r) for r in RESULTS},
                      'spent': batch['spent'], 'secs': batch['secs'],
                      'table': os.path.relpath(os.path.join(out, 'table.md'))}, separators=(',', ':')))
    return 0 if all(x['result'] in FINISHED + ('blocked',) for x in rows) else 1


if __name__ == '__main__':
    sys.exit(main())
