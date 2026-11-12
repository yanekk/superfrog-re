#!/usr/bin/env python3
"""Haiku's trial: whether it keeps the generator and rewrite jobs (translator DESIGN §2.13, T19).

    python3 -I games/forest-dumb/tools/haiku_trial.py [--only gen|reshape] [--tries 3] [--max-calls 100] [--out DIR]

Reruns level 1's recorded sets on Haiku, through the tools sessions use: T61's 5 generators
(`generate.py`, judged by `fd-judge --cover`) and T62's 21 rewrites, 15 engine functions of T08 and T09
(`reshape.py`, judged by the task's rig test), the rewrites starting from the translated routine
`translate.py --task T08|T09` writes first, the input a session really gives it. Muse's numbers are
level 1's records in docs/11-translation.md, cited per item in MUSE_GEN and MUSE_RESHAPE.

Pass mark (owner, 2026-11-11): every generator reaches at least as many of its routine's branch ways as
Muse's did, and at least 20 of the 21 rewritten routines pass their rig with no hand fix. A rewrite is
judged as T62's were, with no tidy round (`--tidy 0`): the mark is the rig's, and a tidy round is a
call the 100-call bound (DESIGN §5.3) has no room for. A unit serving four routines passes or fails
all four.

Writes <out>/table.md (per item: Haiku's result, branch ways covered or rig result, Muse's recorded
result) and <out>/summary.json, and prints the summary as one line:

    {"gen_at_least_muse": n, "gen": 5, "reshape_pass": n, "reshape": 21, "mark_met": bool|null,
     "no_model": n, "calls": n, "usd": x}

`mark_met` is null when only one half ran (`--only`) or any item got no answer from `claude`
(`no_model` > 0: not logged in, say), since neither is a verdict on Haiku. An item whose every call ran
past model.py's 300 s bound is `timeout`, a failure: Haiku answered nothing a session could use. Exit 0 when the mark is met,
1 otherwise, 2 on a usage error. All model calls go through model.py in this one process, so
`--max-calls` (100, the §5.3 row's bound) holds over the whole trial. Standard library only, for
`python3 -I`.
"""
import argparse, json, os, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import model  # noqa: E402

GAME = os.path.dirname(HERE)
OUT = os.path.join(GAME, 'build', 'haiku-trial')
MAX_CALLS = 100

T61 = 'docs/11-translation.md, "The generator service", Record (level 1 T61, 2026-11-05)'
T62 = 'docs/11-translation.md, "The engine-shape trial", The run (level 1 T62, 2026-11-05)'

# (routine as generate.py takes it, generator name, subsystem, Muse's branches, Muse's result and tries).
# Every Muse generator was `covered`: each branch taken both ways.
MUSE_GEN = [
    ('h0+$4e9a', 'h0_4e9a', 'camera', 5, 'covered, 2 tries'),
    ('play_keys', 'play_keys', 'rest', 8, 'covered, 1 try'),
    ('char_sprite_position', 'char_sprite_position', 'drawing', 3, 'covered, 1 try'),
    ('h0+$3566', 'h0_3566', 'objects', 1, 'covered, 1 try'),
    ('h0+$4b0e', 'h0_4b0e', 'character', 14, 'covered, 1 try'),
]

# (task, engine fn, its routines' rig names, Muse's result, tries, paths). Every unit passed.
MUSE_RESHAPE = [
    ('T08', 'step', ['char_step'], 'pass', 1, 91),
    ('T08', 'idle', ['h0_3ea8'], 'pass', 2, 29),
    ('T08', 'land_or_fall', ['h0_410c'], 'pass', 1, 9),
    ('T08', 'fall_step', ['h0_416c'], 'pass', 1, 2),
    ('T08', 'rise_step', ['h0_4274'], 'pass', 1, 6),
    ('T08', 'move_x', ['h0_4696'], 'pass', 1, 4),
    ('T08', 'left_pressed', ['h0_4718'], 'pass', 1, 24),
    ('T08', 'run_left_step', ['h0_47e2'], 'pass', 1, 3),
    ('T08', 'right_pressed', ['h0_48be'], 'pass', 1, 24),
    ('T08', 'run_right_step', ['h0_497c'], 'pass', 1, 3),
    ('T08', 'up_pressed', ['h0_4b0e'], 'pass', 1, 8),
    ('T08', 'probe_flags', ['char_probe_flags'], 'pass', 1, 15),
    ('T09', 'follow', ['h0_4e9a', 'h0_4ea6', 'h0_4eb4', 'h0_4ec2'], 'pass', 1, 260),
    ('T09', 'clamp', ['h0_513c'], 'pass', 1, 3),
    ('T09', 'scroll', ['h0_51fc', 'h0_526a', 'h0_52d8', 'h0_5346'], 'pass', 1, 16),
]
GEN_MARK = len(MUSE_GEN)  # every generator at least Muse's
RESHAPE_MARK = 20  # of 21


class Usage(Exception):
    """exit 2"""


def ways(branches, uncovered):
    """Branch ways a generator's states took: two a branch, less each way none took (`either` is both)."""
    return 2 * branches - sum(2 if u.get('never') == 'either' else 1 for u in uncovered)


def gen_row(muse, res):
    """One generator's row: Haiku's result line `res` against Muse's record."""
    routine, name, sub, branches, muse_result = muse
    got = ways(res.get('branches', 0), res.get('uncovered', [])) if res['result'] in ('covered', 'partial') else 0
    return {'routine': routine, 'name': name, 'subsystem': sub, 'result': res['result'],
            'tries': res.get('tries', 0), 'branches': res.get('branches', 0), 'ways': got,
            'uncovered': res.get('uncovered', []), 'muse_ways': 2 * branches, 'muse': muse_result,
            'at_least_muse': res['result'] in ('covered', 'partial') and got >= 2 * branches,
            'stopped': res.get('stopped'), 'error': res.get('error')}


def reshape_row(muse, res, translated):
    """One rewrite unit's row; `translated` maps each rig name to translate.py's result for it."""
    task, fn, routines, muse_result, muse_tries, muse_paths = muse
    return {'task': task, 'fn': fn, 'routines': routines, 'result': res['result'], 'tries': res.get('tries', 0),
            'paths': res.get('paths', 0), 'translated': [translated.get(n, '?') for n in routines],
            'muse': f'{muse_result}, {muse_tries} {"try" if muse_tries == 1 else "tries"}, {muse_paths} paths',
            'stopped': res.get('stopped'), 'error': res.get('error')}


def summarise(gen_rows, reshape_rows, calls=0, usd=0.0):
    """The mark (DESIGN §2.13) over whichever halves ran; None for a half that did not."""
    s = {'gen_at_least_muse': None, 'gen': len(MUSE_GEN), 'reshape_pass': None, 'reshape': sum(len(m[2]) for m in MUSE_RESHAPE),
         'mark_met': None, 'no_model': 0, 'calls': calls, 'usd': round(usd, 6)}
    rows = (gen_rows or []) + (reshape_rows or [])
    s['no_model'] = sum(r['result'] == 'no-model' for r in rows)
    if gen_rows is not None:
        s['gen_at_least_muse'] = sum(r['at_least_muse'] for r in gen_rows)
    if reshape_rows is not None:
        s['reshape_pass'] = sum(len(r['routines']) for r in reshape_rows if r['result'] == 'pass')
    if gen_rows is not None and reshape_rows is not None and not s['no_model']:
        s['mark_met'] = s['gen_at_least_muse'] >= GEN_MARK and s['reshape_pass'] >= RESHAPE_MARK
    return s


def table(gen_rows, reshape_rows, s):
    """table.md: what the docs and FINDINGS quote."""
    out = ['# Haiku trial (translator T19)', '',
           f"Mark (DESIGN §2.13): all {GEN_MARK} generators at least Muse's branch ways, and at least "
           f"{RESHAPE_MARK} of {s['reshape']} rewritten routines passing their rig. "
           f"Met: {s['mark_met']}. Calls {s['calls']}, ${s['usd']} list.", '']
    if gen_rows is not None:
        out += [f'## Generators ({s["gen_at_least_muse"]} of {s["gen"]} at least Muse)', '', f'Muse: {T61}.', '',
                '| Routine | Subsystem | Haiku: result, tries | Branch ways covered | Untaken | Muse (record) | At least Muse |',
                '|---|---|---|---|---|---|---|']
        for r in gen_rows:
            un = ', '.join(f"{u['at']} {u['never']}" for u in r['uncovered']) or '—'
            note = f" ({r['error']})" if r.get('error') else ''
            out.append(f"| `{r['name']}` | {r['subsystem']} | {r['result']}{note}, {r['tries']} | {r['ways']} of "
                       f"{2 * r['branches']} | {un} | {r['muse']}, {r['muse_ways']} ways | "
                       f"{'yes' if r['at_least_muse'] else 'no'} |")
        out.append('')
    if reshape_rows is not None:
        out += [f'## Rewrites ({s["reshape_pass"]} of {s["reshape"]} routines pass)', '', f'Muse: {T62}.', '',
                '| Task | Function | Routines | Translated | Haiku: rig result, tries | Paths | Muse (record) |',
                '|---|---|---|---|---|---|---|']
        for r in reshape_rows:
            note = f" ({r['error']})" if r.get('error') else ''
            out.append(f"| {r['task']} | `{r['fn']}` | {', '.join(f'`{n}`' for n in r['routines'])} | "
                       f"{', '.join(r['translated'])} | {r['result']}{note}, {r['tries']} | {r['paths']} | {r['muse']} |")
        out.append('')
    return '\n'.join(out)


def write(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path + '.tmp', 'w', encoding='utf-8') as f:
        f.write(text)
    os.replace(path + '.tmp', path)


def translate_task(task):
    """translate.py --task: writes build/translate/h0-<off>/translated.rs for the task's routines.
    -> {rig name: result}. A step that failed leaves the routines it did not reach out."""
    p = subprocess.run([sys.executable, '-I', os.path.join(HERE, 'translate.py'), '--task', task],
                       capture_output=True, text=True)
    sys.stderr.write(p.stderr[-2000:])
    got = {}
    for line in p.stdout.splitlines():
        try:
            d = json.loads(line)
        except ValueError:
            continue
        if isinstance(d, dict) and 'routine' in d:
            off = d['routine'].split('$')[-1]
            got[f'h0_{off}'] = d['result']
            if d.get('name'):
                got[d['name']] = d['result']
    return got


def timed_out(res, tries_log, key, value):
    """`res` with result `timeout` when the tool said `no-model` only because every call of this item ran past
    model.py's bound: Haiku was reached and gave nothing in time, which a session meets too, so it is a
    failed item, not a missing verdict as a logged-out `claude` is."""
    if res.get('result') != 'no-model' or not res.get('tries'):
        return res
    try:
        with open(tries_log, encoding='utf-8') as f:
            mine = [d for d in map(json.loads, f) if d.get(key) == value]
    except (OSError, ValueError):
        return res
    last = mine[-res['tries']:]
    if last and all(d.get('outcome') == 'timeout' for d in last):
        return dict(res, result='timeout')
    return res


def run_gen(tries, out):
    import generate
    rows = []
    for m in MUSE_GEN:
        try:
            res = generate.generate(m[0], m[1], tries, os.path.join(out, 'generate'))
        except generate.jr.Usage as e:
            res = {'result': 'usage', 'error': str(e)}
        res = timed_out(res, os.path.join(out, 'generate', 'tries.jsonl'), 'gen', m[1])
        rows.append(gen_row(m, res))
        print(json.dumps({'gen': m[1], 'result': res['result'], 'ways': rows[-1]['ways'], 'error': res.get('error')}),
              file=sys.stderr)
    return rows


def run_reshape(tries, out):
    import reshape
    translated = {}
    for task in sorted({m[0] for m in MUSE_RESHAPE}):
        translated.update(translate_task(task))
    rows = []
    for m in MUSE_RESHAPE:
        task, fn, routines = m[:3]
        try:
            res = reshape.reshape(routines[0], task, tries, os.path.join(out, 'reshape'), tidy=0)
        except reshape.jr.Usage as e:
            res = {'result': 'usage', 'error': str(e)}
        res = timed_out(res, os.path.join(out, 'reshape', 'tries.jsonl'), 'fn', fn)
        rows.append(reshape_row(m, res, translated))
        print(json.dumps({'reshape': fn, 'result': res['result'], 'tries': res.get('tries', 0), 'error': res.get('error')}),
              file=sys.stderr)
    return rows


def spent(out):
    """Calls made and their list price, from the tools' calls.jsonl under `out`."""
    n, usd = 0, 0.0
    for sub in ('generate', 'reshape'):
        try:
            with open(os.path.join(out, sub, 'calls.jsonl'), encoding='utf-8') as f:
                for line in f:
                    d = json.loads(line)
                    if d.get('result') != 'max-calls':
                        n += 1
                        usd += d.get('usd', 0.0) or 0.0
        except OSError:
            pass
    return n, usd


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--only', choices=('gen', 'reshape'))
    ap.add_argument('--tries', type=int, default=3)
    ap.add_argument('--max-calls', type=int, default=MAX_CALLS)
    ap.add_argument('--out', default=OUT)
    a = ap.parse_args(argv)
    if a.tries < 1 or a.max_calls < 0:
        ap.error('--tries is at least 1, --max-calls at least 0')
    out = os.path.abspath(a.out)
    model.set_max_calls(a.max_calls)
    for sub in ('generate', 'reshape'):
        # A rerun's logs would otherwise count the last run's calls too.
        for log in ('calls.jsonl', 'tries.jsonl'):
            try:
                os.remove(os.path.join(out, sub, log))
            except OSError:
                pass
    gen_rows = run_gen(a.tries, out) if a.only in (None, 'gen') else None
    reshape_rows = run_reshape(a.tries, out) if a.only in (None, 'reshape') else None
    s = summarise(gen_rows, reshape_rows, *spent(out))
    write(os.path.join(out, 'table.md'), table(gen_rows, reshape_rows, s))
    write(os.path.join(out, 'summary.json'), json.dumps(dict(s, gen_rows=gen_rows, reshape_rows=reshape_rows), indent=1))
    print(json.dumps(s, separators=(',', ':')))
    return 0 if s['mark_met'] else 1


if __name__ == '__main__':
    sys.exit(main())
